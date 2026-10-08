//! Provider-independent operation composition and generation ownership.
//!
//! Native callbacks receive copied bytes. Mutations and recursive dispatch from
//! a callback fail promptly. Removal/revocation waits for all admitted dispatches;
//! callbacks must not wait for another thread to mutate this registry.
use rekenshi_plugin_api::{ERROR, HOOK_BUSY, platform::*};
use std::{
    cell::Cell,
    collections::{BTreeMap, HashSet, VecDeque},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

type Transform = dyn Fn(&[u8], &[u8]) -> Result<Option<Vec<u8>>, i32> + Send + Sync;

/// Shared broker state; clones refer to the same registration and generation set.
#[derive(Clone, Default)]
pub struct Registry(Arc<Shared>);
#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    quiescence: RwLock<()>,
    diagnostics: AtomicBool,
}
#[derive(Default)]
struct State {
    contracts: BTreeMap<String, Contract>,
    subscriptions: BTreeMap<u64, Subscription>,
    faulted: HashSet<u64>,
    serial: u64,
    watches: BTreeMap<u64, Pending>,
    removed: BTreeMap<u64, u64>,
    events: BTreeMap<u64, EventPending>,
}
struct EventPending {
    owner: u64,
    name: String,
    queue: VecDeque<Arc<Occurrence>>,
    overflow: bool,
}
struct Occurrence {
    generation: u64,
    metadata: Vec<u8>,
    payload: Vec<u8>,
}
struct Pending {
    owner: u64,
    name: String,
    queue: VecDeque<StatusV1>,
    overflow: bool,
}
fn notify(state: &mut State, name: &str, status: StatusV1) {
    for watch in state.watches.values_mut().filter(|w| w.name == name) {
        if status.phase == REVOKED {
            watch
                .queue
                .retain(|s| s.generation != status.generation || s.phase != READY);
        }
        if watch.queue.len() == 64 {
            watch.overflow = true;
            watch.queue.clear();
        }
        if !watch.overflow {
            watch.queue.push_back(status);
        }
    }
}
fn quarantine(state: &mut State, owner: u64) {
    state.faulted.insert(owner);
    invalidate(state);
    let mut revoked = Vec::new();
    for (name, contract) in state
        .contracts
        .iter_mut()
        .filter(|(_, c)| c.status.provider == owner && c.status.phase == READY)
    {
        contract.status.phase = REVOKED;
        contract.status.revision = contract.status.revision.saturating_add(1);
        revoked.push((name.clone(), contract.status));
    }
    for (name, status) in revoked {
        notify(state, &name, status);
    }
}
fn invalidate(state: &mut State) {
    // A process cannot perform u64::MAX registration mutations; never wrap a stamp.
    for contract in state.contracts.values_mut() {
        contract.composition = contract.composition.saturating_add(1);
    }
}
struct Contract {
    descriptor: ContractV1,
    status: StatusV1,
    composition: u64,
    calls: u64,
}
struct Subscription {
    owner: u64,
    contract: String,
    id: String,
    priority: i32,
    mode: u32,
    before: Vec<String>,
    after: Vec<String>,
    function: Arc<Transform>,
}
fn ordered<'a>(state: &'a State, name: &str) -> Result<Vec<&'a Subscription>, i32> {
    let mut pending: Vec<_> = state
        .subscriptions
        .values()
        .filter(|s| s.contract == name && !state.faulted.contains(&s.owner))
        .collect();
    pending.sort_by_key(|s| (s.priority, &s.id, s.owner));
    let mut result = Vec::with_capacity(pending.len());
    while !pending.is_empty() {
        let eligible = pending.iter().position(|candidate| {
            !pending.iter().any(|peer| {
                peer.id != candidate.id
                    && (candidate.after.contains(&peer.id) || peer.before.contains(&candidate.id))
            })
        });
        let index = eligible.ok_or(CONFLICT)?;
        let next = pending.remove(index);
        if next.before.contains(&next.id) || next.after.contains(&next.id) {
            return Err(CONFLICT);
        }
        result.push(next);
    }
    Ok(result)
}
thread_local! { static DISPATCHING: Cell<bool> = const { Cell::new(false) }; }
struct DispatchGuard;
impl Drop for DispatchGuard {
    fn drop(&mut self) {
        DISPATCHING.set(false);
    }
}
fn mutation() -> Result<(), i32> {
    if DISPATCHING.get() {
        Err(REENTRANT)
    } else {
        Ok(())
    }
}

