use serde::Deserialize;
use std::collections::HashMap;

fn default_input_name() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    format!("default{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct InputGroup {
    #[serde(default = "default_input_name")]
    pub name: String,
    #[serde(default)]
    pub args: HashMap<String, ArgValue>,
    #[serde(default)]
    pub should_panic: Option<bool>,
    #[serde(default)]
    pub break_if_fail: Option<bool>,
    #[serde(default)]
    pub refs: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum ArgValue {
    Single(String),
    List(Vec<String>),
    Range(RangeExpr),
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct RangeExpr {
    pub start: i32,
    pub end: i32,
    pub step: Option<i32>,
}
