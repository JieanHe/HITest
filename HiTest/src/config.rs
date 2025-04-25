use super::{Cmd, ConcurrencyGroup, Test};
use log::{debug, info};
use serde::Deserialize;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[derive(Debug, Deserialize, Clone)]
struct Env {
    pub name: String,
    pub init: Vec<Cmd>,
    pub exit: Vec<Cmd>,
    pub tests: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    concurrences: Option<Vec<ConcurrencyGroup>>,
    #[serde(default)]
    envs: Vec<Env>,
    tests: Vec<Test>,
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
        .tests.clone()
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
        let mut total_tests = 0;
        let mut success_tests = 0;
        let mut failed_tests = 0;

        let tests = self.apply_envs();
        // run concurrency group
        let mut concurrency_tests: Vec<String> = Vec::new();
        if let Some(concurrences) = self.concurrences {
            info!("Starting run concurrency groups!");
            for mut concurrency in concurrences {
                let (success_num, expect) = concurrency.run(&tests);
                total_tests += expect;
                success_tests += success_num;
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
            let (succ, total) = test.run();
            total_tests += total;
            success_tests += succ;
            failed_tests += total - succ;
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cmd;

    #[test]
    fn test_validate_global_env() {
        let config = Config {
            envs: vec![
                Env { name: "global1".into(), init: vec![], exit: vec![], tests: vec![] },
                Env { name: "global2".into(), init: vec![], exit: vec![], tests: vec![] },
            ],
            tests: vec![],
            concurrences: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_test_multiple_envs() {
        let config = Config {
            envs: vec![
                Env { name: "env1".into(), init: vec![], exit: vec![], tests: vec!["test1".into()] },
                Env { name: "env2".into(), init: vec![], exit: vec![], tests: vec!["test1".into()] },
            ],
            tests: vec![Test { name: "test1".into(), ..Default::default() }],
            concurrences: None,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_env_application_order() {
        let global_init = Cmd { opfunc: "global_init".into(), ..Default::default() };
        let global_exit = Cmd { opfunc: "global_exit".into(),..Default::default() };
        let local_init = Cmd { opfunc: "local_init".into(), ..Default::default() };
        let local_exit = Cmd { opfunc: "local_exit".into(),..Default::default() };
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
            tests: vec![Test { name: "test1".into(), ..Default::default() }],
            concurrences: None,
        };

        let tests = config.apply_envs();

        let test = &tests[0];
        // 检查local env的cmd在最前面
        assert_eq!(test.cmds[0].opfunc, "global_init");
        assert_eq!(test.cmds[1].opfunc, "local_init");
        // 检查global env的exit cmd在最后面
        assert_eq!(test.cmds[test.cmds.len() - 2].opfunc, "local_exit");
        assert_eq!(test.cmds[test.cmds.len() - 1].opfunc, "global_exit");
    }
}