/// Validate a bounded namespaced contract identifier with an explicit protocol version.
pub fn valid_name(name: &str) -> bool {
    name.len() < 128
        && name.contains('.')
        && name.rsplit_once('@').is_some_and(|(_, version)| {
            !version.is_empty() && version.bytes().all(|b| b.is_ascii_digit())
        })
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-/@".contains(&b))
}
/// Decode a bounded NUL-terminated UTF-8 descriptor field.
pub fn text(bytes: &[u8]) -> Result<&str, i32> {
    let end = bytes.iter().position(|b| *b == 0).ok_or(ERROR)?;
    let value = std::str::from_utf8(&bytes[..end]).map_err(|_| ERROR)?;
    if value.is_empty() {
        Err(ERROR)
    } else {
        Ok(value)
    }
}
impl Registry {
    /// Enable inexpensive per-contract dispatch counts for host diagnostics.
    /// Counts accumulate only while enabled and do not invoke consumer callbacks.
    pub fn diagnostics_enabled(&self, enabled: bool) {
        self.0.diagnostics.store(enabled, Ordering::Relaxed);
    }

    /// Copy bounded diagnostic lines without waiting for an occupied registry lock.
    /// Includes provider generation, subscriptions, first use and observed dispatches.
    pub fn diagnostics(&self, limit: usize) -> Vec<String> {
        let Ok(state) = self.0.state.try_lock() else {
            return vec!["Platform registry busy".into()];
        };
        let mut lines = vec![format!(
            "Contracts: {} | registrations: {} | faulted owners: {}",
            state.contracts.len(),
            state.subscriptions.len(),
            state.faulted.len()
        )];
        let mut contracts: Vec<_> = state.contracts.iter().collect();
        contracts.sort_by_key(|(name, contract)| (std::cmp::Reverse(contract.calls), *name));
        for (name, contract) in contracts.into_iter().take(limit.min(32)) {
            let consumers = state
                .subscriptions
                .values()
                .filter(|s| s.contract == *name && !state.faulted.contains(&s.owner))
                .count();
            lines.push(format!(
                "{} {} g{} | owners {} | calls {} | used {}",
                name,
                if contract.status.phase == READY {
                    "ready"
                } else {
                    "revoked"
                },
                contract.status.generation,
                consumers,
                contract.calls,
                contract.status.used
            ));
        }
        lines
    }

    /// Quiesce admitted calls and revoke every provider/consumer of a failed owner.
    /// # Errors
    /// Rejects callback reentry or poisoned synchronization state.
    pub fn quarantine(&self, owner: u64) -> Result<(), i32> {
        mutation()?;
        let _gate = self.0.quiescence.write().map_err(|_| ERROR)?;
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        quarantine(&mut state, owner);
        Ok(())
    }
    /// Publish a usable generation after verifying the native binding and resource lifetime.
    ///
    /// # Errors
    /// Rejects malformed descriptors, conflicting providers, reentry, and exhausted identifiers.
    pub fn publish(&self, owner: u64, descriptor: ContractV1) -> Result<u64, i32> {
        mutation()?;
        let name = text(&descriptor.name)?.to_owned();
        if owner == 0
            || descriptor.struct_size as usize != size_of::<ContractV1>()
            || descriptor.reserved != 0
            || !valid_name(&name)
            || !matches!(descriptor.mode, TRANSFORM | EXCLUSIVE | LIFECYCLE | EVENT)
            || descriptor.max_payload == 0
            || descriptor.max_payload as usize > MAX_PAYLOAD
        {
            return Err(ERROR);
        }
        text(&descriptor.schema)?;
        text(&descriptor.lane)?;
        text(&descriptor.evidence)?;
        if descriptor.mode == EVENT && descriptor.max_payload > 4096 {
            return Err(ERROR);
        }
        let _gate = self.0.quiescence.write().map_err(|_| ERROR)?;
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        if state.faulted.contains(&owner) {
            return Err(ERROR);
        }
        if state.contracts.len() >= 1024 && !state.contracts.contains_key(&name) {
            return Err(ERROR);
        }
        let old = state
            .contracts
            .get(&name)
            .map(|c| c.status)
            .unwrap_or_default();
        if old.phase == READY {
            return Err(HOOK_BUSY);
        }
        if state.contracts.get(&name).is_some_and(|previous| {
            previous.descriptor.schema != descriptor.schema
                || previous.descriptor.mode != descriptor.mode
        }) {
            return Err(CONFLICT);
        }
        if state
            .subscriptions
            .values()
            .any(|s| s.contract == name && s.mode != descriptor.mode)
        {
            return Err(CONFLICT);
        }
        if descriptor.mode != EVENT && state.events.values().any(|e| e.name == name) {
            return Err(CONFLICT);
        }
        let generation = old.generation.checked_add(1).ok_or(ERROR)?;
        let status = StatusV1 {
            provider: owner,
            generation,
            revision: old.revision.checked_add(1).ok_or(ERROR)?,
            phase: READY,
            used: 0,
        };
        state.contracts.insert(
            name.clone(),
            Contract {
                descriptor,
                status,
                composition: generation,
                calls: 0,
            },
        );
        notify(&mut state, &name, status);
        Ok(generation)
    }

