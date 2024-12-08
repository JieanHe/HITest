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
struct Lib {
    path: String,
    funcs: Vec<String>,
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

// 引入生命周期参数 'a
pub struct LibParse {
    libs: Vec<Arc<Library>>,
    funcs: HashMap<String, Arc<Box<FnPtr>>>,
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
            for func_name in libconfig.funcs {
                let c_func_name = CString::new(func_name.clone())?;
                let func_ptr: Symbol<FnPtr> = unsafe { lib_arc.get(c_func_name.as_bytes())? };
                self.funcs.insert(func_name, Arc::new(Box::new(*func_ptr)));
            }
        }
        Ok(self)
    }

    pub fn get_func(&self, name: &String) -> Result<Arc<Box<FnPtr>>, Box<dyn Error>> {
        match self.funcs.get(name) {
            Some(arc_box_func_ptr) => Ok(arc_box_func_ptr.clone()), // 直接返回 Arc 的克隆
            None => Err(format!("Function '{}' not found", name).into()),
        }
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

        let config_path = env::current_dir().unwrap().join("../sample/dependlibs.toml");
        let config_path = config_path.to_str().unwrap();

        // generate config file
        {
            let config_content = r#"
libs = [
    {path = "../sample/libmalloc.dll", funcs = ["my_malloc", "my_free", "my_read32", "my_write32"]},
]"#;
            let mut file = File::create(config_path).unwrap();
            let _ = file.write_all(config_content.as_bytes());
        }

        let mut libparse = LibParse::new();
        libparse
            .load_config(&config_path)
            .expect(&failed_load_file(&config_path));

        // test: malloc -> write -> read -> free
        let my_malloc = libparse
            .get_func(&"my_malloc".to_string())
            .expect(&failed_get_func("my_malloc"));
        my_malloc(100, 0, 0, 0, 0, 0, 0, 0);

        for idx in 0..25 {
            let mut rng: rand::rngs::ThreadRng = rand::thread_rng();
            let random_number: i32 = rng.gen();

            let my_write32 = libparse
                .get_func(&"my_write32".to_string())
                .expect(&failed_get_func("my_write32"));
            my_write32(0, random_number.into(), idx * 4, 0, 0, 0, 0, 0);

            let my_read32 = libparse
                .get_func(&"my_read32".to_string())
                .expect(&failed_get_func("my_read32"));
            let v = my_read32(0, idx * 4, 0, 0, 0, 0, 0, 0);

            assert_eq!(v, random_number);
        }

        let my_free = libparse
            .get_func(&"my_free".to_string())
            .expect(&failed_get_func("my_free"));
        my_free(0, 0, 0, 0, 0, 0, 0, 0);
    }
}
