use libparser::*;
use log::warn;
use std::fs;
mod args;
use args::RunArgs;
use hitest::Config;
use hitest::ThreadInfo;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ThreadInfo::get_instance();
    // parse command agrs
    let run_args = RunArgs::from_args();

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
    LibParse::init(&lib_cfg_path).unwrap();

    // checking config file of test cases
    let config_content: String = fs::read_to_string(&run_args.test_cfg).expect(&format!(
        "failed to read test case file {}",
        &run_args.test_cfg
    ));

    // loading test cases
    let mut config: Config = match toml::from_str(&config_content) {
        Ok(t) => t,
        Err(e) => {
            return Err(format!(
                "cannot parse the test case config [{}], error: {}?",
                &run_args.test_cfg, e
            )
            .into())
        }
    };

    // run test cases

    if config.debug_test.is_none() {
        if let Some(name) = run_args.debug_test {
            warn!(
                "debug test is set as {}, will run this test case only",
                &name
            );
            config.debug_test = Some(name);
        }
    }

    if !config.default_serial {
        config.default_serial = run_args.serial;
    }
    config.run();
    Ok(())
}
