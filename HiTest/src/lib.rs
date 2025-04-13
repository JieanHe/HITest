use log::{debug, error, info};
use serde::Deserialize;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod concurrency;
use concurrency::ConcurrencyGroup;
mod condition;
use condition::Condition;
mod cmd;
use cmd::Cmd;
mod test;
use test::Test;

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
                        test.push_front(env.init.clone());
                        test.push_back(env.exit.clone());
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
