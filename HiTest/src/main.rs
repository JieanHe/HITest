use clap::{App, Arg, ArgMatches};
use log::{debug, error, info};
use serde::Deserialize;
use std::fs;

use libparser::*;
mod sample;
use sample::prepare_sample_files;

#[derive(Debug, Deserialize)]
struct Config {
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

#[derive(Debug)]
struct run_args {
    test_cfg: String,
    log_lvl: String,
    libs_cfg: String,
}

impl run_args {
    pub fn new(test_cfg: String, log_lvl: String, libs_cfg: String) -> Self {
        run_args {
            test_cfg,
            log_lvl,
            libs_cfg,
        }
    }
}

fn parse_args(matches: &ArgMatches) -> run_args {
    let test_path: String;
    let libs_path: String;

    if matches.is_present("sample") {
        info!("Run as sample mode, run `Hitest -t you_test_case.toml` to run your test cases.");
        (libs_path, test_path) = prepare_sample_files();
    } else {
        libs_path = matches
            .value_of("libs")
            .expect("failed to get lib path")
            .to_string();
        test_path = matches
            .value_of("cases")
            .expect("failed to get lib path")
            .to_string();
    }

    let mut log_lvl: &str = matches.value_of("log").unwrap_or("info");
    let valid_lvl = ["info", "error", "debug"];
    if !valid_lvl.contains(&log_lvl) {
        log_lvl = "info";
    }
    run_args::new(test_path, log_lvl.to_string(), libs_path)
}

fn init_command_line() -> ArgMatches {
    App::new("HITest")
    .version("1.0")
    .author("He Jiean")
    .about("A Integration Testing tool for executing and verifying commands")
    .arg(
        Arg::with_name("cases")
            .short('t')
            .long("test_case")
            .value_name("test cases config file")
            .help(
                r#"a toml file path wthich contains test cases, see sample/tc_libmalloc.toml"#,
            )
            .takes_value(true)
            .required(false),
    )
    .arg(
        Arg::with_name("log")
            .long("log")
            .value_name("log level")
            .help("to control the log level, valid value contains [info, debug, error]. Invalid value will be set to 'info' as default ")
            .takes_value(true)
            .required(false),
    )
    .arg(
        Arg::with_name("libs")
            .short('l')
            .long("libs")
            .value_name("libs config file")
            .help("a toml file contains all dependency libs")
            .takes_value(true)
            .required(false),
    )
    .arg(
        Arg::with_name("sample")
            .long("sample")
            .value_name("sample mode")
            .help("run as sample mode, to see the format of config files")
            .takes_value(false)
            .required(false),
    )
    .get_matches()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // setting command agrs
    let matches: ArgMatches = init_command_line();
    let run_args = parse_args(&matches);

    // setting log level
    unsafe {
        std::env::set_var("RUST_LOG", run_args.log_lvl);
    }
    env_logger::init();

    // loading libraries
    let lib_cfg_path: std::path::PathBuf = std::env::current_dir()
        .unwrap()
        .canonicalize()
        .unwrap()
        .join(&run_args.libs_cfg);

    let lib_cfg_path = lib_cfg_path.to_str().unwrap();
    let lib_parser = LibParse::new(&lib_cfg_path).unwrap();

    // checking config file of test cases
    let config_content: String = fs::read_to_string(&run_args.test_cfg).expect(&format!(
        "failed to read test case file {}",
        &run_args.test_cfg
    ));

    // loading test cases
    let config: Config = match toml::from_str(&config_content) {
        Ok(t) => t,
        _ => {
            return Err(format!(
                "cannot parse the test case config [{}], invalid toml format?",
                &run_args.test_cfg,
            )
            .into())
        }
    };

    if config.tests.is_empty() {
        info!("no test cases be find, do nothing!");
        return Ok(());
    }

    // run test cases
    for test in config.tests {
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

    Ok(())
}
