use libloading::{Library, Symbol};
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::HashMap,
    error::Error,
    ffi::CString,
    fmt, fs,
    os::raw::c_int,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use toml;

#[derive(Debug)]
struct LibError(String);
impl fmt::Display for LibError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for LibError {}

#[derive(Deserialize)]
struct LibConfig {
    para_len: usize,
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
}

//  4k bytes buffer for api communication, buffer of parameters, number of parameters, and buffer of return value.
type FnPtr = extern "C" fn(
    *mut u64,      // uint64_t* param_page, for apis communication
    *const u64, // const uint64_t* params, send parameters to wrapper
    c_int,         // int params_len
) -> c_int;

pub struct FnAttr {
    fnptr: FnPtr,
    paras: Vec<String>,
}

impl FnAttr {
    fn new(fnptr: FnPtr, paras: Vec<String>) -> Self {
        FnAttr { fnptr, paras }
    }

    fn run(&self, params: &[u64]) -> i32 {
        TLS_PAGE.with(|addr| {
            let mut addr = addr.borrow_mut();
            (self.fnptr)(addr.as_mut_ptr(), params.as_ptr(), params.len() as c_int) as i32
        })
    }

    pub fn parse_params(&self, config_params: &Vec<String>) -> Result<Vec<u64>, Box<dyn Error>> {
        if config_params.len() != self.paras.len() {
            return Err(format!(
                "params size mismatch: expect params contains {} element, but got {}",
                self.paras.len(),
                config_params.len()
            )
            .into());
        }

        let mut params: Vec<u64> = Vec::new();

        for key in self.paras.clone() {
            let mut succ = false;
            for value in config_params {
                if let Some(para) = value.strip_prefix(&format!("{}=", key)) {
                    if let Ok(num) = para.parse::<u64>() {
                        params.push(num);
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
    para_len: usize,
    #[allow(dead_code)]
    libs: Vec<Arc<Library>>, // to keep library loaded from file on live, and this field will never be used.
}

impl LibParse {
    pub fn new(config: &str) -> Result<Self, Box<dyn Error>> {
        let mut libs = Vec::new();
        let mut funcs = HashMap::new();

        let config = fs::read_to_string(config)
            .map_err(|_| LibError(format!("failed to read config file {}", config)))?;

        let config: LibConfig = toml::from_str(&config)
            .map_err(|_| LibError(format!("failed to parse TOML config file {}", config)))?;

        for lib_file in config.libs {
            let lib = unsafe { Library::new(lib_file.path.clone()) }.map_err(|e| {
                LibError(format!(
                    "failed to load library file {}: {}",
                    lib_file.path, e
                ))
            })?;

            let lib_arc = Arc::new(lib);
            libs.push(lib_arc.clone());
            for func in lib_file.funcs {
                let func_name = func.name;
                let c_func_name = CString::new(func_name.clone())?;
                let func_ptr: Symbol<FnPtr> = unsafe { lib_arc.get(c_func_name.as_bytes()) }
                    .map_err(|_| {
                        LibError(format!(
                            "failed to get function {} form library {}",
                            func_name, lib_file.path
                        ))
                    })?;
                let func_attr = FnAttr::new(*func_ptr, func.paras);
                funcs.insert(func_name, Arc::new(Box::new(func_attr)));
            }
        }
        let para_len = config.para_len;
        Ok(LibParse {
            libs,
            para_len,
            funcs,
        })
    }

    pub fn get_func(&self, name: &str) -> Result<Arc<Box<FnAttr>>, Box<dyn Error>> {
        match self.funcs.get(name) {
            Some(arc_box_func_attr) => Ok(arc_box_func_attr.clone()),
            None => Err(format!("Function '{}' not found", name).into()),
        }
    }

    pub fn call_func(
        &self,
        func_name: &str,
        config_params: &Vec<String>,
    ) -> Result<i32, Box<dyn Error>> {
        let func_attr = self.get_func(func_name)?;
        let mut params: Vec<u64> = func_attr.parse_params(config_params)?;
        params.resize(self.para_len, 0);
        Ok(func_attr.run(&params).try_into().unwrap())
    }

    pub fn call_func_attr(
        &self,
        fn_attr: &FnAttr,
        config_params: &Vec<u64>,
    ) -> Result<i32, Box<dyn Error>> {
        let mut params: Vec<u64> = config_params.clone();
        params.resize(self.para_len, 0);

        Ok(fn_attr.run(&params).try_into().unwrap())
    }
}

pub fn compile_lib(file: PathBuf) {
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
    let target = file.parent().unwrap().join(target);

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

#[cfg(test)]
mod tests {

    use super::*;
    use std::{env, fs::File, io::Write};

    #[test]
    fn test_libmalloc() {
        let libmalloc = env::current_dir().unwrap().join("../sample/libmalloc.c");
        compile_lib(libmalloc);

        let config_path = env::current_dir()
            .unwrap()
            .join("../sample/dependlibs.toml");
        let config_path = config_path.to_str().unwrap();
        // generate config file
        {
            #[cfg(windows)]
            let config_content = r#"
para_len = 3
[[libs]]
path = "../sample/libmalloc.dll"
funcs = [
    { name = "my_malloc", paras = ["len", "mem_idx"] },
    { name = "my_free", paras = ["mem_idx"] },
    { name = "my_read32", paras = ["mem_idx", "offset"] },
    { name = "my_write32", paras = ["mem_idx", "offset", "val"] }
]
"#;
            #[cfg(unix)]
            let config_content = r#"
para_len = 3
[[libs]]
path = "../sample/libmalloc.so"
funcs = [
    { name = "my_malloc", paras = ["len", "mem_idx"] },
    { name = "my_free", paras = ["mem_idx"] },
    { name = "my_read32", paras = ["mem_idx", "offset"] },
    { name = "my_write32", paras = ["mem_idx", "offset", "val"] }
]
            "#;
            let mut file = File::create(config_path).unwrap();
            let _ = file.write_all(config_content.as_bytes());
        }

        let libparse = LibParse::new(&config_path)
            .expect(&format!("failed to load lib config file {}", &config_path));

        assert_eq!(libparse.funcs.len(), 4);
        assert_eq!(libparse.get_func("my_malloc").unwrap().paras.len(), 2);
        assert_eq!(libparse.get_func("my_free").unwrap().paras.len(), 1);
        assert_eq!(libparse.get_func("my_read32").unwrap().paras.len(), 2);
        assert_eq!(libparse.get_func("my_write32").unwrap().paras.len(), 3);
        assert_eq!(
            libparse
                .call_func(
                    "my_malloc",
                    &vec!["len=4".to_string(), "mem_idx=1".to_string()]
                )
                .unwrap(),
            0
        );
        assert_eq!(
            libparse
                .call_func(
                    "my_write32",
                    &vec![
                        "offset=0".to_string(),
                        "mem_idx=1".to_string(),
                        "val=888".to_string()
                    ]
                )
                .unwrap(),
            0
        );
        assert_eq!(
            libparse
                .call_func(
                    "my_read32",
                    &vec!["offset=0".to_string(), "mem_idx=1".to_string()]
                )
                .unwrap(),
            888
        );
        assert_eq!(
            libparse
                .call_func("my_free", &vec!["mem_idx=1".to_string()])
                .unwrap(),
            0
        );
    }
}
