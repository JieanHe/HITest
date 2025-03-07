use libparser::{FnAttr, LibParse};
use log::{debug, error, info};
use rayon::prelude::*;
use serde::Deserialize;
use std::{error::Error, os::raw::c_long};

#[derive(Debug, Deserialize)]
pub struct Config {
    tests: Vec<Test>,
}

#[derive(Debug, Deserialize)]
struct Test {
    name: String,
    cmds: Vec<Cmd>,
    #[serde(default = "default_one")]
    thread_num: i32,
}

fn default_one() -> i32 {
    1
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

impl Test {
    fn run_one_thread(&self, lib_parser: &LibParse) -> bool {
        for cmd in self.cmds.clone() {
            let fn_attr = match lib_parser.get_func(&cmd.opfunc) {
                Ok(v) => v,
                Err(e) => {
                    error!("execute cmd {} failed! Error:{:?}\n", &cmd.opfunc, e);
                    return false;
                }
            };
            let paras = match fn_attr.parse_params(&cmd.args) {
                Ok(v) => v,
                Err(e) => {
                    error!("execute cmd {} failed! Error:{:?}\n", &cmd.opfunc, e);
                    return false;
                }
            };
            if let Err(e) = cmd.run(&lib_parser, &fn_attr, &paras) {
                error!("execute cmd {} failed! Error:{:?}\n", &cmd.opfunc, e);
                return false;
            }
        }

        return true;
    }

    fn run(&self, lib_parser: &LibParse) -> bool {
        if self.thread_num == 1 {
            return self.run_one_thread(lib_parser);
        }

        let results: Vec<_> = (0..self.thread_num)
            .into_par_iter() // rayon parallel
            .map(|_| self.run_one_thread(lib_parser))
            .collect();

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
        paras: &Vec<c_long>,
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
