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
          { name = "my_write32", paras = ["mem_idx", "offset", "val"] },
          { name = "my_strlen", paras = ["in_idx"]},
          { name = "my_strcmp", paras = ["in_srcidx", "in_dstidx", "len"]},
          { name = "my_memcpy", paras = ["in_srcidx", "in_dstidx", "len"]},
          { name = "my_strfill", paras = ["fill_idx", "content", "len"]},
      ]

      # If there are more libs, you can continue to add them as follows:
      # [[libs]]
      # path = "./other_dir/another_lib.dll"
      # funcs = [...]
    "#;
        #[cfg(unix)]
        let config_content = r#"
para_len = 4  # The default parameter length is 8,
    # which can be modified according to the actual situation of the lib,
    # but the length of all functions must not exceed this value.

[[libs]]
path = "./sample/libmalloc.so"
funcs = [
    { name = "my_malloc", paras = ["len", "mem_idx"] },
    { name = "my_free", paras = ["mem_idx"] },
    { name = "my_read32", paras = ["mem_idx", "offset"] },
    { name = "my_write32", paras = ["mem_idx", "offset", "val"] },
    { name = "my_strlen", paras = ["in_idx"]},
    { name = "my_strcmp", paras = ["in_srcidx", "in_dstidx", "len"]},
    { name = "my_memcpy", paras = ["in_srcidx", "in_dstidx", "len"]},
    { name = "my_strfill", paras = ["fill_idx", "content", "len"]},
    { name = "my_open_fd", paras = ["file_name", "out_fd_idx"] },
    { name = "my_close_fd", paras = ["in_fd_idx"] },
    { name = "my_mmap", paras = ["in_fd_idx", "out_addr_idx", "len"] },
    { name = "my_munmap", paras = ["in_addr_idx", "len"] },
]

# If there are more libs, you can continue to add them as follows:
# [[libs]]
# path = "./other_dir/another_lib.dll"
# funcs = [...]
"#;
        let mut file = File::create(SAMPLE_LIB_CFG).unwrap();
        let _ = file.write_all(config_content.as_bytes());
    }

    {
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut test_case:String = r#"
concurrences = [
    { tests = ["test_rw_u32", "Test_str_fill"], serial = false, name = "group1" },
    ]

[[tests]]
name = "test_rw_u32"
thread_num=100
cmds = [
    { opfunc = "my_malloc", expect_res = 0, args = ["len=100", "mem_idx=1"] },
    { opfunc = "my_write32", expect_res = 0, args = ["mem_idx=1", "offset=0", "val=888"] },
    { opfunc = "my_read32", expect_res = 888, args = ["mem_idx=1", "offset=0"] },
    { opfunc = "my_write32", expect_res = 0, args = ["mem_idx=1", "offset=0", "val=444"] },
    { opfunc = "my_read32", expect_res = 444, args = ["mem_idx=1", "offset=0"] },
    { opfunc = "my_free", expect_res = 0, args = ["mem_idx=1"] },
]

[[tests]]
name = "Test_str_fill"
thread_num=200
cmds = [
    { opfunc = "my_malloc", expect_res = 0, args = ["len=100", "mem_idx=1"] },
    { opfunc = "my_strfill", expect_res = 0, args = ["fill_idx=1", "content='abcdefg'", "len=7"] },
    { opfunc = "my_malloc", expect_res = 0, args = ["mem_idx=2", "len=8"] },
    { opfunc = "my_memcpy", expect_res = 0, args = ["in_srcidx=1", "in_dstidx=2", "len=8"] },
    { opfunc = "my_strcmp", expect_res = 0, args = ["in_srcidx=1", "in_dstidx=2", "len=7"] },
    { opfunc = "my_strlen", expect_res = 7, args = ["in_idx=1"] },
    { opfunc = "my_strlen", expect_res = 7, args = ["in_idx=2"] },
    { opfunc = "my_free", expect_res = 0, args = ["mem_idx=1"] },
    { opfunc = "my_free", expect_res = 0, args = ["mem_idx=2"] }
]
    "#.to_string();
        #[cfg(unix)]
        {
            let panic_test = r#"
[[tests]]
name = "test_write_panic"
thread_num=100
should_panic=true
cmds = [
    { opfunc = "my_malloc", expect_res = 0, args = ["len=100", "mem_idx=1"] },
    { opfunc = "my_write32", expect_res = 0, args = ["mem_idx=1", "offset=0", "val=888"] },
    { opfunc = "my_read32", expect_res = 888, args = ["mem_idx=1", "offset=0"] },
    { opfunc = "my_free", expect_res = 0, args = ["mem_idx=1"] },
    { opfunc = "my_write32", expect_res = 0, args = ["mem_idx=1", "offset=0", "val=444"] },
]

[[tests]]
name = "test_read_panic"
thread_num=100
should_panic=true
cmds = [
    { opfunc = "my_malloc", expect_res = 0, args = ["len=100", "mem_idx=1"] },
    { opfunc = "my_write32", expect_res = 0, args = ["mem_idx=1", "offset=0", "val=888"] },
    { opfunc = "my_read32", expect_res = 888, args = ["mem_idx=1", "offset=0"] },
    { opfunc = "my_free", expect_res = 0, args = ["mem_idx=1"] },
    { opfunc = "my_read32", expect_res = 888, args = ["mem_idx=1", "offset=0"] },
]
    
[[tests]]
name = "test_mmap"
thread_num=1
should_panic=false
cmds = [
    { opfunc = "my_open_fd", expect_res = 0, args = ["file_name='/dev/mem'", "out_fd_idx=1"] },
    { opfunc = "my_mmap", expect_res = 0, args = ["in_fd_idx=1", "out_addr_idx=2", "len=4096"] },
    { opfunc = "my_write32", expect_res = 0, args = ["mem_idx=2", "offset=0", "val=888"] },
    { opfunc = "my_read32", expect_res = 888, args = ["mem_idx=2", "offset=0"] },
    { opfunc = "my_munmap", expect_res = 0, args = ["in_addr_idx=2", "len=4096"] },
    { opfunc = "my_close_fd", expect_res = 888, args = ["in_fd_idx=1", "offset=0"] },
] 
"#;

            test_case.push_str(panic_test);
    }
        let mut file = File::create(SAMPLE_TEST_CFG).unwrap();
        let _ = file.write_all(test_case.as_bytes());
    }
    (SAMPLE_LIB_CFG.into(), SAMPLE_TEST_CFG.into())
}
