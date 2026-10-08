//! Installation barrier and partial-registration cleanup behavior.
use rekenshi_plugin_api::HostV1;
use rekenshi_plugin_runtime::{Context, Error, Plugin, Plugins, Settings};
use std::sync::{Arc, Mutex};

struct Traced {
    id: &'static str,
    log: Arc<Mutex<Vec<String>>>,
    fail_registration: bool,
}
impl Plugin for Traced {
    fn id(&self) -> &'static str {
        self.id
    }
    fn register(&mut self, _: &Context, _: &Settings) -> Result<(), Error> {
        self.log
            .lock()
            .unwrap()
            .push(format!("register/{}", self.id));
        if self.fail_registration {
            Err("partial registration failed".into())
        } else {
            Ok(())
        }
    }
    fn install(&mut self, _: &Context, _: &Settings) -> Result<(), Error> {
        self.log
            .lock()
            .unwrap()
            .push(format!("install/{}", self.id));
        Ok(())
    }
    fn stop(&mut self) -> Result<(), Error> {
        self.log.lock().unwrap().push(format!("stop/{}", self.id));
        Ok(())
    }
}

#[test]
fn all_desired_registrations_commit_before_provider_installation() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let plugins = Plugins::new()
        .register(Traced {
            id: "provider",
            log: log.clone(),
            fail_registration: false,
        })
        .register(Traced {
            id: "consumer",
            log: log.clone(),
            fail_registration: false,
        });
    let root = std::env::temp_dir().join("rekenshi-installation-barrier-no-files");
    let mut runtime = plugins
        .start(root, |_, token| Ok::<_, Error>(HostV1::stub(token)))
        .unwrap();
    assert_eq!(
        *log.lock().unwrap(),
        [
            "register/provider",
            "register/consumer",
            "install/provider",
            "install/consumer"
        ]
    );
    runtime.stop().unwrap();
    assert!(
        log.lock()
            .unwrap()
            .ends_with(&["stop/consumer".into(), "stop/provider".into()])
    );
}

#[test]
fn failed_desired_registration_is_cleaned_before_any_provider_can_use_it() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let plugins = Plugins::new()
        .register(Traced {
            id: "provider",
            log: log.clone(),
            fail_registration: false,
        })
        .register(Traced {
            id: "consumer",
            log: log.clone(),
            fail_registration: true,
        });
    let root = std::env::temp_dir().join("rekenshi-registration-failure-no-files");
    let mut runtime = plugins
        .start(root, |_, token| Ok::<_, Error>(HostV1::stub(token)))
        .unwrap();
    assert_eq!(
        *log.lock().unwrap(),
        [
            "register/provider",
            "register/consumer",
            "stop/consumer",
            "install/provider"
        ]
    );
    assert_eq!(runtime.active_count(), 1);
    runtime.stop().unwrap();
}
