//! Behavioral checks for public registration, generation and cleanup contracts.
use rekenshi_plugin_api::{ERROR, HOOK_BUSY, platform::*};
use rekenshi_plugin_runtime::platform::Registry;
use std::{
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

const NAME: &str = "tests.source@1";

#[test]
fn diagnostics_observe_real_dispatch_without_executing_policy_and_pause_when_disabled() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let registry = Registry::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let (handle, _) = registry
        .subscribe(2, NAME, "tests.observe@1", 0, TRANSFORM, move |_, _| {
            counter.fetch_add(1, Ordering::Relaxed);
            Ok(None)
        })
        .unwrap();
    let generation = registry.publish(1, descriptor(TRANSFORM)).unwrap();
    registry.dispatch(1, NAME, generation, b"", b"x").unwrap();
    assert!(
        registry
            .diagnostics(7)
            .iter()
            .any(|s| s.contains("calls 0"))
    );
    registry.diagnostics_enabled(true);
    registry.dispatch(1, NAME, generation, b"", b"x").unwrap();
    let lines = registry.diagnostics(7);
    assert!(
        lines
            .iter()
            .any(|s| s.contains("owners 1 | calls 1 | used 1"))
    );
    assert_eq!(
        calls.load(Ordering::Relaxed),
        2,
        "reading diagnostics never invokes policy"
    );
    registry.diagnostics_enabled(false);
    registry.dispatch(1, NAME, generation, b"", b"x").unwrap();
    assert!(
        registry
            .diagnostics(7)
            .iter()
            .any(|s| s.contains("calls 1"))
    );
    registry.remove(2, handle).unwrap();
    registry.revoke(1, NAME, generation).unwrap();
    assert!(
        registry
            .diagnostics(7)
            .iter()
            .any(|s| s.contains("revoked g1 | owners 0"))
    );
    registry.publish(1, descriptor(TRANSFORM)).unwrap();
    assert!(
        registry
            .diagnostics(7)
            .iter()
            .any(|s| s.contains("ready g2 | owners 0 | calls 0 | used 0"))
    );
}

#[test]
fn semantic_order_overrides_priority_rejects_cycles_and_stabilizes_cache_identity() {
    let registry = Registry::default();
    let (a, _) = registry
        .subscribe(2, NAME, "tests.a@1", -10, TRANSFORM, |_, input| {
            Ok(Some([input, b"a"].concat()))
        })
        .unwrap();
    registry
        .order(2, a, vec![], vec!["tests.b@1".into()])
        .unwrap();
    let (b, _) = registry
        .subscribe(3, NAME, "tests.b@1", 10, TRANSFORM, |_, input| {
            Ok(Some([input, b"b"].concat()))
        })
        .unwrap();
    assert_eq!(
        registry.order(3, b, vec![], vec!["tests.a@1".into()]),
        Err(CONFLICT)
    );
    let generation = registry.publish(1, descriptor(TRANSFORM)).unwrap();
    let manifest = registry.manifest(NAME).unwrap();
    assert_eq!(
        registry.dispatch(1, NAME, generation, b"", b"").unwrap(),
        b"ba"
    );
    assert_eq!(registry.order(2, a, vec![], vec![]), Err(LATE));
    registry.revoke(1, NAME, generation).unwrap();
    registry.publish(1, descriptor(TRANSFORM)).unwrap();
    assert_eq!(
        manifest,
        registry.manifest(NAME).unwrap(),
        "resource generations do not change persistent policy identity"
    );
    registry.remove(3, b).unwrap();
    assert_ne!(manifest, registry.manifest(NAME).unwrap());
}

#[test]
fn copied_events_preserve_occurrence_generation_enforce_ownership_and_report_loss() {
    let registry = Registry::default();
    let watch = registry.watch_events(2, NAME).unwrap();
    let generation = registry.publish(1, descriptor(EVENT)).unwrap();
    registry
        .dispatch(1, NAME, generation, b"metadata", b"observed")
        .unwrap();
    registry.revoke(1, NAME, generation).unwrap();
    assert!(registry.next_event(3, watch).is_err());
    let event = registry.next_event(2, watch).unwrap().unwrap();
    assert_eq!(
        (event.generation, event.metadata(), event.data()),
        (generation, b"metadata".as_slice(), b"observed".as_slice())
    );
    let second = registry.publish(1, descriptor(EVENT)).unwrap();
    for _ in 0..1025 {
        registry.dispatch(1, NAME, second, b"", b"x").unwrap();
    }
    assert!(
        registry.next_event(2, watch).is_err(),
        "overflow cannot silently discard occurrences"
    );
    registry.release(2).unwrap();
    assert!(registry.next_event(2, watch).is_err());
}
fn field<const N: usize>(value: &str) -> [u8; N] {
    let mut result = [0; N];
    result[..value.len()].copy_from_slice(value.as_bytes());
    result
}
fn descriptor(mode: u32) -> ContractV1 {
    ContractV1 {
        struct_size: size_of::<ContractV1>() as u32,
        mode,
        max_payload: 1024,
        reserved: 0,
        name: field(NAME),
        schema: field("tests.bytes@1"),
        lane: field("test caller; copied bytes only"),
        evidence: field("Pure transport fixture; no native game support claimed"),
    }
}