    /// Copy provider evidence and its latest generation, including revocation.
    ///
    /// # Errors
    /// Returns UNAVAILABLE when the name has never been published.
    pub fn query(&self, name: &str) -> Result<(ContractV1, StatusV1), i32> {
        let state = self.0.state.lock().map_err(|_| ERROR)?;
        let contract = state
            .contracts
            .get(name)
            .ok_or(rekenshi_plugin_api::hooks::UNAVAILABLE)?;
        Ok((contract.descriptor, contract.status))
    }

    /// Register desired behavior before or after a provider becomes usable.
    ///
    /// Ordering is ascending priority, registration identity, then owner token.
    /// The boolean reports missed operations; it never implies resource reload.
    ///
    /// # Errors
    /// Rejects duplicate identities, incompatible/exclusive consumers, and callback reentry.
    pub fn subscribe(
        &self,
        owner: u64,
        name: &str,
        id: &str,
        priority: i32,
        mode: u32,
        callback: impl Fn(&[u8], &[u8]) -> Result<Option<Vec<u8>>, i32> + Send + Sync + 'static,
    ) -> Result<(u64, bool), i32> {
        mutation()?;
        if owner == 0
            || !valid_name(name)
            || !valid_name(id)
            || !matches!(mode, TRANSFORM | EXCLUSIVE)
        {
            return Err(ERROR);
        }
        let _gate = self.0.quiescence.write().map_err(|_| ERROR)?;
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        if state.faulted.contains(&owner) || state.subscriptions.len() + state.removed.len() >= 8192
        {
            return Err(ERROR);
        }
        if state
            .subscriptions
            .values()
            .any(|s| s.contract == name && (s.id == id || s.mode != mode || mode == EXCLUSIVE))
        {
            return Err(CONFLICT);
        }
        let late = if let Some(contract) = state.contracts.get(name) {
            if contract.descriptor.mode != mode {
                return Err(CONFLICT);
            }
            contract.status.phase == READY && contract.status.used != 0
        } else {
            false
        };
        state.serial = state.serial.checked_add(1).ok_or(ERROR)?;
        let handle = state.serial;
        state.subscriptions.insert(
            handle,
            Subscription {
                owner,
                contract: name.to_owned(),
                id: id.to_owned(),
                priority,
                mode,
                before: Vec::new(),
                after: Vec::new(),
                function: Arc::new(callback),
            },
        );
        if ordered(&state, name).is_err() {
            state.subscriptions.remove(&handle);
            return Err(CONFLICT);
        }
        invalidate(&mut state);
        Ok((handle, late))
    }

