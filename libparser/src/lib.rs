use libloading::{Library, Symbol};
use serde::Deserialize;
use toml;

use std::{
    collections::HashMap,
    env,
    error::Error,
    ffi::CString,
    fs,
    os::raw::{c_long, c_longlong},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

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

type FnPtr = extern "C" fn(
    c_longlong,
    c_longlong,
    c_longlong,
    c_longlong,
    c_longlong,
    c_longlong,
    c_longlong,
    c_longlong,
) -> c_long;

struct FnAttr {
    ptr: FnPtr,
    paras: Vec<String>,
}
impl FnAttr {
    pub fn new(ptr: FnPtr, paras: Vec<String>) -> Self {
        FnAttr { ptr, paras }
    }

    pub fn run(&self, params: &[c_longlong]) -> i32 {
        if params.len() < 8 {
            panic!("Insufficient parameters provided");
        }

        (self.ptr)(
            params[0], params[1], params[2], params[3], params[4], params[5], params[6], params[7],
        )
    }
}

// 引入生命周期参数 'a
pub struct LibParse {
    libs: Vec<Arc<Library>>,
    funcs: HashMap<String, Arc<Box<FnAttr>>>,
}

impl LibParse {
    pub fn new() -> Self {
        LibParse {
            libs: Vec::new(),
            funcs: HashMap::new(),
        }
    }

    pub fn load_config(&mut self, config: &str) -> Result<&mut Self, Box<dyn Error>> {
        let config = fs::read_to_string(config).expect(&format!("failed to read file {}", config));
        let config: LibConfig =
            toml::from_str(&config).expect(&format!("failed to parse toml file {}", config));
        for libconfig in config.libs {
            let lib = unsafe {
                Library::new(libconfig.path.clone())
                    .expect(&format!("failed to load lib file {}", libconfig.path))
            };
            let lib_arc = Arc::new(lib);
            self.libs.push(lib_arc.clone());
            for func in libconfig.funcs {
                let func_name = func.name;
                let c_func_name = CString::new(func_name.clone())?;
                let func_ptr: Symbol<FnPtr> = unsafe { lib_arc.get(c_func_name.as_bytes())? };
                let func_attr = FnAttr::new(*func_ptr, func.paras);
                self.funcs.insert(func_name, Arc::new(Box::new(func_attr)));
            }
        }
        Ok(self)
    }

    pub fn get_func(&self, name: &str) -> Result<Arc<Box<FnAttr>>, Box<dyn Error>> {
        match self.funcs.get(name) {
            Some(arc_box_func_attr) => Ok(arc_box_func_attr.clone()), // 直接返回 Arc 的克隆
            None => Err(format!("Function '{}' not found", name).into()),
        }
    }

    pub fn call_func(&self, func_name: &str, config_params: &Vec<String>) -> i32 {
        let func_attr = self.get_func(func_name).expect(&failed_get_func(func_name));
        let mut params: Vec<i64> = Vec::new();

        for key in &func_attr.paras {
            for value in config_params {
                if let Some(value_key) = value.strip_prefix(key) {
                    if let Some(index) = value_key.strip_prefix("=") {
                        if let Ok(num) = index.parse::<i64>() {
                            params.push(num);
                            break;
                        } else {
                            println!("Failed to parse '{}' as i64", index);
                        }
                    } else {
                        println!("cannot find param {} of function {}", key, func_name);
                    }
                }
            }
        }

        params.resize(8, 0);
        func_attr.run(&params).try_into().unwrap()
    }
}

pub fn failed_load_file(file: &str) -> String {
    format!("failed to load file {}", file)
}

pub fn failed_get_func(func: &str) -> String {
    format!("failed to get function {}", func)
}

pub fn compile_lib(file: PathBuf) {
    let file_name = file
        .file_name()
        .expect(&format!("invlaid file {:?}", &file))
        .to_str()
        .and_then(|s| s.split('.').next())
        .unwrap();
    let ext = if env::var("TARGET")
        .unwrap_or("windows".to_string())
        .contains("windows")
    {
        ".dll"
    } else {
        ".so"
    };

    let lib_path = file.parent().unwrap().join(file_name.to_owned() + ext);

    let compiler = "gcc";
    // compile library
    let status = Command::new(compiler)
        .arg("--shared")
        .arg("-fPIC")
        .arg(file)
        .arg("-o")
        .arg(&lib_path)
        .status();

    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                if Path::new(&lib_path).exists() {
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
    use rand::Rng;
    use std::{env, fs::File, io::Write}; // 引入 Rng trait，它提供了 next_u32 方法

    #[test]
    fn test_libmalloc() {
        let binding = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let libmalloc = Path::new(&binding)
            .parent()
            .unwrap()
            .join("sample")
            .join("libmalloc.c");
        compile_lib(libmalloc);

        let config_path = env::current_dir()
            .unwrap()
            .join("../sample/dependlibs.toml");
        let config_path = config_path.to_str().unwrap();

        // generate config file
        {
            let config_content = r#"
[[libs]]
path = "../sample/libmalloc.dll"
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

        let mut libparse = LibParse::new();
        libparse
            .load_config(&config_path)
            .expect(&failed_load_file(&config_path));
        assert_eq!(libparse.funcs.len(), 4);
        assert_eq!(libparse.get_func("my_malloc").unwrap().paras.len(), 2);
        assert_eq!(libparse.get_func("my_free").unwrap().paras.len(), 1);
        assert_eq!(libparse.get_func("my_read32").unwrap().paras.len(), 2);
        assert_eq!(libparse.get_func("my_write32").unwrap().paras.len(), 3);
        assert_eq!(libparse.call_func("my_malloc", &vec!["len=4".to_string(), "mem_idx=1".to_string()]), 0);
        assert_eq!(libparse.call_func("my_write32", &vec!["offset=0".to_string(), "mem_idx=1".to_string(),"val=888".to_string()]), 0);
        assert_eq!(libparse.call_func("my_read32", &vec!["offset=0".to_string(), "mem_idx=1".to_string()]), 888);
        assert_eq!(libparse.call_func("my_free", &vec!["mem_idx=1".to_string()]), 0);
        
    }
}
