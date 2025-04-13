
use super::{Cmd, ConcurrencyGroup, Test};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use serde::Deserialize;
use log::{debug, info};
use std::io::Write;

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
