use crate::input::ArgValue;

use super::{ConcurrencyGroup, Env, ResourceEnv, Test};
use log::{debug, info, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    concurrences: Option<Vec<ConcurrencyGroup>>,
    #[serde(default)]
    envs: Vec<Env>,
    #[serde(default)]
    thread_env: Option<Env>,
    #[serde(default)]
    process_env: Option<Env>,
    #[serde(default)]
    shared_inputs: HashMap<String, HashMap<String, ArgValue>>,
    tests: Vec<Test>,
    #[serde(default = "default_false")]
    pub default_serial: bool,
    #[serde(default)]
    pub debug_test: Option<String>,
}
fn default_false() -> bool {
    false
}

impl Config {
    fn validate(&self) -> Result<(), String> {
        let global_envs = self.envs.iter().filter(|e| e.tests.is_empty()).count();
        if global_envs > 1 {
            return Err("Only one global env (with empty tests) is allowed".to_string());
        }

        for test in &self.tests {
            let applied_envs = self
                .envs
                .iter()
                .filter(|e| !e.tests.is_empty() && e.tests.contains(&test.name))
                .count();
            if applied_envs > 1 {
                return Err(format!(
                    "Test '{}' can only belong to one non-global env",
                    test.name
                ));
            }
        }
        Ok(())
    }

    fn set_env(test: &mut Test, env: &Env) {
        let init_cmds: Vec<_> = env.init.iter().cloned().collect();
        for cmd in init_cmds.iter().rev() {
            test.push_front(cmd.clone());
        }
        let exit_cmds: Vec<_> = env.exit.iter().cloned().collect();
        for cmd in exit_cmds {
            test.push_back(cmd.clone());
        }
        debug!("add env {} to test case {}", env.name, test.name);
    }

    fn apply_envs(&self) -> Vec<Test> {
        let global_env = self.envs.iter().find(|e| e.tests.is_empty());

        let mut tests = self
            .tests
            .clone()
            .into_iter()
            .map(|mut test| {
                for env in &self.envs {
                    if env.tests.contains(&test.name) {
                        Self::set_env(&mut test, &env);
                    }
                }
                test
            })
            .collect::<Vec<_>>();

        if let Some(global_env) = global_env {
            for test in &mut tests {
                Self::set_env(test, &global_env);
            }
        }
        tests
    }

    pub fn run(self) {
        if self.tests.is_empty() {
            info!("no test cases be find, do nothing!");
            return;
        }
        if let Err(e) = self.validate() {
            info!("validate config failed: {}", e);
            return;
        }

        // apply env init
        if let Some(ref process_env) = self.process_env {
            process_env.apply_env_init();
        }
        if let Some(ref thread_env) = self.thread_env {
            thread_env.apply_env_init();
        }
        ResourceEnv::init(self.thread_env.clone(), self.process_env.clone());
        // apply envs for test cases
        let tests = self.apply_envs();
        // merge shared inputs
        let shared_inputs = self.shared_inputs.clone();
        let mut tests = tests
            .into_iter()
            .map(|mut test| {
                test.resolve_refs(&shared_inputs).unwrap();
                test
            })
            .collect::<Vec<_>>();

        let mut total_tests = 0;
        let mut success_tests = 0;
        tests = if let Some(ref debug_test) = self.debug_test {
            info!("Starting debug test: {}", debug_test);
            let tests= tests
                .iter()
                .filter(|test| test.name.contains(debug_test))
                .map(|test| test.clone())
                .collect::<Vec<_>>();
            if tests.is_empty() {
                warn!("Debug test: {} not found!", debug_test);
            }
            tests
        } else {
            // run concurrency group
            let mut concurrency_tests: Vec<String> = Vec::new();
            if let Some(ref concurrences) = self.concurrences {
                info!("Starting run concurrency groups!");
                for concurrency in concurrences {
                    let res = concurrency.run(&tests);
                    total_tests += res.total;
                    success_tests += res.success;
                    concurrency.record_test(&mut concurrency_tests);
                }
            }

            // filter out concurrency test cases
            tests
                .into_iter()
                .filter(|test| !concurrency_tests.contains(&test.name))
                .map(|mut test| {
                    if test.serial.is_none() && self.default_serial && test.thread_num == 1 {
                        warn!(
                            "Test case {} marked as serial because of default_serial is set",
                            test.name
                        );
                        test.serial = Some(true);
                    }
                    test
                })
                .collect::<Vec<_>>()
        };

        // run remaining test cases
        for test in tests {
            let res = test.run();
            total_tests += res.total;
            success_tests += res.success;
        }

        // apply env exit
        if let Some(ref thread_env) = self.thread_env {
            thread_env.apply_env_exit();
        }
        if let Some(ref process_env) = self.process_env {
            process_env.apply_env_exit();
        }
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        if total_tests == success_tests {
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
            total_tests,
            success_tests,
            total_tests - success_tests
        )
        .unwrap();
        stdout.reset().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cmd;

    #[test]
    fn test_validate_global_env() {
        let config = Config {
            envs: vec![
                Env {
                    name: "global1".into(),
                    init: vec![],
                    exit: vec![],
                    tests: vec![],
                },
                Env {
                    name: "global2".into(),
                    init: vec![],
                    exit: vec![],
                    tests: vec![],
                },
            ],
            tests: vec![],
            concurrences: None,
            shared_inputs: HashMap::new(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_test_multiple_envs() {
        let config = Config {
            envs: vec![
                Env {
                    name: "env1".into(),
                    init: vec![],
                    exit: vec![],
                    tests: vec!["test1".into()],
                },
                Env {
                    name: "env2".into(),
                    init: vec![],
                    exit: vec![],
                    tests: vec!["test1".into()],
                },
            ],
            tests: vec![Test {
                name: "test1".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_env_application_order() {
        let global_init = Cmd {
            opfunc: "global_init".into(),
            ..Default::default()
        };
        let global_exit = Cmd {
            opfunc: "global_exit".into(),
            ..Default::default()
        };
        let local_init = Cmd {
            opfunc: "local_init".into(),
            ..Default::default()
        };
        let local_exit = Cmd {
            opfunc: "local_exit".into(),
            ..Default::default()
        };
        let config = Config {
            envs: vec![
                Env {
                    name: "global".into(),
                    init: vec![global_init],
                    exit: vec![global_exit],
                    tests: vec![],
                },
                Env {
                    name: "local".into(),
                    init: vec![local_init],
                    exit: vec![local_exit],
                    tests: vec!["test1".into()],
                },
            ],
            tests: vec![Test {
                name: "test1".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let tests = config.apply_envs();

        let test = &tests[0];

        assert_eq!(test.cmds[0].opfunc, "global_init");
        assert_eq!(test.cmds[1].opfunc, "local_init");

        assert_eq!(test.cmds[test.cmds.len() - 2].opfunc, "local_exit");
        assert_eq!(test.cmds[test.cmds.len() - 1].opfunc, "global_exit");
    }
}
