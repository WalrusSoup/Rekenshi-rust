//! Test-only live command probe. Acts only on an explicit request file.
use rekenshi_plugin_sdk::{
    Context, Plugin, Settings,
    gameplay::{Character, Command, CommandResult, Gameplay, Position},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Default)]
struct Probe {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl Plugin for Probe {
    fn id(&self) -> &'static str {
        "gameplay-probe"
    }
    fn start(&mut self, context: &Context, _: &Settings) -> Result<(), rekenshi_plugin_sdk::Error> {
        let gameplay =
            Gameplay::connect(context).map_err(rekenshi_plugin_sdk::Error::from_cause)?;
        let root = context.root.clone();
        let stop = Arc::clone(&self.stop);
        stop.store(false, Ordering::Release);
        self.worker = Some(
            thread::Builder::new()
                .name("gameplay-probe".into())
                .spawn(move || run(gameplay, root, stop))
                .map_err(rekenshi_plugin_sdk::Error::from_cause)?,
        );
        Ok(())
    }
    fn stop(&mut self) -> Result<(), rekenshi_plugin_sdk::Error> {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            worker
                .join()
                .map_err(|_| "gameplay probe worker panicked")?;
        }
        Ok(())
    }
}
rekenshi_plugin_sdk::export_plugin!(Probe);

fn position(p: Position) -> Value {
    json!({"x":p.x,"y":p.y,"z":p.z})
}
fn character(c: &Character) -> Value {
    json!({"id":{"world":c.id.world,"id":c.id.id},"name":c.name,"position":c.position.map(position)})
}
fn save(path: &Path, report: &Value) -> Result<(), rekenshi_plugin_sdk::Error> {
    let bytes =
        serde_json::to_vec_pretty(report).map_err(rekenshi_plugin_sdk::Error::from_cause)?;
    fs::write(path, bytes).map_err(rekenshi_plugin_sdk::Error::from_cause)
}
fn run(gameplay: Gameplay, root: PathBuf, stop: Arc<AtomicBool>) {
    let request_path = root.join("gameplay-probe.request");
    let result_path = root.join("gameplay-probe.result.json");
    // Persist the highest consumed ID, including an in-progress request, so a
    // leftover request never silently replays after host/plugin restart.
    let mut last = fs::read(&result_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|v| v["request"].as_u64())
        .unwrap_or(0);
    while !stop.load(Ordering::Acquire) {
        let request = fs::metadata(&request_path)
            .ok()
            .filter(|m| m.len() <= 256)
            .and_then(|_| fs::read_to_string(&request_path).ok());
        if let Some(request) = request {
            let words: Vec<_> = request.split_whitespace().collect();
            let next = (words.len() == 2)
                .then(|| words[0].parse::<u64>().ok())
                .flatten()
                .filter(|id| *id > last);
            if let Some(id) = next {
                last = id;
                let mut report = json!({"request":id,"verb":words[1],"result":{"state":"started"}});
                // Record consumption before enqueue; failure to record forbids dispatch.
                if save(&result_path, &report).is_ok() {
                    let result = probe(&gameplay, words[1], &stop, &mut report);
                    if let Err(error) = result {
                        report["result"] = json!({"state":"error","message":error.to_string()});
                    }
                    let _ = save(&result_path, &report);
                }
            }
        }
        if !stop.load(Ordering::Acquire) {
            thread::park_timeout(Duration::from_millis(100));
        }
    }
}
fn probe(
    gameplay: &Gameplay,
    verb: &str,
    stop: &AtomicBool,
    report: &mut Value,
) -> Result<(), rekenshi_plugin_sdk::Error> {
    if !matches!(verb, "move-here" | "stop") {
        return Err("unknown verb; expected move-here or stop".into());
    }
    let snapshot = gameplay
        .snapshot()
        .map_err(rekenshi_plugin_sdk::Error::from_cause)?;
    let actor = snapshot
        .characters
        .iter()
        .find(|c| c.selected == Some(true) && c.position.is_some())
        .or_else(|| snapshot.characters.iter().find(|c| c.position.is_some()))
        .ok_or("no player character with a valid position")?;
    report["character"] = character(actor);
    report["before"] = actor.position.map(position).unwrap_or(Value::Null);
    let command = match verb {
        "move-here" => Command::MoveTo {
            character: actor.id,
            position: actor.position.ok_or("position unavailable")?,
        },
        _ => Command::Stop {
            character: actor.id,
        },
    };
    if !snapshot.capabilities.supports(&command) {
        return Err("requested command unsupported by adapter".into());
    }
    if stop.load(Ordering::Acquire) {
        return Err("shutdown before enqueue".into());
    }
    let ticket = gameplay
        .enqueue(command)
        .map_err(rekenshi_plugin_sdk::Error::from_cause)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if stop.load(Ordering::Acquire) {
            return Err("shutdown while awaiting command result".into());
        }
        match gameplay
            .take_result(ticket)
            .map_err(rekenshi_plugin_sdk::Error::from_cause)?
        {
            CommandResult::Pending if Instant::now() < deadline => {
                thread::park_timeout(Duration::from_millis(50))
            }
            CommandResult::Pending => {
                return Err(
                    "timed out awaiting dispatch; ticket remains host-owned until plugin stops"
                        .into(),
                );
            }
            CommandResult::Applied => {
                report["result"] =
                    json!({"state":"applied","meaning":"native function dispatched"});
                break;
            }
            CommandResult::Rejected { code, message } => {
                report["result"] = json!({"state":"rejected","code":code,"message":message});
                break;
            }
        }
    }
    if let Ok(after) = gameplay.snapshot() {
        report["after"] = after
            .characters
            .iter()
            .find(|c| c.id == actor.id)
            .map(character)
            .unwrap_or(Value::Null);
        report["after_sequence"] = json!(after.sequence);
    }
    Ok(())
}
