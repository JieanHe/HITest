use super::Cmd;
use log::warn;
use serde::Deserialize;
use std::sync::{RwLock, Once};

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
    pub max_threads: Option<usize>,
}
static mut INSTANCE: Option<RwLock<ResourceEnv>> = None;
static INIT: Once = Once::new();

impl ResourceEnv {
    pub fn get_instance() -> &'static RwLock<ResourceEnv> {
        #[cfg_attr(unix, allow(static_mut_refs))]
        unsafe {
            INSTANCE.as_ref().unwrap()
        }
    }

    pub fn init(thread_env: Option<Env>, process_env: Option<Env>, max_threads: Option<usize>) {
        INIT.call_once(|| unsafe {
            INSTANCE = Some(RwLock::new(ResourceEnv {
                thread_env,
                process_env,
                max_threads,
            }));
        });
    }
}
