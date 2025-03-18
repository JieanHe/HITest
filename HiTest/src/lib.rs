use libparser::{FnAttr, LibParse};
use log::{debug, error, info};
#[cfg(unix)]
use nix::{sys::wait::waitpid, sys::wait::WaitStatus, unistd::fork, unistd::ForkResult};

use rayon::prelude::*;
use serde::Deserialize;
use std::error::Error;
#[cfg(unix)]
use std::process::exit;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    concurrences: Option<Vec<ConcurrencyGroup>>,

    tests: Vec<Test>,
}

#[derive(Debug, Deserialize)]
struct ConcurrencyGroup {
    tests: Vec<String>,
    serial: bool,
    #[serde(default = "default_name")]
    name: String,
}

#[derive(Debug, Deserialize, Clone)]
struct Test {
    name: String,
    cmds: Vec<Cmd>,
    #[serde(default = "default_one")]
    thread_num: i32,
    #[serde(default = "default_false")]
    should_panic: bool,
}

fn default_false() -> bool {
    false
}

fn default_one() -> i32 {
    1
}

fn default_name() -> String {
    String::from("default_group")
}

#[derive(Debug, Deserialize, Clone)]
struct Cmd {
    opfunc: String,
    expect_res: i32,
    args: Vec<String>,
}

impl Config {
    pub fn run(self, lib_parser: &LibParse) {
        if self.tests.is_empty() {
            info!("no test cases be find, do nothing!");
            return;
        }

        // run concurrency group
        if let Some(concurrences) = self.concurrences {
            info!("Starting run concurrency groups!");
            for concurrency in concurrences {
                if concurrency.run(lib_parser, &self.tests) {
                    info!("run concurrency group {} succeeded!\n", concurrency.name);
                }
            }
        }
        // run test cases
        for test in self.tests {
            info!(
                "Starting run test case: {} with {} thread",
                test.name, test.thread_num
            );

            // reporting test conclusion
            if test.run(lib_parser) {
                info!("run test case {} succeeded!\n", test.name);
            } else {
                error!("run test case {} failed!\n", test.name);
            }
        }
    }
}

impl ConcurrencyGroup {
    pub fn run(&self, lib_parser: &LibParse, tests: &Vec<Test>) -> bool {
        if self.tests.is_empty() {
            return true;
        }

        let mut test_cases: Vec<Test> = Vec::new();
        for test in tests {
            if self.tests.contains(&test.name) {
                test_cases.push(test.clone());
            }
        }

        if test_cases.is_empty() {
            return true;
        }
        debug!(
            "Concurrency Group {} Contains test cases: {:#?}, serial option: {}",
            self.name, self.tests, self.serial
        );
        let mut succ: bool = true;
        if self.serial {
            for test in test_cases {
                if test.run(lib_parser) {
                    error!(
                        "test case {} in concurrency {} serially execute failed!",
                        test.name, self.name
                    );
                    return false;
                }
            }
            info!(
                "Serial execute concurrency Group {} with {} test cases, all passed!",
                self.name,
                self.tests.len()
            );
        } else {
            // parallel with other test cases ignore source thread num
            test_cases.par_iter_mut().for_each(|test| {
                test.thread_num = 1;
            });
            let results: Vec<_> = test_cases
                .into_par_iter()
                .map(|test| test.run(lib_parser))
                .collect();

            let count = results.into_iter().filter(|&x| x).count();
            debug!(
                "Parallel execute concurrency Group {} with {} thread, {} passed!",
                self.name,
                self.tests.len(),
                count
            );

            succ = count as usize == self.tests.len();
            if succ {
                info!(
                    "Parallel execute concurrency Group {} with {} thread, all passed!",
                    self.name,
                    self.tests.len()
                );
            } else {
                error!(
                    "Parallel execute concurrency Group {} with {} thread, {} passed!",
                    self.name,
                    self.tests.len(),
                    count
                );
            }
        }
        return succ;
    }
}

impl Test {

    #[cfg_attr(not(unix), allow(unused_variables), allow(unused_mut))]
    fn check_panic(mut child_test: Self, lib_parser: &LibParse) -> bool {
        #[cfg(unix)]
        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                child_test.thread_num = 1;
                child_test.run_one_thread(lib_parser);
                exit(0);
            }
            Ok(ForkResult::Parent { child }) => {
                let status = waitpid(child, None).expect("等待子进程失败");

                // 模式匹配处理退出状态
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
        #[cfg(not(unix))]
        {
            error!(
                "error: panic check only support unix, skip test case {}!",
                child_test.name
            );
            return true;
        }

    }

    fn run_one_thread(&self, lib_parser: &LibParse) -> bool {
        let mut res = true;
        for cmd in self.cmds.clone() {
            let fn_attr = match lib_parser.get_func(&cmd.opfunc) {
                Ok(v) => v,
                Err(e) => {
                    error!("execute cmd {} failed! Error: {}\n", &cmd.opfunc, e);
                    return false;
                }
            };
            let paras = match fn_attr.parse_params(&cmd.args) {
                Ok(v) => v,
                Err(e) => {
                    error!("execute cmd {} failed! Error: {}\n", &cmd.opfunc, e);
                    return false;
                }
            };

            match cmd.run(&lib_parser, &fn_attr, &paras) {
                Ok(v) => {
                    if !v {
                        res = false;
                    }
                }
                Err(e) => {
                    error!("execute cmd {} failed! Error: {}\n", &cmd.opfunc, e);
                    return false;
                }
            }
        }

        res
    }

    fn run(&self, lib_parser: &LibParse) -> bool {
        if self.should_panic {
            let child_test = self.clone();
            return Test::check_panic(child_test, lib_parser);
        }

        if self.thread_num == 1 {
            return self.run_one_thread(lib_parser);
        }

        let results: Vec<_> = (0..self.thread_num)
            .into_par_iter() // rayon parallel
            .map(|_| self.run_one_thread(lib_parser))
            .collect();
        debug!("results: {:#?}", results);
        let count = results.into_iter().filter(|&x| x).count();
        debug!(
            "run test case {} with {} thread, {} passed!",
            self.name, self.thread_num, count
        );

        let succ = count as i32 == self.thread_num;
        if succ {
            info!(
                "run test case {} with {} thread, all passed!",
                self.name, self.thread_num
            );
        } else {
            error!(
                "run test case {} with {} thread, {} passed!",
                self.name, self.thread_num, count
            );
        }
        succ
    }
}

impl Cmd {
    pub fn run(
        &self,
        lib_parser: &LibParse,
        fn_attr: &FnAttr,
        paras: &Vec<u64>,
    ) -> Result<bool, Box<dyn Error>> {
        let ret = match lib_parser.call_func_attr(fn_attr, paras) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };
        if ret != self.expect_res {
            error!(
                "execute cmd: {}{:?}, expect return: {}, actual: {}",
                self.opfunc, self.args, self.expect_res, ret
            );
        } else {
            debug!(
                "execute cmd: {}{:?} succeeded, expect return: {}, actual: {}",
                self.opfunc, self.args, self.expect_res, ret
            );
        }

        Ok(ret == self.expect_res)
    }
}
