//! Bounded copied inventory and event stream. No resource/interface ownership.
use crate::resources::Description;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const MAX_LIVE: usize = 100_000;
const MAX_EVENTS_BYTES: u64 = 256 * 1024 * 1024;
#[derive(Clone, Serialize)]
pub(super) struct Resource {
    pub id: u64,
    pub created_us: u64,
    pub description: Description,
    pub estimated_payload_bytes: Option<u64>,
    pub ogre_load_name: String,
    pub debug_name: String,
    pub origin_frames: Vec<String>,
    pub activity: Activity,
    pub batch: Option<super::batch::Batch>,
}
#[derive(Clone, Default, Serialize)]
pub(super) struct Activity {
    pub maps: u64,
    pub map_reads: u64,
    pub map_writes: u64,
    pub unmaps: u64,
    pub copy_source: u64,
    pub copy_destination: u64,
    pub updates: u64,
    pub last_activity_us: Option<u64>,
}
pub(super) enum Operation {
    Map(i32),
    Unmap,
    CopySource,
    CopyDestination,
    Update,
}
pub(super) struct Store {
    pub epoch: u64,
    activity_tag_failures: u64,
    pub directory: PathBuf,
    pub started: Instant,
    pub last_report: Instant,
    serial: u64,
    live: HashMap<u64, Resource>,
    writer: BufWriter<File>,
    event_bytes: u64,
    created: u64,
    destroyed: u64,
    untracked: u64,
    attach_failures: u64,
    dropped_events: u64,
    io_errors: u64,
    known_live_bytes: u64,
    peak_known_bytes: u64,
    peak_live_count: usize,
    closed: bool,
}
impl Store {
    pub fn open(root: &Path) -> Result<Self, String> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let directory = root.join("graphics-captures").join(stamp.to_string());
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("events.jsonl"))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            epoch: u64::try_from(stamp).map_err(|e| e.to_string())?,
            activity_tag_failures: 0,
            directory,
            started: Instant::now(),
            last_report: Instant::now(),
            serial: 0,
            live: HashMap::new(),
            writer: BufWriter::with_capacity(64 * 1024, file),
            event_bytes: 0,
            created: 0,
            destroyed: 0,
            untracked: 0,
            attach_failures: 0,
            dropped_events: 0,
            io_errors: 0,
            known_live_bytes: 0,
            peak_known_bytes: 0,
            peak_live_count: 0,
            closed: false,
        })
    }
    fn event(&mut self, value: &impl Serialize) {
        if self.event_bytes >= MAX_EVENTS_BYTES {
            self.dropped_events += 1;
            return;
        }
        match serde_json::to_vec(value) {
            Ok(mut bytes) => {
                bytes.push(b'\n');
                self.event_bytes += bytes.len() as u64;
                if self.writer.write_all(&bytes).is_err() {
                    self.io_errors += 1;
                }
            }
            Err(_) => self.io_errors += 1,
        }
    }
    pub fn create(
        &mut self,
        description: Description,
        ogre_load_name: String,
        debug_name: String,
        origin_frames: Vec<String>,
        batch: Option<super::batch::Batch>,
    ) -> Option<u64> {
        if self.closed {
            return None;
        }
        self.created += 1;
        if self.live.len() >= MAX_LIVE {
            self.untracked += 1;
            return None;
        }
        self.serial += 1;
        let id = self.serial;
        let resource = Resource {
            id,
            created_us: self.started.elapsed().as_micros() as u64,
            estimated_payload_bytes: description.estimated_bytes(),
            description,
            ogre_load_name,
            debug_name,
            origin_frames,
            activity: Activity::default(),
            batch,
        };
        self.known_live_bytes += resource.estimated_payload_bytes.unwrap_or(0);
        self.peak_known_bytes = self.peak_known_bytes.max(self.known_live_bytes);
        self.peak_live_count = self.peak_live_count.max(self.live.len() + 1);
        self.event(&serde_json::json!({"event":"create", "resource":resource}));
        self.live.insert(id, resource);
        Some(id)
    }
    pub fn retire(&mut self, id: u64, attached: bool) {
        if self.closed {
            return;
        }
        if let Some(resource) = self.live.remove(&id) {
            self.known_live_bytes -= resource.estimated_payload_bytes.unwrap_or(0);
            if attached {
                self.destroyed += 1;
            } else {
                self.attach_failures += 1;
            }
            self.event(
                &serde_json::json!({"event":if attached {"destroy"} else {"tracking_failed"},
                "id":id, "activity":resource.activity, "elapsed_us":self.started.elapsed().as_micros() as u64}),
            );
        }
    }
    pub fn tag_failed(&mut self) {
        self.activity_tag_failures += 1;
    }
    pub fn activity(&mut self, epoch: u64, id: u64, operation: Operation) {
        if self.closed || self.epoch != epoch {
            return;
        }
        let Some(resource) = self.live.get_mut(&id) else {
            return;
        };
        let a = &mut resource.activity;
        match operation {
            Operation::Map(mode) => {
                a.maps += 1;
                a.map_reads += u64::from(matches!(mode, 1 | 3));
                a.map_writes += u64::from(matches!(mode, 2..=5));
            }
            Operation::Unmap => a.unmaps += 1,
            Operation::CopySource => a.copy_source += 1,
            Operation::CopyDestination => a.copy_destination += 1,
            Operation::Update => a.updates += 1,
        }
        a.last_activity_us = Some(self.started.elapsed().as_micros() as u64);
    }
    pub fn snapshot(&mut self, final_report: bool) -> Result<(), String> {
        if self.closed && !final_report {
            return Ok(());
        }
        self.closed |= final_report;
        let mut resources: Vec<_> = self.live.values().collect();
        resources.sort_by_key(|r| std::cmp::Reverse(r.estimated_payload_bytes.unwrap_or(0)));
        let unknown = resources
            .iter()
            .filter(|r| r.estimated_payload_bytes.is_none())
            .count();
        let report = serde_json::json!({
            "schema_version":3, "session_id":self.epoch.to_string(), "final":final_report, "elapsed_us":self.started.elapsed().as_micros() as u64,
            "measurement":"logical resource payload estimates; not system RAM or dedicated VRAM",
            "coverage":"successful CreateBuffer/CreateTexture1D/2D/3D after capture enabled on the probed hardware D3D11 implementation; excludes earlier resources, swap-chain buffers, shared/opened resources and other device implementations",
            "name_semantics":"ogre_load_name is the synchronous enclosing Ogre Resource::load/prepare scope, not proof of exclusive ownership; debug_name is copied at creation when available",
            "origin_semantics":"bounded creation stack, module names plus relative addresses; correlation rather than exclusive ownership",
            "batch_semantics":"copied live input queue of fingerprinted Forests SubBatch/WindSubBatch build; copies count submitted submeshes, not distinct entities. Per-source vertex weights estimate contribution only when their sum equals output_vertices. Placement schema 1 copies position, wxyz orientation, scale and renderer-native color in builder input space plus the parent batch centering origin, not established world coordinates. Omitted/invalid placements are explicit. No native pointers retained",
            "batch_attribution_unavailable":super::HOOKS.get().and_then(|h| h.batch_unavailable.as_ref()),
            "batch_snapshot_failures_process_total":super::batch::failures(),
            "activity_semantics":"staging resources only: successful Map, Unmap, CopyResource, CopySubresourceRegion and UpdateSubresource on the probed context implementation; excludes other context implementations and operations, so no observed activity does not prove unnecessary allocation",
            "activity_tag_failures":self.activity_tag_failures,
            "lifetime_semantics":"destroy means resource-owned private COM marker released; replacing the private GUID can also retire it; no strong resource references retained",
            "created":self.created, "destroyed":self.destroyed, "live_count":self.live.len(),
            "known_live_payload_bytes":self.known_live_bytes, "unknown_size_live_count":unknown,
            "peak_known_payload_bytes":self.peak_known_bytes, "peak_live_count":self.peak_live_count,
            "untracked_creations":self.untracked, "attachment_failures":self.attach_failures,
            "dropped_events":self.dropped_events, "io_errors":self.io_errors,
            "resources":resources
        });
        self.writer.flush().map_err(|e| e.to_string())?;
        let pending = self.directory.join("summary.pending");
        std::fs::write(
            &pending,
            serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(pending, self.directory.join("summary.json")).map_err(|e| e.to_string())?;
        self.last_report = Instant::now();
        Ok(())
    }
}
