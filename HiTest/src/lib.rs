use libparser::{FnAttr, LibParse};
use log::{debug, error, info};
#[cfg(unix)]
use nix::{libc, sys::wait::waitpid, sys::wait::WaitStatus, unistd::fork, unistd::ForkResult};

use rayon::prelude::*;
use serde::{de::Error as DError, Deserialize, Deserializer};
use std::collections::HashMap;
use std::error::Error;
#[cfg(unix)]
use std::process::exit;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    concurrences: Option<Vec<ConcurrencyGroup>>,

    tests: Vec<Test>,
}

#[derive(Debug, Deserialize, Clone)]
struct ConcurrencyGroup {
    tests: Vec<String>,
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

fn default_one() -> i32 {
    1
}

fn default_name() -> String {
    String::from("default_group")
}

#[derive(Debug, Deserialize, Clone)]
pub struct Cmd {
    pub opfunc: String,
    #[serde(flatten)]
    pub condition: Condition,
    pub args: Vec<String>,
    #[serde(default)]
    pub perf: Option<bool>,
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
            Number(i32),
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
            "Concurrency Group {} Contains test cases: {:#?}",
            self.name, self.tests
        );

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

        let succ = count as usize == self.tests.len();
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
                child_test.should_panic = false;
                let res = child_test.run_one_thread(lib_parser);
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

