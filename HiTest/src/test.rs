use super::{ArgValue, Cmd, Condition, InputGroup, ResourceEnv, ThreadInfo};
use log::{debug, error, info, warn};
#[cfg(unix)]
use nix::{sys::wait::waitpid, sys::wait::WaitStatus, unistd::fork, unistd::ForkResult};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
#[cfg(unix)]
use std::process::exit;
use thiserror::Error;

fn default_true() -> bool {
    true
}

fn default_one() -> i64 {
    1
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Test {
    pub name: String,
    pub cmds: Vec<Cmd>,
    #[serde(default = "default_one")]
    pub thread_num: i64,
    #[serde(default)]
    pub should_panic: bool,
    #[serde(default = "default_true")]
    pub break_if_fail: bool,
    #[serde(default)]
    pub inputs: Vec<InputGroup>,
    #[serde(default)]
    pub serial: Option<bool>,
}
pub struct TestResult {
    pub success: usize,
    pub total: usize,
}

#[derive(Debug, Error)]
pub enum TestError {
    #[error("Duplicate argument '{0}' from shared input refs")]
    DuplicateArg(String),

    #[error("Shared input group '{0}' not found")]
    SharedInputNotFound(String),
}

impl Test {
    #[cfg(unix)]
    fn check_panic(mut child_test: Self) -> bool {
        info!(
            "start executing test case {} with panic check.",
            child_test.name
        );
        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                let res_env = ResourceEnv::get_instance().lock().unwrap();
                if let Some(process_env) = &res_env.process_env {
                    process_env.apply_env_init();
                }
                if let Some(thread_env) = &res_env.thread_env {
                    thread_env.apply_env_init();
                }
                child_test.thread_num = 1;
                child_test.should_panic = false;
                if let Some(thread_env) = &res_env.thread_env {
                    thread_env.apply_env_exit();
                }
                if let Some(process_env) = &res_env.process_env {
                    process_env.apply_env_exit();
                }
                let res = child_test.run_one_thread();

                exit(if res { 0 } else { 1 });
            }
            Ok(ForkResult::Parent { child }) => {
                let timeout = std::time::Duration::from_secs(1);
                let start = std::time::Instant::now();

                loop {
                    match waitpid(child, Some(nix::sys::wait::WaitPidFlag::WNOHANG)) {
                        Ok(WaitStatus::StillAlive) => {
                            if start.elapsed() > timeout {
                                let _ = nix::sys::signal::kill(child, nix::sys::signal::SIGKILL);
                                error!(
                                    "Test case {} check panic failed! Child process timeout.",
                                    child_test.name
                                );
                                return false;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            continue;
                        }
                        Ok(status) => match status {
                            WaitStatus::Exited(_, code) => {
                                error!("Test case {} check panic failed! hild process exit as code {} .", child_test.name, code);
                                return false;
                            }
                            WaitStatus::Signaled(_, signal, _) => {
                                info!("Test case {} check panic successfully! crashed with signal {:#?}.", child_test.name, signal);
                                return true;
                            }
                            _ => {
                                error!("Unexpected child status: {:?}", status);
                                return false;
                            }
                        },
                        Err(e) => {
                            error!("Waitpid error: {}", e);
                            return false;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Fork failed: {}", e);
                false
            }
        }
    }

    fn run_one_thread(&self) -> bool {
        let mut cmds: Vec<Cmd> = self.cmds.clone();
        let is_main_thread = ThreadInfo::get_instance().lock().unwrap().is_main_thread();

        if !is_main_thread {
            let res_env = ResourceEnv::get_instance().lock().unwrap();
            let thread_env = res_env.thread_env.as_ref().unwrap();
            info!("start executing test case {} with thread env.", self.name);
            for cmd in thread_env.init.iter().rev() {
                cmds.insert(0, cmd.clone());
            }
            for cmd in &thread_env.exit {
                cmds.push(cmd.clone());
            }
        }

        info!("start executing test case {}.", self.name);
        let mut all_success = true;
        for cmd in cmds {
            match cmd.run() {
                Ok(v) => {
                    if !v {
                        all_success = false;
                        if self.break_if_fail {
                            debug!(
                                "Test case {} stopped because cmd {} failed!",
                                self.name, &cmd.opfunc
                            );
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("execute cmd {} failed! Error: {}\n", &cmd.opfunc, e);
                    return false;
                }
            }
        }
        if all_success {
            info!("Test case {} execute successfully!\n", self.name);
        } else {
            error!("Test case {} execute failed!\n", self.name);
        }

        all_success
    }

    fn process_input_group(&self) -> Vec<Test> {
        if self.inputs.is_empty() {
            return vec![self.clone()];
        }

        let mut expanded_inputs = Vec::new();
        for input in &self.inputs {
            let mut expanded = Self::expand_input_args(input);

            expanded = expanded
                .into_iter()
                .flat_map(|input| {
                    let mut groups = vec![input];
                    while groups.iter().any(|g| {
                        g.args
                            .values()
                            .any(|v| matches!(v, ArgValue::List(_) | ArgValue::Range(_)))
                    }) {
                        groups = groups
                            .into_iter()
                            .flat_map(|g| Self::expand_input_args(&g))
                            .collect();
                    }
                    groups
                })
                .collect();

            expanded_inputs.extend(expanded);
        }

        expanded_inputs
            .into_iter()
            .map(|input| {
                let mut test = self.clone();
                test.inputs = vec![];
                test.break_if_fail = input.break_if_fail.unwrap_or(self.break_if_fail);
                test.should_panic = input.should_panic.unwrap_or(self.should_panic);
                test.name = format!("{}_{}", self.name, input.name);

                let resolved_args: HashMap<String, ArgValue> = input
                    .args
                    .iter()
                    .filter(|(_, v)| matches!(v, ArgValue::Single(_)))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                test.cmds = test
                    .cmds
                    .iter()
                    .map(|cmd| {
                        let condition = match &cmd.condition {
                            Condition::Eq(s) => {
                                let replaced = replace_vars(s.clone(), &resolved_args);
                                if replaced.starts_with("!") {
                                    Condition::Ne(replaced[1..].to_string())
                                } else {
                                    Condition::Eq(replaced)
                                }
                            }
                            Condition::Ne(s) => {
                                Condition::Ne(replace_vars(s.clone(), &resolved_args))
                            }
                        };

                        Cmd {
                            opfunc: cmd.opfunc.clone(),
                            condition,
                            args: cmd
                                .args
                                .iter()
                                .map(|arg| replace_vars(arg.clone(), &resolved_args))
                                .collect(),
                            perf: cmd.perf,
                        }
                    })
                    .collect();
                test
            })
            .collect()
    }

    fn expand_input_args(input: &InputGroup) -> Vec<InputGroup> {
        let mut expanded = Vec::new();
        let current = input.clone();

        for (key, value) in &input.args {
            match value {
                ArgValue::Single(_) => continue,
                ArgValue::List(items) => {
                    for (_, item) in items.iter().enumerate() {
                        let mut new_input = current.clone();
                        new_input
                            .args
                            .insert(key.clone(), ArgValue::Single(item.clone()));
                        new_input.name = format!("{}_{}={}", input.name, &key, &item);
                        expanded.push(new_input);
                    }
                    return expanded;
                }
                ArgValue::Range(range) => {
                    let step = range.step.unwrap_or(1);
                    let mut values = Vec::new();
                    let mut i = range.start;
                    while i <= range.end {
                        values.push(i.to_string());
                        i += step;
                    }

                    for (_, val) in values.iter().enumerate() {
                        let mut new_input = current.clone();
                        new_input
                            .args
                            .insert(key.clone(), ArgValue::Single(val.clone()));
                        new_input.name = format!("{}_{}={}", input.name, key, val);
                        expanded.push(new_input);
                    }
                    return expanded;
                }
            }
        }

        vec![current]
    }

    fn execute(&self) -> bool {
        if self.should_panic {
            #[cfg(unix)]
            {
                let mut child_test = self.clone();
                child_test.should_panic = false;
                Test::check_panic(child_test)
            }
            #[cfg(not(unix))]
            {
                error!("panic check is not supported on this platform.");
                false
            }
        } else {
            self.run_one_thread()
        }
    }
    pub fn run(&self) -> TestResult {
        debug!(
            "start executing test case {}, inputs: {:?}.",
            &self.name, &self.inputs
        );
        let tests = self.process_input_group();
        let tests: Vec<_> = tests
            .into_iter()
            .flat_map(|test| (0..self.thread_num).map(move |_| test.clone()))
            .collect();

        let serial = self.serial.unwrap_or(false);
        let results: Vec<_> = if serial {
            tests.into_iter().map(|test| test.execute()).collect()
        } else {
            let max_thread = {
                let res_env = ResourceEnv::get_instance().lock().unwrap();
                res_env.max_threads
            };
            let results: Vec<_> = if let Some(max_thread) = max_thread {
                warn!("test case {} total sub test cases is {}, but max-threads is {} thread, will be grouped.",
                self.name, tests.len(), max_thread);

                let mut rng = thread_rng();
                let mut shuffled_tests = tests;
                shuffled_tests.shuffle(&mut rng);

                let mut results = Vec::new();
                for chunk in shuffled_tests.chunks(max_thread) {
                    let chunk_results: Vec<_> =
                        chunk.into_par_iter().map(|test| test.execute()).collect();
                    results.extend(chunk_results);
                }
                results
            } else {
                tests.into_par_iter().map(|test| test.execute()).collect()
            };
            results
        };

        let total_count = results.len();
        let success_count = results.iter().filter(|&&x| x).count();
        if total_count != success_count {
            error!(
                "Test {} execute {} sub test failed! {} passed, {} failed!\n",
                self.name,
                total_count,
                success_count,
                total_count - success_count
            );
        }

        TestResult {
            success: success_count,
            total: total_count,
        }
    }

    pub fn push_back(&mut self, cmd: Cmd) {
        self.cmds.push(cmd);
    }

    pub fn push_front(&mut self, cmd: Cmd) {
        self.cmds.insert(0, cmd);
    }

    pub fn resolve_refs(
        &mut self,
        shared_inputs: &HashMap<String, HashMap<String, ArgValue>>,
    ) -> Result<(), TestError> {
        for input in &mut self.inputs {
            let mut all_vars = HashMap::new();
            let mut seen_keys = HashMap::new();
            for ref_name in &input.refs {
                if let Some(ref_group) = shared_inputs.get(ref_name) {
                    for (k, v) in ref_group {
                        if let Some(prev_ref) = seen_keys.insert(k.clone(), ref_name) {
                            return Err(TestError::DuplicateArg(format!(
                                "Parameter '{}' is duplicated in shared_inputs '{}' and '{}'",
                                k, prev_ref, ref_name
                            )));
                        }
                        all_vars.insert(k.clone(), v.clone());
                    }
                } else {
                    return Err(TestError::SharedInputNotFound(ref_name.clone()));
                }
            }

            for ref_name in &input.refs {
                if let Some(ref_group) = shared_inputs.get(ref_name) {
                    for (k, v) in ref_group {
                        if !input.args.contains_key(k) {
                            input.args.insert(k.clone(), v.clone());
                        }
                    }
                }
            }

            for (_, v) in &mut input.args {
                match v {
                    ArgValue::Single(s) => {
                        if s.contains("$") {
                            *v = all_vars.get(&s[1..]).unwrap().clone();
                        }
                    }
                    _ => continue,
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for Test {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Test: {}", self.name)?;
        writeln!(f, "Threads: {}", self.thread_num)?;
        writeln!(f, "Should Panic: {}", self.should_panic)?;
        writeln!(f, "Break if Fail: {}", self.break_if_fail)?;

        if !self.inputs.is_empty() {
            writeln!(f, "Input Groups:")?;
            for input in &self.inputs {
                writeln!(f, "  - {}: {:?}", input.name, input.args)?;
            }
        }

        writeln!(f, "Commands:")?;
        for cmd in &self.cmds {
            writeln!(f, "  - {}", cmd)?;
        }

        Ok(())
    }
}

fn replace_vars(s: String, vars: &HashMap<String, ArgValue>) -> String {
    let mut result = s;
    for (k, v) in vars {
        let value = match v {
            ArgValue::Single(s) => s.clone(),
            ArgValue::List(_) => panic!("List values should be expanded before replace_vars"),
            ArgValue::Range(_) => panic!("Range values should be expanded before replace_vars"),
        };
        result = result.replace(&format!("${}", k), &value);
        result = result.replace(&format!("$!{}", k), &format!("!{}", value));
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{input::RangeExpr, ArgValue};

    #[test]
    fn test_replace_vars() {
        let mut vars = HashMap::new();
        vars.insert("size".into(), ArgValue::Single("100".into()));
        let result = replace_vars("len=$size".into(), &vars);
        assert_eq!(result, "len=100");
    }

    #[test]
    fn test_replace_negated_vars() {
        let mut vars = HashMap::new();
        vars.insert("val".into(), ArgValue::Single("123".into()));
        let result = replace_vars("expect=$!val".into(), &vars);
        assert_eq!(result, "expect=!123");
    }

    #[test]
    fn test_process_input_groups() {
        let test = Test {
            name: "input_test".to_string(),
            cmds: vec![Cmd {
                opfunc: "test_func".to_string(),
                condition: Condition::Eq("$val".to_string()),
                args: vec!["arg=$val".to_string()],
                perf: false,
            }],
            thread_num: 1,
            should_panic: false,
            break_if_fail: true,
            inputs: vec![
                InputGroup {
                    name: "test_input0".to_string(),
                    args: [("val".to_string(), ArgValue::Single("123".to_string()))]
                        .iter()
                        .cloned()
                        .collect(),
                    ..Default::default()
                },
                InputGroup {
                    name: "test_input1".to_string(),
                    args: [("val".to_string(), ArgValue::Single("456".to_string()))]
                        .iter()
                        .cloned()
                        .collect(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let processed = test.process_input_group();
        assert_eq!(processed.len(), 2);
        assert_eq!(processed[0].name, "input_test_test_input0");
        assert_eq!(processed[1].name, "input_test_test_input1");
    }

    #[test]
    fn test_list_input_expansion() {
        let test = Test {
            name: "list_test".to_string(),
            cmds: vec![Cmd {
                opfunc: "test_func".to_string(),
                condition: Condition::Eq("$val".to_string()),
                args: vec!["arg=$val".to_string()],
                perf: false,
            }],
            thread_num: 1,
            should_panic: false,
            break_if_fail: true,
            inputs: vec![InputGroup {
                name: "list_input".to_string(),
                args: [(
                    "val".to_string(),
                    ArgValue::List(vec!["a".into(), "b".into(), "c".into()]),
                )]
                .iter()
                .cloned()
                .collect(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let processed = test.process_input_group();
        assert_eq!(processed.len(), 3);
        assert_eq!(processed[0].name, "list_test_list_input_0");
        assert_eq!(processed[1].name, "list_test_list_input_1");
        assert_eq!(processed[2].name, "list_test_list_input_2");
    }

    #[test]
    fn test_range_input_expansion() {
        let test = Test {
            name: "range_test".to_string(),
            cmds: vec![Cmd {
                opfunc: "test_func".to_string(),
                condition: Condition::Eq("$val".to_string()),
                args: vec!["arg=$val".to_string()],
                perf: false,
            }],
            thread_num: 1,
            should_panic: false,
            break_if_fail: true,
            inputs: vec![InputGroup {
                name: "range_input".to_string(),
                args: [(
                    "val".to_string(),
                    ArgValue::Range(RangeExpr {
                        start: 1,
                        end: 3,
                        step: Some(1),
                    }),
                )]
                .iter()
                .cloned()
                .collect(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let processed = test.process_input_group();
        assert_eq!(processed.len(), 3);
        assert_eq!(processed[0].name, "range_test_range_input_0");
        assert_eq!(processed[1].name, "range_test_range_input_1");
        assert_eq!(processed[2].name, "range_test_range_input_2");
    }

    #[test]
    fn test_same_arg_in_multi_cmd() {
        let test = Test {
            name: "multi_test".to_string(),
            cmds: vec![
                Cmd {
                    opfunc: "test_func1".to_string(),
                    condition: Condition::Eq("$val".to_string()),
                    args: vec!["arg=$val".to_string()],
                    perf: false,
                },
                Cmd {
                    opfunc: "test_func2".to_string(),
                    condition: Condition::Eq("$val".to_string()),
                    args: vec!["arg=$val".to_string()],
                    perf: false,
                },
            ],
            inputs: vec![InputGroup {
                name: "test_input".to_string(),
                args: [(
                    "val".to_string(),
                    ArgValue::List(vec!["100".to_string(), "200".to_string()]),
                )]
                .into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let processed = test.process_input_group();
        assert_eq!(processed.len(), 2);
        assert_eq!(processed[0].name, "multi_test_test_input_0");
        assert_eq!(processed[1].name, "multi_test_test_input_1");
    }
    #[test]
    fn test_resolve_refs() {
        let mut shared_inputs = HashMap::new();
        shared_inputs.insert(
            "group1".to_string(),
            [("size".to_string(), ArgValue::Single("100".to_string()))]
                .iter()
                .cloned()
                .collect(),
        );

        let mut test = Test {
            inputs: vec![InputGroup {
                refs: vec!["group1".to_string()],
                args: [("val".to_string(), ArgValue::Single("$size".to_string()))]
                    .iter()
                    .cloned()
                    .collect(),
                ..Default::default()
            }],
            ..Default::default()
        };

        test.resolve_refs(&shared_inputs).unwrap();
        assert_eq!(test.inputs[0].args.len(), 2);
        assert_eq!(
            test.inputs[0].args.get("size").unwrap(),
            &ArgValue::Single("100".to_string())
        );
        assert_eq!(
            test.inputs[0].args.get("val").unwrap(),
            &ArgValue::Single("100".to_string())
        );
    }
    #[test]
    fn test_resolve_refs_conflict() {
        let mut shared_inputs = HashMap::new();
        shared_inputs.insert(
            "group1".to_string(),
            [("size".to_string(), ArgValue::Single("100".to_string()))]
                .iter()
                .cloned()
                .collect(),
        );
        shared_inputs.insert(
            "group2".to_string(),
            [("size".to_string(), ArgValue::Single("200".to_string()))]
                .iter()
                .cloned()
                .collect(),
        );

        let mut test = Test {
            inputs: vec![InputGroup {
                refs: vec!["group1".to_string(), "group2".to_string()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let result = test.resolve_refs(&shared_inputs);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TestError::DuplicateArg(_)));
    }
}
