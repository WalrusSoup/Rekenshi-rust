use crate::abi::{HostV1, OK, services::*};
use rekenshi_plugin_runtime::{Hook, HookEvents, Host};
use std::{
    ffi::c_void,
    path::PathBuf,
    sync::{Arc, Mutex},
};
pub(crate) struct Connection {
    api: ServicesV1,
    host: HostV1,
    events: HookEvents,
    subscriptions: Mutex<Vec<(u64, Box<Dispatch>)>>,
}
struct Dispatch {
    host: HostV1,
    events: HookEvents,
    kind: u32,
}
impl Connection {
    pub(crate) fn new(host: HostV1) -> Result<Arc<Self>, crate::Error> {
        let address = host
            .find_export(c"RE_Kenshi_Rust.dll", c"rekenshi_services_api")
            .ok_or("SDK service unavailable; update host")?;
        // SAFETY: Named versioned export has GetApi signature and is pinned.
        let get: GetApi = unsafe { std::mem::transmute(address.as_ptr()) };
        let mut output = std::mem::MaybeUninit::uninit();
        // SAFETY: Aligned writable output with exact service version/size.
        if unsafe { get(VERSION, output.as_mut_ptr(), size_of::<ServicesV1>()) } != OK {
            return Err("SDK service negotiation failed".into());
        }
        // SAFETY: A successful negotiation initialized the whole table.
        let api: ServicesV1 = unsafe { output.assume_init() };
        if api.abi_version != VERSION || api.struct_size as usize != size_of::<ServicesV1>() {
            return Err("incompatible SDK service".into());
        }
        let events = HookEvents::default();
        events.set_available(Hook::OgreCalls, (api.supports)(OGRE_CALLS));
        events.set_available(Hook::UiFrame, (api.supports)(UI_FRAME));
        Ok(Arc::new(Self {
            api,
            host,
            events,
            subscriptions: Mutex::default(),
        }))
    }
    pub(crate) fn root(&self) -> Result<PathBuf, crate::Error> {
        let mut output = vec![0; 32768];
        let mut len = 0;
        // SAFETY: Host writes into this bounded output and length synchronously.
        if unsafe {
            (self.api.root)(
                self.host.plugin_token,
                output.as_mut_ptr(),
                output.len(),
                &mut len,
            )
        } != OK
            || len > output.len()
        {
            return Err("host settings folder unavailable".into());
        }
        output.truncate(len);
        String::from_utf8(output)
            .map(PathBuf::from)
            .map_err(crate::Error::from)
    }
    pub(crate) fn requires(&self, id: &str) -> Result<(), crate::Error> {
        // SAFETY: This UTF-8 input remains readable for the synchronous call.
        if unsafe { (self.api.dependency)(self.host.plugin_token, id.as_ptr(), id.len()) } == OK {
            Ok(())
        } else {
            Err(format!("requires unavailable host plugin {id}").into())
        }
    }
    pub(crate) fn host(self: &Arc<Self>) -> Host {
        let connection = self.clone();
        Host::new(self.host, self.events.clone())
            .with_connector(move |hook| connection.connect(hook))
    }
    fn connect(&self, hook: Hook) -> Result<(), crate::Error> {
        let kind = match hook {
            Hook::OgreCalls => OGRE_CALLS,
            Hook::UiFrame => UI_FRAME,
        };
        let mut subscriptions = self
            .subscriptions
            .lock()
            .map_err(|_| "callback transport poisoned")?;
        if subscriptions.iter().any(|(_, d)| d.kind == kind) {
            return Ok(());
        }
        let mut dispatch = Box::new(Dispatch {
            host: self.host,
            events: self.events.clone(),
            kind,
        });
        let mut handle = 0;
        // SAFETY: Dispatch is heap-stable and remains owned until unsubscribe
        // quiesces its calls. The trampoline contains Rust panics.
        let result = unsafe {
            (self.api.subscribe)(
                self.host.plugin_token,
                kind,
                dispatch_event,
                (&mut *dispatch as *mut Dispatch).cast(),
                &mut handle,
            )
        };
        if result != OK {
            return Err(format!("host callback unavailable ({result})").into());
        }
        subscriptions.push((handle, dispatch));
        Ok(())
    }
    pub(crate) fn close(&self) -> Result<(), crate::Error> {
        let mut subscriptions = self
            .subscriptions
            .lock()
            .map_err(|_| "callback transport poisoned")?;
        while let Some((handle, _)) = subscriptions.last() {
            if (self.api.unsubscribe)(self.host.plugin_token, *handle) != OK {
                return Err("callback transport did not quiesce; restart required".into());
            }
            subscriptions.pop();
        }
        Ok(())
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        if self.close().is_err() {
            // Retained foreign callbacks may still reference Dispatch; retain
            // its allocation too. Host pins the DLL for the process lifetime.
            let subscriptions = self
                .subscriptions
                .get_mut()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for (_, dispatch) in subscriptions.drain(..) {
                Box::leak(dispatch);
            }
        }
    }
}
unsafe extern "C" fn dispatch_event(pointer: *mut c_void, delta: f32) {
    let _ = crate::dll::boundary(|| {
        if pointer.is_null() {
            return crate::abi::ERROR;
        }
        // SAFETY: Connection owns Dispatch until host unsubscribe quiesces calls.
        let dispatch = unsafe { &*pointer.cast::<Dispatch>() };
        let failures = if dispatch.kind == OGRE_CALLS {
            dispatch.events.dispatch_ogre_calls()
        } else {
            dispatch.events.dispatch_ui_frame(delta)
        };
        if failures != 0 {
            dispatch.host.write_log(
                "plugin callback panicked; callback group quarantined; restart Kenshi required",
            );
        }
        OK
    });
}
