use std::{env, fs::File, io::Write, path::Path};
use libparser::compile_lib;

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
    libs = [
        {path = "./sample/libmalloc.dll", funcs = ["my_malloc", "my_free", "my_read32", "my_write32"]},
    ]"#;
        let mut file = File::create(SAMPLE_LIB_CFG).unwrap();
        let _ = file.write_all(config_content.as_bytes());
    }

    {
        let test_case = r#"
[[tests]]
cmds = [
    {opfunc = "my_malloc", expect_res = 0,  args = [4, 1, 0, 0, 0]},
    {opfunc = "my_write32", expect_res = 0,  args = [1, 888]},
    {opfunc = "my_read32", expect_res = 888,  args = [1]},
    {opfunc = "my_free", expect_res = 0, args = [1]},
]
name = "Test_Case_1"


[[tests]]
cmds = [
    {opfunc = "my_malloc", expect_res = 0,  args = [100, 4, 0, 0, 0]},
    {opfunc = "my_write32", expect_res = 0,  args = [4, 888]},
    {opfunc = "my_read32", expect_res = 888,  args = [4]},
    {opfunc = "my_free", expect_res = -12, args = [1]},
]
name = "Test_Case_2"           
    "#;
        let mut file = File::create(SAMPLE_TEST_CFG).unwrap();
        let _ = file.write_all(test_case.as_bytes());
    }
    (SAMPLE_LIB_CFG.into(), SAMPLE_TEST_CFG.into())
}
