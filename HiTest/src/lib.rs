use libparser::{FnAttr, LibParse};
use log::{debug, error, info};
#[cfg(unix)]
use nix::{sys::wait::waitpid, sys::wait::WaitStatus, unistd::fork, unistd::ForkResult};
use rayon::prelude::*;
use serde::{de::Error as DError, Deserialize, Deserializer};
use std::collections::HashMap;
use std::{error::Error, io::Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[cfg(unix)]
use std::process::exit;

mod concurrency;
use concurrency::ConcurrencyGroup;
mod cmd;
use cmd::Cmd;

#[derive(Debug, Deserialize, Clone)]
struct Env {
    name: String,
    init: Cmd,
    exit: Cmd,
    tests: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    concurrences: Option<Vec<ConcurrencyGroup>>,
    #[serde(default)]
    envs: Vec<Env>,
    tests: Vec<Test>,
}

#[derive(Debug, Deserialize, Clone)]
struct Test {
    name: String,
    cmds: Vec<Cmd>,
    #[serde(default = "default_one")]
    thread_num: i64,
    #[serde(default = "default_false")]
    should_panic: bool,
    #[serde(default = "default_true")]
    break_if_fail: bool,
    #[serde(default)]
    inputs: Vec<InputGroup>,
}

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

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn default_one() -> i64 {
    1
}


#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Eq(String),
    Ne(String),
}

impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Value {
            Number(i64),
            String(String),
        }

        #[derive(Deserialize)]
        struct Helper {
            expect_eq: Option<Value>,
            expect_ne: Option<Value>,
        }

        let helper = Helper::deserialize(deserializer)?;

        let to_string = |value: Value| -> String {
            match value {
                Value::Number(n) => n.to_string(),
                Value::String(s) => s,
            }
        };

        match (helper.expect_eq, helper.expect_ne) {
            (Some(eq), None) => Ok(Condition::Eq(to_string(eq))),
            (None, Some(ne)) => Ok(Condition::Ne(to_string(ne))),
            (Some(_), Some(_)) => Err(D::Error::custom("mutually exclusive fields")),
            (None, None) => Err(D::Error::custom(
                "missing condition, please give 'expect_eq' or 'expect_ne'",
            )),
        }
    }
}

pub fn replace_vars(s: String, vars: &HashMap<String, String>) -> String {
    let mut result = s;
    for (k, v) in vars {
        result = result.replace(&format!("${}", k), v);
        result = result.replace(&format!("$!{}", k), &format!("!{}", v));
    }
    result
}

impl Config {
    pub fn run(self) {
        if self.tests.is_empty() {
            info!("no test cases be find, do nothing!");
            return;
        }

        let mut total_tests = 0;
        let mut success_tests = 0;
        let mut failed_tests = 0;

        let tests = self
            .tests
            .into_iter()
            .map(|mut test| {
                for env in &self.envs {
                    if env.tests.contains(&test.name) {
                        test.cmds.insert(0, env.init.clone());
                        test.cmds.push(env.exit.clone());
                        debug!("add env {} to test case {}", env.name, test.name);
                    }
                }
                test
            })
            .collect::<Vec<_>>();

        // run concurrency group
        let mut concurrency_tests: Vec<String> = Vec::new();
        if let Some(concurrences) = self.concurrences {
            info!("Starting run concurrency groups!");
            for mut concurrency in concurrences {
                total_tests += concurrency.len();
                if concurrency.run(&tests) {
                    success_tests += concurrency.success_num();
                } else {
                    failed_tests += concurrency.len() - concurrency.success_num();
                }
                concurrency.record_test(&mut concurrency_tests);
            }
        }

        // filter out concurrency test cases
        let tests = tests
            .into_iter()
            .filter(|test| !concurrency_tests.contains(&test.name))
            .collect::<Vec<_>>();

        // run remaining test cases
        for test in tests {
            info!(
                "Starting run test case: {} with {} thread",
                test.name, test.thread_num
            );

            let test_result = test.run();
            total_tests += 1;
            if test_result {
                success_tests += 1;
            } else {
                failed_tests += 1;
            }

            // reporting test conclusion
            if test_result {
                info!("run test case {} succeeded!\n", test.name);
            } else {
                error!("run test case {} failed!\n", test.name);
            }
        }

        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        if failed_tests == 0 {
            stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Green)))
                .unwrap();
        } else {
            stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Red)))
                .unwrap();
        }
        writeln!(
            stdout,
            "Global Summary: Total tests: {}, Success: {}, Failure: {}",
            total_tests, success_tests, failed_tests
        )
        .unwrap();
        stdout.reset().unwrap();
    }
}

impl Test {
    #[cfg_attr(not(unix), allow(unused_variables), allow(unused_mut))]
    fn check_panic(mut child_test: Self) -> bool {
        #[cfg(unix)]
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
        #[cfg(not(unix))]
        {
            error!(
                "error: panic check only support unix, skip test case {}!",
                child_test.name
            );
            return true;
        }
    }

    fn run_one_thread(&self) -> bool {
        let mut all_success = true;
        let lib_parser = LibParse::get_instance().unwrap().read().unwrap();
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
            info!("Test case {} execute successfully!\n", self.name);
        } else {
            error!("Test case {} execute failed!\n", self.name);
        }

