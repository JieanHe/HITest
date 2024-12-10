use libparser::compile_lib;
use std::{fs::File, io::Write, path::Path};

pub fn prepare_sample_files() -> (String, String) {
    let libmalloc = Path::new("./sample/libmalloc.c").to_path_buf();
    compile_lib(libmalloc);

    const SAMPLE_LIB_CFG: &str = "./sample/dependlibs.toml";
    const SAMPLE_TEST_CFG: &str = "./sample/tc_libmalloc.toml";

    {
        // write libs config
        #[cfg(windows)]
        let config_content = r#"
para_len = 3  # The default parameter length is 8, 
  # which can be modified according to the actual situation of the lib, 
  # but the length of all functions must not exceed this value.

[[libs]]
path = "./sample/libmalloc.dll"
funcs = [
    { name = "my_malloc", paras = ["len", "mem_idx"] },
    { name = "my_free", paras = ["mem_idx"] },
    { name = "my_read32", paras = ["mem_idx", "offset"] },
    { name = "my_write32", paras = ["mem_idx", "offset", "val"] }
]

# If there are more libs, you can continue to add them as follows:
# [[libs]]
# path = "./other_dir/another_lib.dll"
# funcs = [...]
    "#;
    #[cfg(unix)]
    let config_content = r#"
para_len = 3  # The default parameter length is 8, 
# which can be modified according to the actual situation of the lib, 
# but the length of all functions must not exceed this value.

[[libs]]
path = "./sample/libmalloc.so"
funcs = [
{ name = "my_malloc", paras = ["len", "mem_idx"] },
{ name = "my_free", paras = ["mem_idx"] },
{ name = "my_read32", paras = ["mem_idx", "offset"] },
{ name = "my_write32", paras = ["mem_idx", "offset", "val"] }
]

# If there are more libs, you can continue to add them as follows:
# [[libs]]
# path = "./other_dir/another_lib.so"
# funcs = [...]
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
    { opfunc = "my_free", expect_res = -12, args = ["mem_idx=5"] }
]        
    "#;
        let mut file = File::create(SAMPLE_TEST_CFG).unwrap();
        let _ = file.write_all(test_case.as_bytes());
    }
    (SAMPLE_LIB_CFG.into(), SAMPLE_TEST_CFG.into())
}