    /// Set semantic ordering against optional named peers on the same operation.
    /// Missing peers impose no edge until they register. Cycles reject the mutation.
    ///
    /// # Errors
    /// Rejects foreign handles, invalid edges, cycles or changes after first use.
    pub fn order(
        &self,
        owner: u64,
        handle: u64,
        before: Vec<String>,
        after: Vec<String>,
    ) -> Result<(), i32> {
        mutation()?;
        if before.len() + after.len() > 32 || before.iter().chain(&after).any(|id| !valid_name(id))
        {
            return Err(ERROR);
        }
        let _gate = self.0.quiescence.write().map_err(|_| ERROR)?;
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        let subscription = state.subscriptions.get(&handle).ok_or(ERROR)?;
        if subscription.owner != owner || state.faulted.contains(&owner) {
            return Err(ERROR);
        }
        let name = subscription.contract.clone();
        if state
            .contracts
            .get(&name)
            .is_some_and(|c| c.status.used != 0 && c.status.phase == READY)
        {
            return Err(LATE);
        }
        let subscription = state.subscriptions.get_mut(&handle).ok_or(ERROR)?;
        let old_before = std::mem::replace(&mut subscription.before, before);
        let old_after = std::mem::replace(&mut subscription.after, after);
        if ordered(&state, &name).is_err() {
            let subscription = state.subscriptions.get_mut(&handle).ok_or(ERROR)?;
            subscription.before = old_before;
            subscription.after = old_after;
            return Err(CONFLICT);
        }
        invalidate(&mut state);
        Ok(())
    }
    /// Stable SHA-256 identity of effective ordered policy versions and constraints.
    /// Tokens, generations and mutable captures are excluded; source bytes remain cache inputs.
    ///
    /// # Errors
    /// Rejects unknown contracts or inconsistent ordering.
    pub fn manifest(&self, name: &str) -> Result<[u8; 32], i32> {
        use sha2::{Digest, Sha256};
        let state = self.0.state.lock().map_err(|_| ERROR)?;
        let contract = state.contracts.get(name).ok_or(ERROR)?;
        let mut hash = Sha256::new();
        hash.update(contract.descriptor.name);
        hash.update(contract.descriptor.schema);
        for subscription in ordered(&state, name)? {
            hash.update((subscription.id.len() as u64).to_le_bytes());
            hash.update(subscription.id.as_bytes());
            hash.update(subscription.priority.to_le_bytes());
            for edges in [&subscription.before, &subscription.after] {
                hash.update((edges.len() as u64).to_le_bytes());
                for edge in edges {
                    hash.update((edge.len() as u64).to_le_bytes());
                    hash.update(edge.as_bytes());
                }
            }
        }
        Ok(hash.finalize().into())
    }
    /// Disable an owned registration and wait until admitted callbacks have returned.
    ///
    /// # Errors
    /// Rejects foreign handles and callback reentry.
    pub fn remove(&self, owner: u64, handle: u64) -> Result<(), i32> {
        mutation()?;
        let _gate = self.0.quiescence.write().map_err(|_| ERROR)?;
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        if state.removed.get(&handle) == Some(&owner) {
            return Ok(());
        }
        if state
            .subscriptions
            .get(&handle)
            .is_none_or(|s| s.owner != owner)
        {
            return Err(ERROR);
        }
        state.subscriptions.remove(&handle);
        state.removed.insert(handle, owner);
        invalidate(&mut state);
        Ok(())
    }

    /// Invalidate composed policy after an owner updates its local callback state.
    ///
    /// # Errors
    /// Rejects foreign/removed handles, faulted owners and callback reentry.
    pub fn changed(&self, owner: u64, handle: u64) -> Result<(), i32> {
        mutation()?;
        let _gate = self.0.quiescence.write().map_err(|_| ERROR)?;
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        if state.faulted.contains(&owner)
            || state
                .subscriptions
                .get(&handle)
                .is_none_or(|s| s.owner != owner)
        {
            return Err(ERROR);
        }
        invalidate(&mut state);
        Ok(())
    }

    /// Read a live provider's policy revision without copying its full descriptor.
    ///
    /// # Errors
    /// Rejects stale/foreign/revoked providers and callback reentry.
    pub fn stamp(&self, owner: u64, name: &str, generation: u64) -> Result<(u64, u32), i32> {
        mutation()?;
        let state = self.0.state.lock().map_err(|_| ERROR)?;
        let contract = state.contracts.get(name).ok_or(ERROR)?;
        if contract.status.provider != owner
            || contract.status.generation != generation
            || contract.status.phase != READY
            || state.faulted.contains(&owner)
        {
            return Err(ERROR);
        }
        let count = state
            .subscriptions
            .values()
            .filter(|s| s.contract == name && !state.faulted.contains(&s.owner))
            .count() as u32;
        Ok((contract.composition, count))
    }

