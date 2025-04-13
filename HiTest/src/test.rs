use super::{Cmd, Condition};
use log::{error, info};
#[cfg(unix)]
use nix::{sys::wait::waitpid, sys::wait::WaitStatus, unistd::fork, unistd::ForkResult};
#[cfg(unix)]
use std::process::exit;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;
use std::collections::HashMap;

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

fn replace_vars(s: String, vars: &HashMap<String, String>) -> String {
    let mut result = s;
    for (k, v) in vars {
        result = result.replace(&format!("${}", k), v);
        result = result.replace(&format!("$!{}", k), &format!("!{}", v));
    }
    result
}
