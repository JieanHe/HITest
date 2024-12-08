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
            {opfunc = "my_malloc", expect_res = 0,  args = [4, 1, 0, 0, 0]}, #申请四个字节内存,首地址存入地址数组的idx1,预期成功返回0
            {opfunc = "my_write32", expect_res = 0,  args = [1, 888]}, # 向地址数组的idx1写入魔鬼数字888,预期成功返回0
            {opfunc = "my_read32", expect_res = 888,  args = [1]}, # 从地址数组idx1读回4个字节,预期读到888
            {opfunc = "my_free", expect_res = 0, args = [1]}, # 释放地址数组的idx1对应内存,预期成功返回0
        ]
        name = "Test_Case_1"
        
        
        [[tests]]
        cmds = [
            {opfunc = "my_malloc", expect_res = 0,  args = [100, 4, 0, 0, 0]},  #申请100个字节内存,首地址存入地址数组的idx4,预期成功返回0
            {opfunc = "my_write32", expect_res = 0,  args = [4, 888]}, # 向地址数组的idx4写入魔鬼数字888,预期成功返回0
            {opfunc = "my_read32", expect_res = 888,  args = [4]}, # 从地址数组idx4读回4个字节,预期读到888
            {opfunc = "my_free", expect_res = -12, args = [1]}, # # 释放地址数组的idx1对应内存,预期失败返回-12
        ]
        name = "Test_Case_2"            
    "#;
        let mut file = File::create(SAMPLE_TEST_CFG).unwrap();
        let _ = file.write_all(test_case.as_bytes());
    }
    (SAMPLE_LIB_CFG.into(), SAMPLE_TEST_CFG.into())
}
