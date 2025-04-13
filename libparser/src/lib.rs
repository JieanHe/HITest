use libloading::{Library, Symbol};
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::HashMap,
    error::Error,
    ffi::CString,
    fs,
    os::raw::c_longlong,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use toml;
mod error;
pub use error::LibError;
mod perf;
use perf::Perf;

#[derive(Deserialize)]
struct LibConfig {
    libs: Vec<Lib>,
}

#[derive(Deserialize)]
struct LibFunc {
    name: String,
    paras: Vec<String>,
}

#[derive(Deserialize)]
struct Lib {
    path: String,
    funcs: Vec<LibFunc>,
}

type AddressArray = [u64; 512];
thread_local! {
    static TLS_PAGE: RefCell<AddressArray> = RefCell::new([0; 512]);
    static C_STRINGS: RefCell<Vec<CString>> = RefCell::new(Vec::new());
}

//  4k bytes buffer for api communication, buffer of parameters, number of parameters, and buffer of return value.
type FnPtr = extern "C" fn(
    *mut u64,   // uint64_t* param_page, for apis communication
    *const i64, // const uint64_t* params, send parameters to wrapper
    c_longlong, // int params_len
) -> c_longlong;

pub struct FnAttr {
    fnptr: FnPtr,
    paras: Vec<String>,
}

impl FnAttr {
    fn new(fnptr: FnPtr, paras: Vec<String>) -> Self {
        FnAttr { fnptr, paras }
    }

    fn run(&self, params: &[i64]) -> i64 {
        TLS_PAGE.with(|addr| {
            let mut addr = addr.borrow_mut();
            (self.fnptr)(
                addr.as_mut_ptr(),
                params.as_ptr(),
                params.len() as c_longlong,
            ) as i64
        })
    }

    pub fn parse_params(&self, config_params: &Vec<String>) -> Result<Vec<i64>, Box<dyn Error>> {
        if config_params.len() != self.paras.len() {
            return Err(format!(
                "params size mismatch: expect params contains {} element, but got {}",
                self.paras.len(),
                config_params.len()
            )
            .into());
        }

        let mut params: Vec<i64> = Vec::new();

        for key in self.paras.clone() {
            let mut succ = false;
            for value in config_params {
                if let Some(para) = value.strip_prefix(&format!("{}=", key)) {
                    if let Ok(num) = if para.starts_with("0x") || para.starts_with("0X") {
                        i64::from_str_radix(&para[2..], 16)
                    } else {
                        para.parse::<i64>()
                    } {
                        params.push(num);
                        succ = true;
                        break;
                    } else if para.starts_with('\'') && para.ends_with('\'') {
                        let content = &para[1..para.len() - 1];
                        let c_str = CString::new(content)
                            .map_err(|e| format!("Invalid string parameter: {}", e))?;
                        let raw_ptr = c_str.clone().into_raw();
                        C_STRINGS.with(|c_strings| {
                            let mut c_strings = c_strings.borrow_mut();
                            c_strings.push(c_str);
                        });
                        params.push(raw_ptr as i64);
                        succ = true;
                        break;
                    }
                }
            }
            if !succ {
                return Err(format!(
                    "failed to get param [{}], all candidate params are {:?}",
                    key, config_params
                )
                .into());
            }
        }

        Ok(params)
    }
}

pub struct LibParse {
    funcs: HashMap<String, Arc<Box<FnAttr>>>,

    #[allow(dead_code)]
    // to keep library loaded from file on live, and this field will never be used.
    libs: Vec<Arc<Library>>,
}

use std::sync::{Once, RwLock};

static mut LIB_PARSER_INSTANCE: Option<RwLock<LibParse>> = None;
static INIT: Once = Once::new();

impl LibParse {
    // for test only
    #[cfg(test)]
    pub fn new_with_mock() -> Self {
        extern "C" fn mock_fn(_: *mut u64, _: *const i64, _: i64) -> i64 {
            0
        }

        let mut funcs = HashMap::new();
        funcs.insert(
            "test_func".to_string(),
            Arc::new(Box::new(FnAttr {
                fnptr: mock_fn,
                paras: vec!["param1".to_string(), "param2".to_string()],
            })),
        );

        LibParse {
            funcs,
            libs: Vec::new(),
        }
    }

