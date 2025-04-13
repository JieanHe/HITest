use crate::Test;
use log::{debug, error, info};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ConcurrencyGroup {
    tests: Vec<String>,
    #[serde(default = "default_name")]
    name: String,
    #[serde(default)]
    success_num: usize,
}
fn default_name() -> String {
    String::from("default_group")
}

impl ConcurrencyGroup {
    pub fn run(&mut self, tests: &Vec<Test>) -> bool {
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
            "Concurrency Group {} parallel running test cases: {:#?}",
            self.name, self.tests
        );

        let results: Vec<_> = test_cases
            .into_par_iter()
            .map(|test| test.run())
            .collect();

        let expect_success_num = results.len();
        let count = results.into_iter().filter(|&x| x).count();

        let success = count as usize == expect_success_num;
        if success {
            info!(
                "Parallel execute concurrency Group {} with {} thread, all passed!\n",
                self.name, expect_success_num
            );
        } else {
            error!(
                "Parallel execute concurrency Group {} with {} thread, {} passed!\n",
                self.name, expect_success_num, count
            );
        }
        self.success_num = count;
        return success;
    }

    pub fn record_test(&self, tests: &mut Vec<String>) {
        for test in &self.tests {
            tests.push(test.clone());
        }
    }

    pub fn len(&self) -> usize {
        self.tests.len()
    }

    pub fn success_num(&self) -> usize {
        self.success_num
    }
}

#[cfg(test)]
mod tests {
    use crate::Config;

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
        assert_eq!(config.concurrences.clone().unwrap()[0].name, "group1");
        assert_eq!(config.concurrences.clone().unwrap().len(), 1);
        assert_eq!(config.concurrences.unwrap()[0].tests.len(), 2);
    }

}