#[test]
fn pending_registrations_compose_before_first_use_in_identity_order() {
    let registry = Registry::default();
    for (owner, id, suffix) in [(2, "tests.z@1", b'z'), (3, "tests.a@1", b'a')] {
        assert!(
            !registry
                .subscribe(owner, NAME, id, 0, TRANSFORM, move |metadata, input| {
                    assert_eq!(metadata, b"profile");
                    Ok(Some([input, &[suffix]].concat()))
                })
                .unwrap()
                .1
        );
    }
    let generation = registry.publish(1, descriptor(TRANSFORM)).unwrap();
    assert_eq!(registry.query(NAME).unwrap().1.used, 0);
    assert_eq!(
        registry
            .dispatch(1, NAME, generation, b"profile", b"source")
            .unwrap(),
        b"sourceaz"
    );
    assert!(
        registry
            .subscribe(4, NAME, "tests.late@1", -10, TRANSFORM, |_, input| Ok(
                Some([b"early:".as_slice(), input].concat())
            ))
            .unwrap()
            .1
    );
    assert_eq!(
        registry
            .dispatch(1, NAME, generation, b"profile", b"source")
            .unwrap(),
        b"early:sourceaz"
    );
}

#[test]
fn readiness_does_not_mean_first_use_and_generation_replacement_rejects_stale_calls() {
    let registry = Registry::default();
    let first = registry.publish(1, descriptor(TRANSFORM)).unwrap();
    let (handle, late) = registry
        .subscribe(2, NAME, "tests.policy@1", 0, TRANSFORM, |_, input| {
            Ok(Some([input, b"!"].concat()))
        })
        .unwrap();
    assert!(!late);
    assert_eq!(registry.publish(3, descriptor(TRANSFORM)), Err(HOOK_BUSY));
    assert!(registry.revoke(3, NAME, first).is_err());
    registry.revoke(1, NAME, first).unwrap();
    assert!(registry.dispatch(1, NAME, first, b"", b"a").is_err());
    let second = registry.publish(3, descriptor(TRANSFORM)).unwrap();
    assert!(second > first);
    assert!(registry.dispatch(1, NAME, first, b"", b"a").is_err());
    assert_eq!(
        registry.dispatch(3, NAME, second, b"", b"a").unwrap(),
        b"a!"
    );
    assert!(registry.remove(3, handle).is_err());
    registry.remove(2, handle).unwrap();
    registry.remove(2, handle).unwrap();
    assert_eq!(registry.dispatch(3, NAME, second, b"", b"a").unwrap(), b"a");
}

#[test]
fn exclusive_contracts_reject_competing_consumers_even_while_waiting() {
    let registry = Registry::default();
    registry
        .subscribe(2, NAME, "tests.first@1", 0, EXCLUSIVE, |_, _| {
            Ok(Some(b"replacement".to_vec()))
        })
        .unwrap();
    assert_eq!(
        registry.subscribe(3, NAME, "tests.second@1", -100, EXCLUSIVE, |_, _| Ok(None)),
        Err(CONFLICT)
    );
    assert_eq!(registry.publish(1, descriptor(TRANSFORM)), Err(CONFLICT));
    let generation = registry.publish(1, descriptor(EXCLUSIVE)).unwrap();
    assert_eq!(
        registry
            .dispatch(1, NAME, generation, b"", b"original")
            .unwrap(),
        b"replacement"
    );
}

#[test]
fn failed_composition_never_returns_partial_output() {
    let registry = Registry::default();
    let generation = registry.publish(1, descriptor(TRANSFORM)).unwrap();
    registry
        .subscribe(2, NAME, "tests.first@1", 0, TRANSFORM, |_, _| {
            Ok(Some(b"partial".to_vec()))
        })
        .unwrap();
    registry
        .subscribe(3, NAME, "tests.second@1", 1, TRANSFORM, |_, _| Err(ERROR))
        .unwrap();
    assert_eq!(
        registry.dispatch(1, NAME, generation, b"", b"original"),
        Err(ERROR)
    );
    registry.release(3).unwrap();
    assert_eq!(
        registry
            .dispatch(1, NAME, generation, b"", b"original")
            .unwrap(),
        b"partial"
    );
}

