//! Shared Rust callbacks behind the core's existing verified native detours.
#![forbid(unsafe_code)]

use crate::logging;
use rekenshi_plugin_runtime::{Hook, HookEvents};
use std::sync::OnceLock;

static EVENTS: OnceLock<HookEvents> = OnceLock::new();

pub(crate) fn events() -> HookEvents {
    EVENTS.get_or_init(HookEvents::default).clone()
}

pub(crate) fn ready(ogre: bool, ui_frame: bool) {
    let events = EVENTS.get_or_init(HookEvents::default);
    events.set_available(Hook::OgreCalls, ogre);
    events.set_available(Hook::UiFrame, ui_frame);
}

pub(crate) fn ogre_call() {
    if let Some(events) = EVENTS.get() {
        let failures = events.dispatch_ogre_calls();
        if failures != 0 {
            logging::write(&format!(
                "{failures} Ogre callbacks panicked; affected plugins quarantined; restart Kenshi required"
            ));
        }
    }
}

pub(crate) fn ui_frame(delta_seconds: f32) {
    if let Some(events) = EVENTS.get() {
        let failures = events.dispatch_ui_frame(delta_seconds);
        if failures != 0 {
            logging::write(&format!(
                "{failures} UI frame callbacks panicked; affected plugins quarantined; restart Kenshi required"
            ));
        }
    }
}
