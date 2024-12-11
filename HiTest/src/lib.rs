use libparser::LibParse;
use log::{debug, error, info};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    tests: Vec<Test>,
}

#[derive(Debug, Deserialize)]
struct Test {
    name: String,
    cmds: Vec<Cmd>,
}

#[derive(Debug, Deserialize)]
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
            info!("Starting run test case: {}", test.name);
            let mut succ = true;
            for cmd in test.cmds {
                let ret: i32 = match lib_parser.call_func(&cmd.opfunc, &cmd.args) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("run {} failed,Error:\n{:?}", test.name, e);
                        break;
                    }
                };
                if ret != cmd.expect_res {
                    error!(
                        "run cmd {} Failed: [expect_res={}, res={}]",
                        cmd.opfunc, cmd.expect_res, ret
                    );
                    succ = false;
                    break;
                }
                debug!(
                    "Executing cmd: {}{:?}, [expect_res={}, res={}]",
                    cmd.opfunc, cmd.args, cmd.expect_res, ret
                );
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
