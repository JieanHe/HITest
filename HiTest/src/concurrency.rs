use crate::{Test, TestResult};
use log::{debug, error, info};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ConcurrencyGroup {
    tests: Vec<String>,
    #[serde(default = "default_name")]
    name: String,
}
fn default_name() -> String {
    String::from("default_group")
}

impl ConcurrencyGroup {
    pub fn run(&self, tests: &Vec<Test>) -> TestResult {
        if self.tests.is_empty() {
            return TestResult {
                total: 0,
                success: 0,
            };
        }

        let mut test_cases: Vec<Test> = Vec::new();
        for original_test in tests {
            if self.tests.contains(&original_test.name) {
                let mut cloned_test = original_test.clone();
                cloned_test.name = format!("{}_{}", self.name, original_test.name);
                test_cases.push(cloned_test);
            }
        }

        if test_cases.is_empty() {
            return TestResult {
                total: 0,
                success: 0,
            };
        }

        debug!(
            "Concurrency Group {} parallel running test cases: {:#?}",
            self.name, self.tests
        );

        let results: Vec<_> = test_cases.into_par_iter().map(|test| test.run()).collect();

        let total = results.iter().map(|r| r.total).sum();
        let success = results.iter().map(|r| r.success).sum();

        if total == success {
            info!(
                "Parallel execute concurrency Group {} with {} thread, all passed!\n",
                self.name, total
            );
        } else {
            error!(
                "Parallel execute concurrency Group {} with {} thread, {} passed!\n",
                self.name, total, success
            );
        }

        TestResult { total, success }
    }

    pub fn record_test(&self, tests: &mut Vec<String>) {
        for test in &self.tests {
            tests.push(test.clone());
        }
    }
}
