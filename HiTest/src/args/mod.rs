use clap::{App, Arg, ArgMatches};
use log::info;
mod sample;
use sample::prepare_sample_files;

#[derive(Debug)]
pub struct RunArgs {
    pub test_cfg: String,
    pub log_lvl: String,
    pub libs_cfg: String,
    pub debug_test: Option<String>,
    pub serial: bool,
}

impl RunArgs {
    pub fn from_args() -> Self {
        let arg_matchs = init_command_line();
        parse_command_line(&arg_matchs)
    }
}

fn parse_command_line(matches: &ArgMatches) -> RunArgs {
    let test_path: String;
    let libs_path: String;

    if matches.is_present("sample") {
        info!("Run as sample mode, run `Hitest -t you_test_case.toml` to run your test cases.");
        (libs_path, test_path) = prepare_sample_files();
    } else {
        libs_path = matches
            .value_of("inputs")
            .expect("failed to get library config path")
            .to_string();
        test_path = matches
            .value_of("cases")
            .expect("failed to get test cases config path")
            .to_string();
    }
    let debug_test = if matches.is_present("debug") {
        Some(matches.value_of("debug").unwrap().to_string())
    } else {
        None
    };

    let mut log_lvl: &str = matches.value_of("log").unwrap_or("info");
    if (log_lvl == "1") || (log_lvl == "error") {
        log_lvl = "error";
    } else if (log_lvl == "2") || (log_lvl == "warn") {
        log_lvl = "warn";
    } else if (log_lvl == "4") || (log_lvl == "debug") {
        log_lvl = "debug";
    } else {
        log_lvl = "info";
    }

    RunArgs {
        test_cfg: test_path,
        log_lvl: log_lvl.to_string(),
        libs_cfg: libs_path,
        debug_test,
        serial: matches.is_present("serial"),
    }
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
            .short('l')
            .value_name("log level")
            .help("to control the log level, valid value contains [1,2,3,4]. which means [debug, info, warn, error]. default is info")
            .takes_value(true)
            .required(false),
    )
    .arg(
        Arg::with_name("inputs")
            .short('i')
            .long("input")
            .value_name("input libs config file")
            .help("a toml file contains all dependency libs")
            .takes_value(true)
            .required(false),
    )
        .arg(
        Arg::with_name("debug")
            .short('d')
            .long("debug")
            .value_name("debug test case")
            .help("only run this test case")
            .takes_value(true)
            .required(false),
    )
        .arg(
        Arg::with_name("serial")
            .long("serial")
            .value_name("default run in serial mode")
            .help("run test cases in serial mode, default is parallel mode. this not effected the test cases in concurrent mode and the thread_num > 1 cases")
            .takes_value(false)
            .required(false),
    )
    .arg(
        Arg::with_name("sample")
        .short('s')
            .long("sample")
            .value_name("sample mode")
            .help("run as sample mode, to see the format of config files")
            .takes_value(false)
            .required(false),
    )
    .get_matches()
}
