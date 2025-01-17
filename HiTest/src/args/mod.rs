use clap::{App, Arg, ArgMatches};
use log::info;
mod sample;
use sample::prepare_sample_files;

#[derive(Debug)]
pub struct RunArgs {
    pub test_cfg: String,
    pub log_lvl: String,
    pub libs_cfg: String,
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
    RunArgs {
        test_cfg: test_path,
        log_lvl: log_lvl.to_string(),
        libs_cfg: libs_path,
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
