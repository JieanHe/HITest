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
    args: Vec<i64>,
}

#[derive(Debug)]
struct RunArgs {
    test_cfg: String,
    log_lvl: String,
    libscfg: String,
}

impl RunArgs {
    pub fn new(test_cfg: String, log_lvl: String, libscfg: String) -> Self {
        RunArgs {
            test_cfg,
            log_lvl,
            libscfg,
        }
    }
}

fn parse_args(matches: &ArgMatches) -> RunArgs {
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
    RunArgs::new(test_path, log_lvl.to_string(), libs_path)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // setting command agrs
    let matches = App::new("HITest")
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
        .get_matches();

    // parse command args
    let runargs = parse_args(&matches);

    // setting log level
    unsafe {
        std::env::set_var("RUST_LOG", runargs.log_lvl);
    }
    env_logger::init();

    // loading libraries
    let mut libparser: LibParse = LibParse::new();
    let libpath: std::path::PathBuf = std::env::current_dir()?
        .canonicalize()
        .expect(&format!(
            "failed to get absolute path of {}",
            &runargs.libscfg
        ))
        .join(&runargs.libscfg);

    let pathstr = libpath
        .to_str()
        .expect(&format!("failed to get path str of {}", &runargs.libscfg));

    libparser
        .load_config(&pathstr)
        .expect(&format!("failed to load {}", pathstr));

    // checking config file of test cases
    let config_content: String = fs::read_to_string(&runargs.test_cfg).expect(&format!(
        "failed to read test case file {}",
        &runargs.test_cfg
    ));
    if config_content.is_empty() {
        info!("input file {} is empty, do nothing!", runargs.test_cfg);
        return Ok(());
    }

    // loading test cases
    let config: Config = toml::from_str(&config_content).expect(&format!(
        "cannot parse the test case config, , content is {}",
        config_content
    ));
    if config.tests.is_empty() {
        info!("no test cases be find, do nothing!");
        return Ok(());
    }

    // run test cases
    for test in config.tests {
        info!("Starting run test case: {}", test.name);
        let mut succ = true;
        for cmd in test.cmds {
            debug!(
                "Executing cmd: {}{:?}, Expect result: {}",
                cmd.opfunc, cmd.args, cmd.expect_res
            );

            let args: Vec<i64> = cmd.args;

            let func = libparser
                .get_func(&cmd.opfunc)
                .expect("get function failed");

            let ret = match args.len() {
                0 => func(0, 0, 0, 0, 0, 0, 0, 0),
                1 => func(args[0].try_into().unwrap(), 0, 0, 0, 0, 0, 0, 0),
                2 => func(
                    args[0].try_into().unwrap(),
                    args[1].try_into().unwrap(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                ),
                3 => func(
                    args[0].try_into().unwrap(),
                    args[1].try_into().unwrap(),
                    args[2].try_into().unwrap(),
                    0,
                    0,
                    0,
                    0,
                    0,
                ),
                4 => func(
                    args[0].try_into().unwrap(),
                    args[1].try_into().unwrap(),
                    args[2].try_into().unwrap(),
                    args[3].try_into().unwrap(),
                    0,
                    0,
                    0,
                    0,
                ),
                5 => func(
                    args[0].try_into().unwrap(),
                    args[1].try_into().unwrap(),
                    args[2].try_into().unwrap(),
                    args[3].try_into().unwrap(),
                    args[4].try_into().unwrap(),
                    0,
                    0,
                    0,
                ),
                6 => func(
                    args[0].try_into().unwrap(),
                    args[1].try_into().unwrap(),
                    args[2].try_into().unwrap(),
                    args[3].try_into().unwrap(),
                    args[4].try_into().unwrap(),
                    args[5].try_into().unwrap(),
                    0,
                    0,
                ),
                7 => func(
                    args[0].try_into().unwrap(),
                    args[1].try_into().unwrap(),
                    args[2].try_into().unwrap(),
                    args[3].try_into().unwrap(),
                    args[4].try_into().unwrap(),
                    args[5].try_into().unwrap(),
                    args[6].try_into().unwrap(),
                    0,
                ),
                8 => func(
                    args[0].try_into().unwrap(),
                    args[1].try_into().unwrap(),
                    args[2].try_into().unwrap(),
                    args[3].try_into().unwrap(),
                    args[4].try_into().unwrap(),
                    args[5].try_into().unwrap(),
                    args[6].try_into().unwrap(),
                    args[7].try_into().unwrap(),
                ),
                _ => {
                    error!("parameters is too much, the lengh of parameter is limited to 8");
                    -1
                }
            };

            if ret != cmd.expect_res {
                error!("run cmd {} Failed!", cmd.opfunc);
                succ = false;
                break;
            }
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