        all_success
    }

    fn run(&self) -> bool {
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

        let tests: Vec<_> = tests
            .into_iter()
            .flat_map(|test| (0..self.thread_num).map(move |_| test.clone()))
            .collect();

        let results: Vec<_> = tests
            .into_par_iter()
            .map(|test| {
                if test.should_panic {
                    let mut child_test = test.clone();
                    child_test.should_panic = false;
                    Test::check_panic(child_test)
                } else {
                    test.run_one_thread()
                }
            })
            .collect();

        let total_count = results.len();
        let success_count = results.iter().filter(|&&x| x).count();
        let failure_count = total_count - success_count;

        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        if failure_count == 0 {
            stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Green)))
                .unwrap();
        } else {
            stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))
                .unwrap();
        }
        writeln!(
            stdout,
            "Test case {} run tests: {}, Success: {}, Failure: {}",
            self.name, total_count, success_count, failure_count
        )
        .unwrap();
        stdout.reset().unwrap();

        success_count == total_count
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_config() {
        let config_content = r#"
        [[tests]]
        name = "test_rw_u32"
        thread_num=100
        cmds = [
            { opfunc = "my_malloc", expect_eq = 0, args = ["len=100", "mem_idx=1"] },
            { opfunc = "my_write32", expect_eq = 0, args = ["mem_idx=1", "offset=0", "val=888"] },
            { opfunc = "my_read32", expect_eq = 888, args = ["mem_idx=1", "offset=0"] },
        ]
        "#;

        let config: Config = toml::from_str(config_content).unwrap();
        assert_eq!(config.tests.len(), 1);
        assert_eq!(config.tests[0].name, "test_rw_u32");
    }

    #[test]
    fn test_parse_config_with_inputs() {
        let config_content = r#"
        [[tests]]
        name = "test_with_inputs"
        thread_num=2
        cmds = [
            { opfunc = "my_malloc", expect_eq = 0, args = ["len=$alloc_size", "mem_idx=1"] },
        ]
        inputs = [
            { name = "input1", args = { alloc_size = "100" } },
            { args = { alloc_size = "200" } },
            { args = { alloc_size = "300" } },
        ]
        "#;

        let config: Config = toml::from_str(config_content).unwrap();
        assert_eq!(config.tests.len(), 1);
        assert_eq!(config.tests[0].inputs.len(), 3);
        assert_eq!(config.tests[0].inputs[0].name, "input1".to_string());
        assert_eq!(config.tests[0].inputs[1].name, "default1".to_string());
        assert_eq!(config.tests[0].inputs[2].name, "default2".to_string());
    }

    #[test]
    fn test_condition_parsing() {
        let cmd_content = r#"
        opfunc = "test_func"
        expect_eq = 0
        args = ["arg1"]
        "#;

        let cmd: Cmd = toml::from_str(cmd_content).unwrap();
        assert!(matches!(cmd.condition, Condition::Eq(_)));

        let cmd_content = r#"
        opfunc = "test_func"
        expect_ne = 1
        args = ["arg1"]
        "#;

        let cmd: Cmd = toml::from_str(cmd_content).unwrap();
        assert!(matches!(cmd.condition, Condition::Ne(_)));
    }

    #[test]
    fn test_replace_vars() {
        let mut vars = HashMap::new();
        vars.insert("var1".to_string(), "value1".to_string());
        vars.insert("var2".to_string(), "value2".to_string());

        let result = replace_vars("test_$var1_$var2".to_string(), &vars);
        assert_eq!(result, "test_value1_value2");
    }

    #[test]
    fn test_parse_config_with_panic() {
        let config_content = r#"
        [[tests]]
        name = "test_write_panic"
        thread_num=100
        should_panic=true
        cmds = [
            { opfunc = "my_malloc", expect_eq = 0, args = ["len=100", "mem_idx=1"] },

        ]"#;
        let config: Config = toml::from_str(config_content).unwrap();
        assert_eq!(config.tests.len(), 1);
        assert_eq!(config.tests[0].name, "test_write_panic");
        assert_eq!(config.tests[0].should_panic, true);
    }

    #[test]
    fn test_parse_config_with_condition() {
        let config_content = r#"
        [[tests]]
        name = "test_write_panic"
        thread_num=100
        should_panic=true
        cmds = [
            { opfunc = "my_malloc", expect_eq = 0, args = ["len=100", "mem_idx=1"] },
            { opfunc = "my_write32", expect_ne = 0, args = ["mem_idx=1", "offset=0", "val=888"] },
            { opfunc = "my_read32", expect_eq = 888, args = ["mem_idx=1", "offset=0"] },
            { opfunc = "my_read32", expect_ne = 888, args = ["mem_idx=1", "offset=0"] },
        ]"#;
        let config: Config = toml::from_str(config_content).unwrap();
        assert_eq!(config.tests.len(), 1);
        assert_eq!(config.tests[0].name, "test_write_panic");
        assert_eq!(config.tests[0].should_panic, true);
        assert_eq!(
            config.tests[0].cmds[0].condition,
            Condition::Eq("0".to_string())
        );
        assert_eq!(
            config.tests[0].cmds[1].condition,
            Condition::Ne("0".to_string())
        );
        assert_eq!(
            config.tests[0].cmds[2].condition,
            Condition::Eq("888".to_string())
        );
        assert_eq!(
            config.tests[0].cmds[3].condition,
            Condition::Ne("888".to_string())
        );
    }
}