    fn run_one_thread(&self, lib_parser: &LibParse) -> bool {
        let mut all_success = true;

        let input_groups: Vec<(String, HashMap<String, String>, Option<bool>, Option<bool>)> =
            if self.inputs.is_empty() {
                vec![(
                    "default".to_string(),
                    HashMap::new(),
                    Some(false),
                    Some(false),
                )]
            } else {
                self.inputs
                   .iter()
                   .map(|ig| {
                        (
                            ig.name.clone(),
                            ig.args.clone(),
                            ig.should_panic,
                            ig.break_if_fail,
                        )
                    })
                   .collect()
            };

        for (input_name, input, should_panic_opt, break_if_fail_opt) in input_groups {
            let should_panic = should_panic_opt.unwrap_or(false);
            let break_if_fail = break_if_fail_opt.unwrap_or(self.break_if_fail);

            let mut case_success = true;
            info!(
                "Running test case '{}' with input group: {}",
                self.name,
                input_name
            );

            if should_panic {
                // 复制 Test 实例
                let mut new_test = self.clone();
                new_test.should_panic = true;

                // 替换 cmd 参数
                new_test.cmds = new_test.cmds.iter().map(|cmd| {
                    let resolved_args = cmd
                       .args
                       .iter()
                       .map(|arg| {
                            input.iter().fold(arg.clone(), |acc, (k, v)| {
                                acc.replace(&format!("${}", k), v)
                            })
                        })
                       .collect();
                    Cmd {
                        opfunc: cmd.opfunc.clone(),
                        condition: cmd.condition.clone(),
                        args: resolved_args,
                        perf: cmd.perf,
                    }
                }).collect();

                case_success = Test::check_panic(new_test, lib_parser);
            } else {
                for cmd in self.cmds.clone() {
                    let uses_input_args = cmd.args.iter().any(|arg| arg.contains('$'));
                    let resolved_args = cmd
                       .args
                       .iter()
                       .map(|arg| {
                            input.iter().fold(arg.clone(), |acc, (k, v)| {
                                acc.replace(&format!("${}", k), v)
                            })
                        })
                       .collect::<Vec<_>>();

                    let fn_attr = match lib_parser.get_func(&cmd.opfunc) {
                        Ok(v) => v,
                        Err(e) => {
                            error!("execute cmd {} failed! Error: {}", &cmd.opfunc, e);
                            if uses_input_args {
                                error!("Input group '{}' args: {:?}", input_name, input);
                            }
                            if break_if_fail {
                                case_success = false;
                                break;
                            }
                            case_success = false;
                            continue;
                        }
                    };
                    let paras = match fn_attr.parse_params(&resolved_args) {
                        Ok(v) => v,
                        Err(e) => {
                            error!("execute cmd {} failed! Error: {}", &cmd.opfunc, e);
                            if uses_input_args {
                                error!("Input group '{}' args: {:?}", input_name, input);
                            }
                            if break_if_fail {
                                case_success = false;
                                break;
                            }
                            case_success = false;
                            continue;
                        }
                    };

                    match cmd.run(&lib_parser, &fn_attr, &paras, &input) {
                        Ok(v) => {
                            if !v {
                                case_success = false;
                                if break_if_fail {
                                    error!(
                                        "Test case {} stopped because cmd {} executing failed!\n",
                                        self.name, &cmd.opfunc
                                    );
                                    if uses_input_args {
                                        error!("Input group '{}' args: {:?}", input_name, input);
                                    }
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("execute cmd {} failed! Error: {}\n", &cmd.opfunc, e);
                            if uses_input_args {
                                error!("Input group '{}' args: {:?}", input_name, input);
                            }
                            if break_if_fail {
                                case_success = false;
                                break;
                            }
                            case_success = false;
                        }
                    }
                }
            }

            if case_success {
                info!(
                    "Test case '{}' with input group '{}' succeeded",
                    self.name,
                    input_name
                );
            } else {
                error!(
                    "Test case '{}' with input group '{}' failed",
                    self.name,
                    input_name
                );
            }
            all_success = all_success && case_success;
        }
        all_success
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
            .into_par_iter()
            .map(|_| self.run_one_thread(lib_parser))
            .collect();

        debug!("results: {:#?}", results);
        let count = results.into_iter().filter(|&x| x).count();

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
        input_vars: &HashMap<String, String>,
    ) -> Result<bool, Box<dyn Error>> {
        #[cfg(target_os = "linux")]
        let start = high_precision_time();
        #[cfg(not(target_os = "linux"))]
        let start = std::time::Instant::now();

        let ret = lib_parser.call_func_attr(fn_attr, paras)?;

        #[cfg(target_os = "linux")]
        let duration = high_precision_time() - start;
        #[cfg(not(target_os = "linux"))]
        let duration = start.elapsed();

        if self.perf.unwrap_or(false) {
            info!("cmd '{}' executed cost {:?}", self.opfunc, duration);
        }

        let (expected, operator, is_success) = match &self.condition {
            Condition::Eq(v) => {
                let resolved = replace_vars(v.to_string(), input_vars);
                let expected = resolved.parse::<i32>()?;
                (expected, "==", ret == expected)
            },
            Condition::Ne(v) => {
                let resolved = replace_vars(v.to_string(), input_vars);
                let expected = resolved.parse::<i32>()?;
                (expected, "!=", ret != expected)
            },
        };

        let message = format!(
            "execute cmd: {}{:?}, expect return value {}{}, actual: {}",
            self.opfunc, self.args, operator, expected, ret
        );

        if !is_success {
            error!("{}", message);
        } else {
            debug!("{} succeeded", message);
        }

        Ok(is_success)
    }
}

#[cfg(target_os = "linux")]
fn high_precision_time() -> std::time::Duration {
    use std::mem::MaybeUninit;
    let mut ts = MaybeUninit::<libc::timespec>::uninit();
    unsafe {
        libc::clock_gettime(libc::CLOCK_MONOTONIC_RAW, ts.as_mut_ptr());
        let ts = ts.assume_init();
        std::time::Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
    }
}

fn replace_vars(s: String, vars: &HashMap<String, String>) -> String {
    vars.iter().fold(s, |acc, (k, v)| {
        acc.replace(&format!("${}", k), v)
    })
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
    fn test_concurrency_group() {
        let config_content = r#"
        concurrences = [
            { tests = ["test1", "test2"], name = "group1" }
        ]

        [[tests]]
        name = "test1"
        thread_num=1
        cmds = []

        [[tests]]
        name = "test2"
        thread_num=1
        cmds = []
        "#;

        let config: Config = toml::from_str(config_content).unwrap();
        assert_eq!(config.concurrences.unwrap()[0].name, "group1");
    }

    #[test]
    fn test_parse_config_with_concurrency() {
        let config_content = r#"
        concurrences = [
            { tests = ["test_rw_u32", "Test_str_fill"], name = "group1" },
            ]

        [[tests]]
        name = "test_rw_u32"
        thread_num=100
        cmds = [

            { opfunc = "my_malloc", expect_eq = 0, args = ["len=100", "mem_idx=1"] },
            { opfunc = "my_write32", expect_eq = 0, args = ["mem_idx=1", "offset=0", "val=888"] },
        ]

        [[tests]]
        name = "Test_str_fill"
        thread_num=100
        cmds = [

            { opfunc = "my_malloc", expect_eq = 0, args = ["len=100", "mem_idx=1"] },
            { opfunc = "my_write32", expect_eq = 0, args = ["mem_idx=1", "offset=0", "val=888"] },
        ]"#;

        let config: Config = toml::from_str(config_content).unwrap();
        assert_eq!(config.concurrences.clone().unwrap().len(), 1);
        assert_eq!(config.concurrences.unwrap()[0].name, "group1");
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
        assert_eq!(config.tests[0].cmds[0].condition, Condition::Eq("0".to_string()));
        assert_eq!(config.tests[0].cmds[1].condition, Condition::Ne("0".to_string()));
        assert_eq!(config.tests[0].cmds[2].condition, Condition::Eq("888".to_string()));
        assert_eq!(config.tests[0].cmds[3].condition, Condition::Ne("888".to_string()));
    }
}
