use libparser::compile_lib;
use std::{env, fs::File, io::Write, path::Path};

pub fn prepare_sample_files() -> (String, String) {
    let binding = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let libmalloc = Path::new(&binding)
        .parent()
        .unwrap()
        .join("sample")
        .join("libmalloc.c");
    compile_lib(libmalloc);

    const SAMPLE_LIB_CFG: &str = "./sample/dependlibs.toml";
    const SAMPLE_TEST_CFG: &str = "./sample/tc_libmalloc.toml";

    {
        // write libs config
        let config_content = r#"
[[libs]]
path = "./sample/libmalloc.dll"
funcs = [
    { name = "my_malloc", paras = ["len", "mem_idx"] },
    { name = "my_free", paras = ["mem_idx"] },
    { name = "my_read32", paras = ["mem_idx", "offset"] },
    { name = "my_write32", paras = ["mem_idx", "offset", "val"] }
]
    "#;
        let mut file = File::create(SAMPLE_LIB_CFG).unwrap();
        let _ = file.write_all(config_content.as_bytes());
    }

    {
        let test_case = r#"
[[tests]]
name = "Test_Case_1"
cmds = [
    { opfunc = "my_malloc", expect_res = 0, args = ["len=4", "mem_idx=1"] },
    { opfunc = "my_write32", expect_res = 0, args = ["mem_idx=1", "offset=0", "val=888"] },
    { opfunc = "my_read32", expect_res = 888, args = ["mem_idx=1", "offset=0"] },
    { opfunc = "my_free", expect_res = 0, args = ["mem_idx=1"] }
]

[[tests]]
name = "Test_Case_2"
cmds = [
    { opfunc = "my_malloc", expect_res = 0, args = ["len=4", "mem_idx=2"] },
    { opfunc = "my_write32", expect_res = 0, args = ["mem_idx=2", "offset=4", "val=888"] },
    { opfunc = "my_read32", expect_res = 888, args = ["mem_idx=2", "offset=4"] },
    { opfunc = "my_read32", expect_res = 0, args = ["mem_idx=2", "offset=96"] },
    { opfunc = "my_free", expect_res = -12, args = ["mem_idx=5"] }
]        
    "#;
        let mut file = File::create(SAMPLE_TEST_CFG).unwrap();
        let _ = file.write_all(test_case.as_bytes());
    }
    (SAMPLE_LIB_CFG.into(), SAMPLE_TEST_CFG.into())
}
