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
}

fn default_one() -> i32 {
    1
}

#[derive(Debug, Deserialize)]
struct Cmd {
    opfunc: String,
    expect_res: i32,
    args: Vec<String>,
    #[serde(default = "default_one")]
    thread_num: i32,
}

fn handle_error(err: &Box<dyn Error>, succ: &mut bool, function_name: &str) {
    *succ = false;
    error!(
        "Error:{:?}\n execute function {} failed!",
        err, function_name
    );
}

impl Config {
    pub fn run(self, lib_parser: &LibParse) {
        if self.tests.is_empty() {
            info!("no test cases be find, do nothing!");
            return;
        }

        // run test cases
        for test in self.tests {
            info!("Starting run test case: {}", test.name);
            let mut succ = true;
            for cmd in test.cmds {
                let fn_attr = match lib_parser.get_func(&cmd.opfunc) {
                    Ok(v) => v,
                    Err(e) => {
                        handle_error(&e, &mut succ, &cmd.opfunc);
                        break;
                    }
                };
                let paras = match fn_attr.parse_params(&cmd.args) {
                    Ok(v) => v,
                    Err(e) => {
                        handle_error(&e, &mut succ, &cmd.opfunc);
                        break;
                    }
                };
                if let Err(e) = cmd.run(&lib_parser, &fn_attr, &paras) {
                    handle_error(&e, &mut succ, &cmd.opfunc);
                    break;
                }
            }

            // reporting test conclusion
            if succ {
                info!("run test case {} successed!\n", test.name);
            } else {
                error!("run test case {} failed!\n", test.name);
            }
        }
    }
}

impl Cmd {
    pub fn run(
        &self,
        lib_parser: &LibParse,
        fn_attr: &FnAttr,
        paras: &Vec<c_long>,
    ) -> Result<(), Box<dyn Error>> {
        if self.thread_num == 1 {
            let ret: i32 = lib_parser.call_func_attr(&fn_attr, &paras)?;
            if ret != self.expect_res {
                error!(
                    "excute {}{:?} failed, expect res is {} but got {}!",
                    self.opfunc, self.args, self.expect_res, ret
                );
            }
            debug!(
                "Executing cmd: {}{:?}, [expect_res={}, res={}]",
                self.opfunc, self.args, self.expect_res, ret
            );
        } else {
            let results: Vec<_> = (0..self.thread_num)
                .into_par_iter() // rayon parallel
                .map(|_| {
                    let params = paras.clone();
                    let ret = match lib_parser.call_func_attr(fn_attr, &params) {
                        Ok(r) => r,
                        Err(_) => self.expect_res - 1,
                    };
                    ret != self.expect_res
                })
                .collect();

            // calculate failed thread number
            let count = results.into_iter().filter(|&x| x).count();
            if count != 0 {
                error!(
                    "excute cmd: {}{:?} with {} thread, {} thread run failed!",
                    self.opfunc, self.args, self.thread_num, count
                );
            } else {
                debug!(
                    "excute cmd: {}{:?} with {} thread, {} thread passed!",
                    self.opfunc, self.args, self.thread_num, self.thread_num
                );
            }
        }

        Ok(())
    }
}