    /// Revoke a provider generation before destroying its native resources.
    ///
    /// # Errors
    /// Rejects stale generations, foreign providers, and callback reentry.
    pub fn revoke(&self, owner: u64, name: &str, generation: u64) -> Result<(), i32> {
        mutation()?;
        let _gate = self.0.quiescence.write().map_err(|_| ERROR)?;
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        let contract = state.contracts.get_mut(name).ok_or(ERROR)?;
        if contract.status.provider != owner || contract.status.generation != generation {
            return Err(ERROR);
        }
        if contract.status.phase == REVOKED {
            return Ok(());
        }
        if contract.status.phase != READY {
            return Err(ERROR);
        }
        contract.status.phase = REVOKED;
        contract.status.revision = contract.status.revision.checked_add(1).ok_or(ERROR)?;
        let status = contract.status;
        notify(&mut state, name, status);
        Ok(())
    }

    /// Dispatch on the provider's native lane, composing owned copied payloads.
    ///
    /// An error discards the entire chain; providers must forward the original
    /// operation. A panicking callback quarantines its owner across contracts.
    ///
    /// # Errors
    /// Rejects stale/foreign providers, excessive payloads, callback failures and reentry.
    pub fn dispatch(
        &self,
        owner: u64,
        name: &str,
        generation: u64,
        metadata: &[u8],
        input: &[u8],
    ) -> Result<Vec<u8>, i32> {
        mutation()?;
        let _gate = self.0.quiescence.read().map_err(|_| ERROR)?;
        let (limit, callbacks) = {
            let mut state = self.0.state.lock().map_err(|_| ERROR)?;
            if state.faulted.contains(&owner) {
                return Err(FAULTED);
            }
            let contract = state.contracts.get_mut(name).ok_or(ERROR)?;
            if contract.status.provider != owner
                || contract.status.generation != generation
                || contract.status.phase != READY
                || contract.descriptor.mode == LIFECYCLE
            {
                return Err(ERROR);
            }
            let limit = contract.descriptor.max_payload as usize;
            if input.len() > limit || metadata.len() > 65536 {
                return Err(ERROR);
            }
            contract.status.used = 1;
            if self.0.diagnostics.load(Ordering::Relaxed) {
                contract.calls = contract.calls.saturating_add(1);
            }
            if contract.descriptor.mode == EVENT {
                if metadata.len() > 512 {
                    return Err(ERROR);
                }
                let event = Arc::new(Occurrence {
                    generation,
                    metadata: metadata.to_vec(),
                    payload: input.to_vec(),
                });
                for pending in state.events.values_mut().filter(|e| e.name == name) {
                    if pending.queue.len() == 1024 {
                        pending.overflow = true;
                        pending.queue.clear();
                    }
                    if !pending.overflow {
                        pending.queue.push_back(event.clone());
                    }
                }
                return Ok(input.to_vec());
            }
            let callbacks = ordered(&state, name)?
                .into_iter()
                .map(|s| (s.priority, s.id.clone(), s.owner, s.function.clone()))
                .collect::<Vec<_>>();
            (limit, callbacks)
        };
        DISPATCHING.set(true);
        let _guard = DispatchGuard;
        let mut current = input.to_vec();
        for (_, _, consumer, callback) in callbacks {
            if self
                .0
                .state
                .lock()
                .map_err(|_| ERROR)?
                .faulted
                .contains(&consumer)
            {
                continue;
            }
            let result = match catch_unwind(AssertUnwindSafe(|| callback(metadata, &current))) {
                Ok(result) => result,
                Err(payload) => {
                    std::mem::forget(payload);
                    quarantine(&mut *self.0.state.lock().map_err(|_| ERROR)?, consumer);
                    return Err(FAULTED);
                }
            };
            if result == Err(FAULTED) {
                quarantine(&mut *self.0.state.lock().map_err(|_| ERROR)?, consumer);
            }
            if let Some(next) = result? {
                if next.len() > limit {
                    return Err(ERROR);
                }
                current = next;
            }
        }
        Ok(current)
    }

    /// Quiesce all registrations and revoke every live contract owned by a plugin.
    ///
    /// # Errors
    /// Rejects callback reentry; poisoned state requires restart.
    pub fn release(&self, owner: u64) -> Result<(), i32> {
        mutation()?;
        let _gate = self.0.quiescence.write().map_err(|_| ERROR)?;
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        let removed: Vec<_> = state
            .subscriptions
            .iter()
            .filter(|(_, s)| s.owner == owner)
            .map(|(handle, _)| *handle)
            .collect();
        for handle in removed {
            state.subscriptions.remove(&handle);
            state.removed.insert(handle, owner);
        }
        invalidate(&mut state);
        state.watches.retain(|_, w| w.owner != owner);
        state.events.retain(|_, w| w.owner != owner);
        let mut revoked = Vec::new();
        for (name, contract) in state
            .contracts
            .iter_mut()
            .filter(|(_, c)| c.status.provider == owner && c.status.phase == READY)
        {
            contract.status.phase = REVOKED;
            contract.status.revision = contract.status.revision.checked_add(1).ok_or(ERROR)?;
            revoked.push((name.clone(), contract.status));
        }
        for (name, status) in revoked {
            notify(&mut state, &name, status);
        }
        Ok(())
    }

