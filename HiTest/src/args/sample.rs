use libparser::compile_lib;
use std::{fs::File, io::Write, path::Path, process::Command};

pub fn prepare_sample_files() -> (String, String) {
    let libmalloc = Path::new("./sample/libmalloc.c").to_path_buf();
    compile_lib(libmalloc, "./libs/");

    const SAMPLE_LIB_CFG: &str = "./cfgs/dependlibs.toml";
    const SAMPLE_TEST_CFG: &str = "./cfgs/tc_libmalloc.toml";

    {
        // 使用Python脚本生成lib配置
        let python_cmd = if cfg!(target_os = "windows") { "python" } else { "python3" };
        let target = if cfg!(unix) {
            "./libs/libmalloc.so"
        } else if cfg!(windows) {
            "./libs/libmalloc.dll"
        } else {
            panic!("Unsupported platform");
        };
        let status = Command::new(python_cmd)
            .arg("scripts/generate_config.py")
            .arg("-f")
            .arg("./sample/libmalloc.c")
            .arg("-o")
            .arg(SAMPLE_LIB_CFG)
            .arg("-l")
            .arg(target)
            .status()
            .expect("Failed to generate lib config");

        if !status.success() {
            panic!("Failed to generate lib config");
        }
    }

    {
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut test_case: String = r#"concurrences = [
{ tests = ["test_rw_u32", "Test_str_fill"], name = "group1" },
]

[[tests]]
name = "test_rw_u32"
thread_num=2
cmds = [
{ opfunc = "Call_malloc", expect_eq = 0, perf=true, args = ["len=$alloc_size", "mem_idx=1"] },
{ opfunc = "Call_write32", expect_eq = 0, perf=true, args = ["addr_idx=1", "val=$write_val"] },
{ opfunc = "Call_read32", expect_eq = "$write_val", perf=true, args = ["addr_idx=1"] },
{ opfunc = "Call_free", expect_eq = 0, perf=true, args = ["mem_idx=1"] },
]
inputs = [
{name = "ipt1", args = { alloc_size = "100", write_val = "888" } },
{name = "ipt2", args = { alloc_size = "200", write_val = "999" } },
{ args = { alloc_size = "50", write_val = "555" } }
]

[[tests]]
name = "test_rw_u32_ne"
thread_num=2
cmds = [
{ opfunc = "Call_malloc", expect_eq = 0, args = ["len=$alloc_size", "mem_idx=1"] },
{ opfunc = "Call_write32", expect_eq = 0, args = ["addr_idx=1", "val=$write_val1"] },
{ opfunc = "Call_read32", expect_eq = "$write_val1", args = ["addr_idx=1"] },
{ opfunc = "Call_write32", expect_eq = 0, args = ["addr_idx=1", "val=$write_val2"] },
{ opfunc = "Call_read32", expect_eq = "$!write_val1", args = ["addr_idx=1"] },
{ opfunc = "Call_free", expect_eq = 0, args = ["mem_idx=1"] },
]
inputs = [
{ args = { alloc_size = "100", write_val1 = "0x888", write_val2 = "444" } },
{ args = { alloc_size = "200", write_val1 = "0x999", write_val2 = "555" } },
{ args = { alloc_size = "50", write_val1 = "0x555", write_val2 = "666" } }
]

[[tests]]
name = "Test_str_fill"
thread_num=2
cmds = [
{ opfunc = "Call_malloc", expect_eq = 0, args = ["len=100", "mem_idx=1"] },
{ opfunc = "Call_strfill", expect_eq = 0, args = ["dst_addr=1", "content='abcdefg'", "len=7"] },
{ opfunc = "Call_malloc", expect_eq = 0, args = ["mem_idx=2", "len=8"] },
{ opfunc = "Call_memcpy", expect_eq = 0, args = ["src_idx=1", "dst_idx=2", "len=8"] },
{ opfunc = "Call_strcmp", expect_eq = 0, args = ["str1=1", "str2=2"] },
{ opfunc = "Call_mem_strlen", expect_eq = 7, args = ["str_idx=1"] },
{ opfunc = "Call_mem_strlen", expect_eq = 7, args = ["str_idx=2"] },
{ opfunc = "Call_strfill", expect_eq = 0, args = ["dst_addr=1", "content='casdaaasda'", "len=10"] },
{ opfunc = "Call_mem_strlen", expect_eq = "$second_len", args = ["str_idx=1"] },
{ opfunc = "Call_strcmp", expect_ne = 0, args = ["str1=1", "str2=2"] },
{ opfunc = "Call_free", expect_eq = 0, args = ["mem_idx=1"] },
{ opfunc = "Call_free", expect_eq = 0, args = ["mem_idx=2"] }
]
inputs = [
{ args = { second_len = "!7" } },
{ args = { second_len = "10" } }
]
"#
        .to_string();


        #[cfg(unix)]
        {
            test_case += r#"

[[tests]]
name = "test_dev_mem_page"
thread_num=1
cmds = [
{ opfunc = "Call_open", expect_eq = 0, args = ["pathname='/dev/mem'", "fd_idx=8"] },
{ opfunc = "Call_mmap", expect_eq = "$map_res", args = ["addr=0", "len=$len", "prot=3", "flags=5", "fd_idx=8", "offset=0", "addr_idx=11"] },
{ opfunc = "Call_write64", expect_eq = 0, args = ["addr_idx=11", "val=$val"] },
{ opfunc = "Call_read64", expect_eq = "$val", args = ["addr_idx=11"] },
{ opfunc = "Call_munmap", expect_eq = 0, args = ["addr_idx=11", "len=$len"] },
{ opfunc = "Call_close", expect_eq = 0, args = ["fd_idx=8"] },
]
inputs = [
{ args = { len = "8192", prot = "7", flags = "5", val = "0x44564" , map_res = "0"} },
{ args = { len = "81920", prot = "3", flags = "4", val = "13214", map_res = "0" } },
{ args = { len = "81920", prot = "2", flags = "1", val = "13214" , map_res = "0"} },
{ should_panic = false, args = { len = "8192", prot = "2", flags = "4", val = "44564" , map_res = "0"} },
]


[[tests]]
name = "test_dev_mem_page"
thread_num=1
cmds = [
{ opfunc = "Call_open", expect_eq = 0, args = ["pathname='/dev/mem'", "fd_idx=8"] },
{ opfunc = "Call_mmap", expect_eq = 0, args = ["addr=0", "len=$len", "prot=$prot", "flags=$flags", "fd_idx=8", "offset=0", "addr_idx=11"] },
{ opfunc = "Call_write64", expect_eq = 0, args = ["addr_idx=11", "val=$val"] },
{ opfunc = "Call_read64", expect_eq = "$val", args = ["addr_idx=11"] },
{ opfunc = "Call_munmap", expect_eq = 0, args = ["addr_idx=11", "len=$len"] },
{ opfunc = "Call_close", expect_eq = 0, args = ["fd_idx=8"] },
]
inputs = [
{ args = { len = "8192", prot = "7", flags = "5", val = "44564" } },
{ args = { len = "81920", prot = "3", flags = "4", val = "13214" } },
{ should_panic = true, args = { len = "8192", prot = "1", flags = "4", val = "44564" } },
]

    "#;
        }

        let mut file = File::create(SAMPLE_TEST_CFG).unwrap();
        let _ = file.write_all(test_case.as_bytes());
    }

    (SAMPLE_LIB_CFG.into(), SAMPLE_TEST_CFG.into())
}
