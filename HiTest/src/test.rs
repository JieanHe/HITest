use super::{Cmd, Condition};
use log::{error, debug, info};
#[cfg(unix)]
use nix::{sys::wait::waitpid, sys::wait::WaitStatus, unistd::fork, unistd::ForkResult};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;
use std::collections::HashMap;
#[cfg(unix)]
use std::process::exit;
use std::fmt;

fn default_input_name() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    format!("default{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}
#[derive(Debug, Deserialize, Clone)]
struct InputGroup {
    #[serde(default = "default_input_name")]
    name: String,
    args: HashMap<String, String>,
    should_panic: Option<bool>,
    break_if_fail: Option<bool>,
}

fn default_true() -> bool {
    true
}

fn default_one() -> i64 {
    1
}

#[derive(Debug, Deserialize, Clone)]
pub struct Test {
    pub name: String,
    cmds: Vec<Cmd>,
    #[serde(default = "default_one")]
    thread_num: i64,
    #[serde(default)]
    should_panic: bool,
    #[serde(default = "default_true")]
    break_if_fail: bool,
    #[serde(default)]
    inputs: Vec<InputGroup>,
}

impl Test {
    #[cfg(unix)]
    fn check_panic(mut child_test: Self) -> bool {
        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                child_test.thread_num = 1;
                child_test.should_panic = false;
                let _res = child_test.run_one_thread();
                exit(0);
            }
            Ok(ForkResult::Parent { child }) => {
                let status = waitpid(child, None).expect("Waiting for child failed");

                match status {
                    WaitStatus::Exited(_, code) => {
                        error!("child process exit as code {}.", code);
                        return false;
                    }
                    WaitStatus::Signaled(_, signal, _) => {
                        info!("child process been terminate with signal {:#?}.", signal);
                        return true;
                    }
                    _ => {
                        error!(
                            "child process unexpectedly exited with status: {:?}",
                            status
                        );
                        return true;
                    }
                }
            }
            Err(e) => {
                error!("start child failed with error: {:?}", e);
                return false;
            }
        }
    }

    pub fn will_panic(&mut self) {
        self.should_panic = true;
    }

    fn run_one_thread(&self) -> bool {
        let mut all_success = true;
        for cmd in self.cmds.clone() {
            match cmd.run() {
                Ok(v) => {
                    if !v {
                        all_success = false;
                        if self.break_if_fail {
                            error!(
                                "Test case {} stopped because cmd {} failed!\n",
                                self.name, &cmd.opfunc
                            );
                            return false;
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
            info!("Test case {} execute successfully!", self.name);
        } else {
            error!("Test case {} execute failed!", self.name);
        }

        all_success
    }

    fn process_input_group(&self) -> Vec<Test> {
        let tests = if self.inputs.is_empty() {
            vec![self.clone()]
        } else {
            self.inputs
                .iter()
                .map(|input| {
                    let mut test = self.clone();
                    test.inputs = vec![];
                    test.break_if_fail = input.break_if_fail.unwrap_or(self.break_if_fail);
                    test.should_panic = input.should_panic.unwrap_or(self.should_panic);
                    test.name = format!("{}_{}", self.name, input.name);
                    test.cmds = test
                        .cmds
                        .iter()
                        .map(|cmd| {
                            let resolved_args = cmd
                                .args
                                .iter()
                                .map(|arg| replace_vars(arg.clone(), &input.args))
                                .collect();

                            let condition = match &cmd.condition {
                                Condition::Eq(s) => {
                                    let replaced = replace_vars(s.clone(), &input.args);
                                    if replaced.starts_with("!") {
                                        Condition::Ne(replaced[1..].to_string())
                                    } else {
                                        Condition::Eq(replaced)
                                    }
                                }
                                Condition::Ne(s) => {
                                    Condition::Ne(replace_vars(s.clone(), &input.args))
                                }
                            };
                            Cmd {
                                opfunc: cmd.opfunc.clone(),
                                condition,
                                args: resolved_args,
                                perf: cmd.perf,
                            }
                        })
                        .collect();

                    test
                })
                .collect()
        };
        tests
    }

    pub fn run(&self) -> (usize, usize) {
        debug!("start executing test case {}.",  self);
        let tests = self.process_input_group();
        let tests: Vec<_> = tests
            .into_iter()
            .flat_map(|test| (0..self.thread_num).map(move |_| test.clone()))
            .collect();

        let results: Vec<_> = tests
            .into_par_iter()
            .map(|test| {
                if test.should_panic {
                    #[cfg(unix)]
                    {
                        let mut child_test = test.clone();
                        child_test.should_panic = false;
                        Test::check_panic(child_test)
                    }
                    #[cfg(not(unix))]
                    {
                        error!("panic check is not supported on this platform.");
                        false
                    }
                } else {
                    test.run_one_thread()
                }
            })
            .collect();

        let total_count = results.len();
        let success_count = results.iter().filter(|&&x| x).count();
        if total_count != success_count {
            error!(
                "Test {} execute failed! {} passed, {} failed!\n",
                self.name,
                success_count,
                total_count - success_count
            );
        }
        (success_count, total_count)
    }

    pub fn push_back(&mut self, cmd: Cmd) {
        self.cmds.push(cmd);
    }

    pub fn push_front(&mut self, cmd: Cmd) {
        self.cmds.insert(0, cmd);
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

fn replace_vars(s: String, vars: &HashMap<String, String>) -> String {
    let mut result = s;
    for (k, v) in vars {
        result = result.replace(&format!("${}", k), v);
        result = result.replace(&format!("$!{}", k), &format!("!{}", v));
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_replace_vars() {
        let mut vars = HashMap::new();
        vars.insert("size".into(), "100".into());
        let result = replace_vars("len=$size".into(), &vars);
        assert_eq!(result, "len=100");
    }

    #[test]
    fn test_replace_negated_vars() {
        let mut vars = HashMap::new();
        vars.insert("val".into(), "123".into());
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
                    args: [("val".to_string(), "123".to_string())]
                        .iter()
                        .cloned()
                        .collect(),
                    should_panic: None,
                    break_if_fail: None,
                },
                InputGroup {
                    name: "test_input1".to_string(),
                    args: [("val".to_string(), "456".to_string())]
                        .iter()
                        .cloned()
                        .collect(),
                    should_panic: None,
                    break_if_fail: None,
                },
            ],
        };
        let processed = test.process_input_group();
        assert_eq!(processed.len(), 2);
        assert_eq!(processed[0].name, "input_test_test_input0");
        assert_eq!(processed[1].name, "input_test_test_input1");
    }


}
