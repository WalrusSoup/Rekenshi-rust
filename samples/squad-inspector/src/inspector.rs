#![forbid(unsafe_code)]

use rekenshi_plugin_sdk::gameplay::{
    Availability, Character, CharacterId, Gameplay, GameplayError,
};
use rekenshi_plugin_sdk::{Context, Plugin, Settings};
use serde_json::{Value, json};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Default, Debug)]
pub struct SquadInspector {
    stop: Option<Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl Plugin for SquadInspector {
    fn id(&self) -> &'static str {
        "squad-inspector"
    }
    fn title(&self) -> &'static str {
        "Live squad inspector"
    }

    fn start(&mut self, context: &Context, _: &Settings) -> Result<(), rekenshi_plugin_sdk::Error> {
        let gameplay = Gameplay::connect(context);
        let path = context.root.join("squad-inspector.json");
        let logger = context.host.logger();
        let (stop, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("squad-inspector".into())
            .spawn(move || {
                let mut previous = None;
                let mut last_write = None::<Instant>;
                let mut last_error = None::<Instant>;
                loop {
                    let current = document(&gameplay);
                    let due =
                        last_write.is_none_or(|time| time.elapsed() >= Duration::from_secs(1));
                    if due && previous.as_ref() != Some(&current) {
                        let written = serde_json::to_vec_pretty(&current)
                            .map_err(std::io::Error::other)
                            .and_then(|bytes| std::fs::write(&path, bytes));
                        last_write = Some(Instant::now());
                        match written {
                            Ok(()) => previous = Some(current),
                            Err(error)
                                if last_error.is_none_or(|time| {
                                    time.elapsed() >= Duration::from_secs(30)
                                }) =>
                            {
                                logger.log(&format!(
                                    "squad inspector could not write {}: {error}",
                                    path.display()
                                ));
                                last_error = Some(Instant::now());
                            }
                            Err(_) => {}
                        }
                    }
                    match receiver.recv_timeout(Duration::from_millis(250)) {
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        _ => break,
                    }
                }
            })
            .map_err(|error| format!("start squad inspector: {error}"))?;
        self.stop = Some(stop);
        self.worker = Some(worker);
        context.log(&format!(
            "live squad inspector writes {}",
            context.root.join("squad-inspector.json").display()
        ));
        Ok(())
    }

    fn stop(&mut self) -> Result<(), rekenshi_plugin_sdk::Error> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| "squad inspector worker panicked".to_owned())?;
        }
        Ok(())
    }
}

fn document(gameplay: &Result<Gameplay, GameplayError>) -> Value {
    let service = match gameplay {
        Ok(service) => service,
        Err(error) => {
            return json!({"status": "unsupported", "message": error.to_string(), "characters": []});
        }
    };
    let status = match service.status() {
        Ok(status) => status,
        Err(error) => {
            return json!({"status": "waiting", "message": error.to_string(), "characters": []});
        }
    };
    let availability = match status.availability {
        Availability::Ready => "ready",
        Availability::NoWorld => "no-world",
        Availability::AdapterUnavailable => "unsupported",
        Availability::Unknown(_) => "waiting",
    };
    let mut output = json!({
        "status": availability, "message": status.message,
        "world": status.world.to_string(), "capabilities": status.capabilities.0,
        "characters": [],
    });
    if status.availability == Availability::Ready {
        match service.snapshot() {
            Ok(snapshot) => {
                output["world"] = json!(snapshot.world.to_string());
                output["capabilities"] = json!(snapshot.capabilities.0);
                output["characters"] =
                    Value::Array(snapshot.characters.iter().map(character).collect());
            }
            Err(error) => {
                output["status"] = json!("waiting");
                output["message"] = json!(error.to_string());
            }
        }
    }
    output
}

// Decimal strings keep opaque 64-bit IDs exact in JavaScript consumers.
fn id(value: CharacterId) -> Value {
    json!({"world": value.world.to_string(), "id": value.id.to_string()})
}

fn character(value: &Character) -> Value {
    json!({
        "id": id(value.id), "name": value.name,
        "position": value.position.map(|p| json!({"x": p.x, "y": p.y, "z": p.z})),
        "squad": value.squad.as_ref().map(|s| json!({
            "id": s.id.to_string(), "name": s.name, "size": s.size, "leader": s.leader.map(id),
        })),
        "selected": value.selected, "dead": value.dead, "unconscious": value.unconscious,
        "stats": value.stats.map(|s| json!({
            "strength": s.strength, "dexterity": s.dexterity, "toughness": s.toughness,
            "perception": s.perception, "melee_attack": s.melee_attack, "melee_defence": s.melee_defence,
        })),
        "health": value.health.as_ref().map(|h| json!({
            "blood": h.blood, "hunger": h.hunger, "head": h.head, "chest": h.chest,
            "stomach": h.stomach, "left_arm": h.left_arm, "right_arm": h.right_arm,
            "left_leg": h.left_leg, "right_leg": h.right_leg,
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_preserves_opaque_ids_unicode_and_missing_fields() {
        let row = Character {
            id: CharacterId {
                world: u64::MAX,
                id: u64::MAX - 1,
            },
            name: "Beep \"侍\"\n".into(),
            position: None,
            squad: None,
            selected: Some(false),
            dead: None,
            unconscious: None,
            stats: None,
            health: None,
        };
        let text = serde_json::to_string(&character(&row)).unwrap();
        let decoded: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(decoded["id"]["world"], u64::MAX.to_string());
        assert_eq!(decoded["name"], row.name);
        assert_eq!(decoded["selected"], false);
        assert!(decoded["position"].is_null());
        assert!(decoded["health"].is_null());
    }
}