    /// Watch provider generation changes with bounded copied current-state replay.
    ///
    /// # Errors
    /// Rejects malformed names, callback reentry, faulted owners and capacity exhaustion.
    pub fn watch(&self, owner: u64, name: &str) -> Result<u64, i32> {
        mutation()?;
        if owner == 0 || !valid_name(name) {
            return Err(ERROR);
        }
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        if state.watches.len() >= 8192 || state.faulted.contains(&owner) {
            return Err(ERROR);
        }
        let queue = state
            .contracts
            .get(name)
            .map(|c| VecDeque::from([c.status]))
            .unwrap_or_default();
        state.serial = state.serial.checked_add(1).ok_or(ERROR)?;
        let handle = state.serial;
        state.watches.insert(
            handle,
            Pending {
                owner,
                name: name.to_owned(),
                queue,
                overflow: false,
            },
        );
        Ok(handle)
    }
    /// Drain one copied transition, discarding ready events superseded by revocation.
    ///
    /// # Errors
    /// Rejects foreign owners, missing watches and queue overflow.
    pub fn next_transition(&self, owner: u64, handle: u64) -> Result<Option<StatusV1>, i32> {
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        let pending = state.watches.get_mut(&handle).ok_or(ERROR)?;
        if pending.owner != owner || pending.overflow {
            return Err(ERROR);
        }
        Ok(pending.queue.pop_front())
    }
    /// Remove an owned lifecycle subscription.
    ///
    /// # Errors
    /// Rejects missing watches and foreign owners.
    pub fn unwatch(&self, owner: u64, handle: u64) -> Result<(), i32> {
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        if let Some(event) = state.events.get(&handle) {
            if event.owner != owner {
                return Err(ERROR);
            }
            state.events.remove(&handle);
            return Ok(());
        }
        if state.watches.get(&handle).is_none_or(|w| w.owner != owner) {
            return Err(ERROR);
        }
        state.watches.remove(&handle);
        Ok(())
    }
    /// Watch copied future occurrences without replaying historical operations.
    ///
    /// # Errors
    /// Rejects invalid owners/names, incompatible contracts, faulted owners and capacity.
    pub fn watch_events(&self, owner: u64, name: &str) -> Result<u64, i32> {
        mutation()?;
        if owner == 0 || !valid_name(name) {
            return Err(ERROR);
        }
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        if state.events.len() >= 256 || state.faulted.contains(&owner) {
            return Err(ERROR);
        }
        if state
            .contracts
            .get(name)
            .is_some_and(|c| c.descriptor.mode != EVENT)
        {
            return Err(CONFLICT);
        }
        state.serial = state.serial.checked_add(1).ok_or(ERROR)?;
        let handle = state.serial;
        state.events.insert(
            handle,
            EventPending {
                owner,
                name: name.to_owned(),
                queue: VecDeque::new(),
                overflow: false,
            },
        );
        Ok(handle)
    }
    /// Drain one copied event, rejecting silent loss and foreign ownership.
    ///
    /// # Errors
    /// Rejects unknown streams, foreign owners and overflow.
    pub fn next_event(&self, owner: u64, handle: u64) -> Result<Option<EventV1>, i32> {
        let mut state = self.0.state.lock().map_err(|_| ERROR)?;
        let pending = state.events.get_mut(&handle).ok_or(ERROR)?;
        if pending.owner != owner || pending.overflow {
            return Err(ERROR);
        }
        Ok(pending.queue.pop_front().map(|event| {
            let mut output = EventV1 {
                generation: event.generation,
                metadata_len: event.metadata.len() as u32,
                payload_len: event.payload.len() as u32,
                metadata: [0; 512],
                payload: [0; 4096],
            };
            output.metadata[..event.metadata.len()].copy_from_slice(&event.metadata);
            output.payload[..event.payload.len()].copy_from_slice(&event.payload);
            output
        }))
    }
}
impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry").finish_non_exhaustive()
    }
}
