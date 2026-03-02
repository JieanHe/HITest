use crate::{ResourceEnv, Test, TestResult};
use log::{debug, error, info, warn};
use rand::seq::SliceRandom;
use rand::thread_rng;
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
                passed: 0,
                failed: 0,
                skipped:0,
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
                passed: 0,
                failed: 0,
                skipped:0,
            };
        }

        debug!(
            "Concurrency Group {} parallel running test cases: {:#?}",
            self.name, self.tests
        );
        let max_thread = {
            let res_env = ResourceEnv::get_instance().unwrap().read().unwrap();
            res_env.max_threads
        };
        let results: Vec<_> = if let Some(max_thread) = max_thread{
            if test_cases.len() > max_thread {
                warn!("Concurrency Group {} total test cases is {}, but max-threads is {} thread, will be grouped.",
                self.name, test_cases.len(), max_thread);

                let mut rng = thread_rng();
                let mut shuffled_tests = test_cases;
                shuffled_tests.shuffle(&mut rng);

                let mut results = Vec::new();
                for chunk in shuffled_tests.chunks(max_thread) {
                    let chunk_results: Vec<_> = chunk.into_par_iter().map(|test| test.run()).collect();
                    results.extend(chunk_results);
                }
                results
            } else {
                test_cases.into_par_iter().map(|test| test.run()).collect()
            }
        } else {
            test_cases.into_par_iter().map(|test| test.run()).collect()
        };

        let passed = results.iter().map(|r| r.passed).sum();
        let failed = results.iter().map(|r| r.failed).sum();
        let skipped = results.iter().map(|r| r.skipped).sum();
        if failed == 0 {
            info!(
                "Parallel execute concurrency Group {} with {} thread, all passed!\n",
                self.name, failed
            );
        } else {
            error!(
                "Parallel execute concurrency Group {} with {} thread, {} passed!\n",
                self.name, failed + passed, passed
            );
        }

        TestResult { passed, failed, skipped }
    }

    pub fn record_test(&self, tests: &mut Vec<String>) {
        for test in &self.tests {
            tests.push(test.clone());
        }
    }
}
