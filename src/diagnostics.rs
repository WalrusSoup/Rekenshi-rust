//! Bounded copied diagnostics. Native rendering remains on MyGUI's UI lane.
#![forbid(unsafe_code)]
use rekenshi_plugin_runtime::{Context, Control, Error, Plugin, Setting, Settings, Value};
use std::collections::VecDeque;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

static ENABLED: AtomicBool = AtomicBool::new(false);
static FROZEN: AtomicBool = AtomicBool::new(false);
static LOGS: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
const SETTINGS: &[Setting] = &[
    Setting {
        key: "debug.overlay",
        label: "Debug overlay",
        description: "Show recent logs, plugin status and observed public operation counts. F11 toggles visibility for this session.",
        default: Value::Toggle(false),
        control: Control::Toggle,
    },
    Setting {
        key: "debug.freeze",
        label: "Freeze debug overlay",
        description: "Hold the displayed snapshot while inspecting it. Logging continues in the background.",
        default: Value::Toggle(false),
        control: Control::Toggle,
    },
];
pub(crate) struct Diagnostics;
impl Plugin for Diagnostics {
    fn id(&self) -> &'static str {
        "developer-diagnostics"
    }
    fn title(&self) -> &'static str {
        "Developer diagnostics"
    }
    fn settings(&self) -> &'static [Setting] {
        SETTINGS
    }
    fn install(&mut self, _: &Context, settings: &Settings) -> Result<(), Error> {
        apply(settings);
        Ok(())
    }
    fn settings_changed(&mut self, _: &Context, settings: &Settings) -> Result<(), Error> {
        apply(settings);
        Ok(())
    }
    fn stop(&mut self) -> Result<(), Error> {
        ENABLED.store(false, Ordering::Relaxed);
        crate::platform::diagnostics_enabled(false);
        Ok(())
    }
}
fn apply(settings: &Settings) {
    let enabled = settings.enabled("debug.overlay");
    FROZEN.store(settings.enabled("debug.freeze"), Ordering::Relaxed);
    crate::platform::diagnostics_enabled(enabled);
    ENABLED.store(enabled, Ordering::Relaxed);
}
pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}
pub(crate) fn toggle() {
    let enabled = !ENABLED.fetch_xor(true, Ordering::Relaxed);
    crate::platform::diagnostics_enabled(enabled);
    crate::logging::write(if enabled {
        "debug overlay: shown with F11"
    } else {
        "debug overlay: hidden with F11"
    });
}
pub(crate) fn frozen() -> bool {
    FROZEN.load(Ordering::Relaxed)
}

fn plain(text: &str, limit: usize) -> String {
    text.chars()
        .map(|c| if c.is_control() || c == '#' { ' ' } else { c })
        .take(limit)
        .collect()
}
pub(crate) fn record(message: &str) {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
        % 86400;
    let message = format!(
        "{:02}:{:02}:{:02} {}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60,
        plain(message, 200)
    );
    if let Ok(mut logs) = LOGS.lock() {
        if logs.len() == 128 {
            logs.pop_front();
        }
        logs.push_back(message);
    }
}
pub(crate) fn snapshot() -> String {
    let mut lines = vec![
        "RE_Kenshi Rust | Developer diagnostics | F11: show/hide".into(),
        format!(
            "Host {} | internal {} | DLLs {} | failed DLLs {} | Ogre calls {}{}",
            crate::STATUS.load(Ordering::Relaxed),
            crate::internal_plugins::active_count(),
            crate::extensions::active_count(),
            crate::extensions::failed_count(),
            crate::CALLS.load(Ordering::Relaxed),
            if frozen() { " | FROZEN" } else { "" }
        ),
    ];
    lines.push(format!(
        "Cache hits: shader {} | collider {} | sidecar {} | terrain reads {}",
        crate::rekenshi_shader_cache_hits(),
        crate::rekenshi_collider_cache_hits(),
        crate::rekenshi_texture_sidecar_hits(),
        crate::rekenshi_heightmap_cache_reads(),
    ));
    lines.extend(crate::platform::diagnostics());
    lines.push(rekenshi_graphics::landscape_diagnostics());
    lines.push("Recent logs (UTC) | counts accumulate while overlay is enabled".into());
    if let Ok(logs) = LOGS.try_lock() {
        lines.extend(logs.iter().rev().take(12).rev().cloned());
    }
    lines
        .into_iter()
        .map(|line| plain(&line, 91))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_caption_text_is_bounded_and_cannot_inject_colour_or_control_codes() {
        assert_eq!(plain("#FF0000bad\0\ntexté", 20), " FF0000bad  texté");
        assert_eq!(plain("é".repeat(201).as_str(), 200).chars().count(), 200);
    }
}