#[test]
fn lifecycle_replay_cannot_deliver_ready_after_that_generation_was_revoked() {
    let registry = Registry::default();
    let first = registry.publish(1, descriptor(LIFECYCLE)).unwrap();
    let watch = registry.watch(2, NAME).unwrap();
    registry.revoke(1, NAME, first).unwrap();
    let second = registry.publish(1, descriptor(LIFECYCLE)).unwrap();
    let revoked = registry.next_transition(2, watch).unwrap().unwrap();
    let ready = registry.next_transition(2, watch).unwrap().unwrap();
    assert_eq!((revoked.generation, revoked.phase), (first, REVOKED));
    assert_eq!((ready.generation, ready.phase), (second, READY));
    assert!(ready.revision > revoked.revision);
    assert_eq!(registry.next_transition(2, watch).unwrap(), None);
    assert!(registry.dispatch(1, NAME, second, b"", b"").is_err());
}

#[test]
fn late_watch_replays_current_generation_once_and_enforces_ownership() {
    let registry = Registry::default();
    let waiting = registry.watch(2, NAME).unwrap();
    assert_eq!(registry.next_transition(2, waiting).unwrap(), None);
    let generation = registry.publish(1, descriptor(LIFECYCLE)).unwrap();
    assert_eq!(
        registry
            .next_transition(2, waiting)
            .unwrap()
            .unwrap()
            .generation,
        generation
    );
    let late = registry.watch(3, NAME).unwrap();
    assert!(registry.next_transition(2, late).is_err());
    assert_eq!(
        registry
            .next_transition(3, late)
            .unwrap()
            .unwrap()
            .generation,
        generation
    );
    assert_eq!(registry.next_transition(3, late).unwrap(), None);
    registry.release(1).unwrap();
    assert_eq!(
        registry.next_transition(3, late).unwrap().unwrap().phase,
        REVOKED
    );
}

#[test]
fn lifecycle_overflow_is_explicit_instead_of_silent_loss() {
    let registry = Registry::default();
    let watch = registry.watch(2, NAME).unwrap();
    for _ in 0..70 {
        let generation = registry.publish(1, descriptor(LIFECYCLE)).unwrap();
        registry.revoke(1, NAME, generation).unwrap();
    }
    assert_eq!(registry.next_transition(2, watch), Err(ERROR));
}

#[test]
fn reentry_fails_promptly_and_panic_quarantines_all_contracts_for_the_owner() {
    let registry = Registry::default();
    let generation = registry.publish(1, descriptor(TRANSFORM)).unwrap();
    let nested = registry.clone();
    registry
        .subscribe(2, NAME, "tests.bad@1", 0, TRANSFORM, move |_, _| {
            assert_eq!(nested.release(2), Err(REENTRANT));
            assert_eq!(
                nested.dispatch(1, NAME, generation, b"", b""),
                Err(REENTRANT)
            );
            panic!("adversarial interceptor");
        })
        .unwrap();
    registry
        .subscribe(2, NAME, "tests.unreachable@1", 1, TRANSFORM, |_, _| {
            panic!("must be quarantined")
        })
        .unwrap();
    assert_eq!(
        registry.dispatch(1, NAME, generation, b"", b"original"),
        Err(FAULTED)
    );
    assert_eq!(
        registry
            .dispatch(1, NAME, generation, b"", b"original")
            .unwrap(),
        b"original"
    );
    assert!(
        registry
            .subscribe(2, NAME, "tests.retry@1", 0, TRANSFORM, |_, _| Ok(None))
            .is_err()
    );
}

#[test]
fn shutdown_waits_for_native_callback_and_drops_captures_before_return() {
    let registry = Registry::default();
    let generation = registry.publish(1, descriptor(TRANSFORM)).unwrap();
    let (started, entered) = mpsc::channel();
    let (resume, paused) = mpsc::channel();
    let paused = Mutex::new(paused);
    let capture = Arc::new(());
    let weak = Arc::downgrade(&capture);
    let (handle, _) = registry
        .subscribe(2, NAME, "tests.blocking@1", 0, TRANSFORM, move |_, _| {
            let _keep = &capture;
            started.send(()).unwrap();
            paused
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            Ok(None)
        })
        .unwrap();
    let copy = registry.clone();
    let dispatch = std::thread::spawn(move || copy.dispatch(1, NAME, generation, b"", b"original"));
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let copy = registry.clone();
    let (stopped, done) = mpsc::channel();
    let shutdown = std::thread::spawn(move || {
        copy.release(2).unwrap();
        stopped.send(()).unwrap();
    });
    assert!(done.recv_timeout(Duration::from_millis(30)).is_err());
    assert!(weak.upgrade().is_some());
    resume.send(()).unwrap();
    assert_eq!(dispatch.join().unwrap().unwrap(), b"original");
    done.recv_timeout(Duration::from_secs(5)).unwrap();
    shutdown.join().unwrap();
    assert!(weak.upgrade().is_none());
    // Host owner cleanup can precede the SDK's explicit handle drop.
    registry.remove(2, handle).unwrap();
}
