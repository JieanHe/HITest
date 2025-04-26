use serde::Deserialize;
use std::collections::HashMap;

fn default_input_name() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    format!("default{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[derive(Debug, Deserialize, Clone)]
pub struct InputGroup {
    #[serde(default = "default_input_name")]
    pub name: String,
    #[serde(default)]
    pub args: HashMap<String, String>,
    #[serde(default)]
    pub should_panic: Option<bool>,
    #[serde(default)]
    pub break_if_fail: Option<bool>,
}
