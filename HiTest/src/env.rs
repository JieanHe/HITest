use super::Cmd;
use log::warn;
use serde::Deserialize;
use std::sync::{Mutex, Once};

#[derive(Debug, Deserialize, Clone)]
pub struct Env {
    pub name: String,
    pub init: Vec<Cmd>,
    pub exit: Vec<Cmd>,
    #[serde(default)]
    pub tests: Vec<String>,
}

impl Env {
    pub fn apply_env_init(&self) {
        for cmd in self.init.iter().cloned() {
            if let Err(e) = cmd.run() {
                warn!(
                    "failed to run thread env init command: {} {}",
                    cmd.opfunc, e
                );
            }
        }
    }

    pub fn apply_env_exit(&self) {
        for cmd in self.exit.iter().cloned() {
            if let Err(e) = cmd.run() {
                warn!(
                    "failed to run thread env init command:  {} {}",
                    cmd.opfunc, e
                );
            }
        }
    }
}

pub struct ResourceEnv {
    pub thread_env: Option<Env>,
    pub process_env: Option<Env>,
}
static mut INSTANCE: Option<Mutex<ResourceEnv>> = None;
static INIT: Once = Once::new();

impl ResourceEnv {
    pub fn get_instance() -> &'static Mutex<ResourceEnv> {
        #[cfg_attr(unix, allow(static_mut_refs))]
        unsafe { INSTANCE.as_ref().unwrap() }
    }

    pub fn init(thread_env: Option<Env>, process_env: Option<Env>) {
        INIT.call_once(|| unsafe {
            INSTANCE = Some(Mutex::new(ResourceEnv {
                thread_env,
                process_env,
            }));
        });
    }
}