    pub fn get_func(&self, name: &str) -> Result<Arc<Box<FnAttr>>, Box<dyn Error>> {
        match self.funcs.get(name) {
            Some(arc_box_func_attr) => Ok(arc_box_func_attr.clone()),
            None => Err(format!("Function '{}' not found", name).into()),
        }
    }

    pub fn execute(
        &self,
        fn_name: String,
        config_params: &Vec<String>,
    ) -> Result<i64, Box<dyn Error>> {
        let fn_attr = self.get_func(&fn_name)?;
        let params: Vec<i64> = fn_attr.parse_params(config_params)?;

        Ok(fn_attr.run(&params).try_into().unwrap())
    }

    pub fn execute_with_perf(
        &self,
        fn_name: String,
        config_params: &Vec<String>,
    ) -> Result<(i64, Perf), Box<dyn Error>> {
        let fn_attr = self.get_func(&fn_name)?;
        let params: Vec<i64> = fn_attr.parse_params(config_params)?;

        let mut perf = Perf::new();
        let ret: i64 = fn_attr.run(&params).try_into()?;
        perf.record();
        Ok((ret, perf))
    }

    pub fn init(config: &str) -> Result<(), Box<dyn Error>> {
        unsafe {
            INIT.call_once(|| {
                let instance = LibParse::new(config).expect("Failed to initialize LibParse");
                LIB_PARSER_INSTANCE = Some(RwLock::new(instance));
            });
            Ok(())
        }
    }

    pub fn get_instance() -> Result<&'static RwLock<LibParse>, Box<dyn Error>> {

        unsafe {
            LIB_PARSER_INSTANCE
                .as_ref()
                .ok_or("LibParse not initialized".into())
        }
    }

    fn new(config: &str) -> Result<Self, Box<dyn Error>> {
        let mut libs = Vec::new();
        let mut funcs = HashMap::new();

        let config = fs::read_to_string(config)
            .map_err(|e| LibError::LoadError(config.into(), format!("{}", e)))?;

        let config: LibConfig = toml::from_str(&config)
            .map_err(|e| LibError::LoadError(config.into(), format!("{}", e)))?;

        for lib_file in config.libs {
            let lib = unsafe { Library::new(lib_file.path.clone()) }
                .map_err(|e| LibError::LoadError(lib_file.path, format!("{}", e)))?;

            let lib_arc = Arc::new(lib);
            libs.push(lib_arc.clone());
            for func in lib_file.funcs {
                let func_name = func.name;
                let c_func_name = CString::new(func_name.clone())?;
                let func_ptr: Symbol<FnPtr> = unsafe { lib_arc.get(c_func_name.as_bytes()) }
                    .map_err(|_| LibError::FuncNotFound(func_name.clone()))?;
                let func_attr = FnAttr::new(*func_ptr, func.paras);
                funcs.insert(func_name, Arc::new(Box::new(func_attr)));
            }
        }

        Ok(LibParse { libs, funcs })
    }
}

pub fn compile_lib(file: PathBuf, out_dir: &str) {
    let file_name = file
        .file_name()
        .expect(&format!("invlaid file {:?}", &file))
        .to_str()
        .and_then(|s| s.split('.').next())
        .unwrap();

    let target = if cfg!(unix) {
        format!("{}.so", file_name)
    } else if cfg!(windows) {
        format!("{}.dll", file_name)
    } else {
        panic!("Unsupported platform");
    };
    let target = format!("{}/{}", out_dir, target);

    let compiler = "gcc";

    let status = Command::new(compiler)
        .arg("--shared")
        .arg("-fPIC")
        .arg(file)
        .arg("-o")
        .arg(&target)
        .status();

    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                if Path::new(&target).exists() {
                    println!("Library file was created successfully.");
                } else {
                    println!("Library file was not created.");
                }
            } else {
                println!("Command failed to executed.");
            }
        }
        Err(e) => {
            eprintln!("Failed to execute command: {}", e);
        }
    }
}
