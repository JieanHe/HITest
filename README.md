# HITest

#### 介绍
一个SDK库的集成测试工具, 从配置文件读取SDK库提供的所有对外接口以及测试用例，执行用例并判别是否执行成功。
- **SDK开发人员需提供**
    1. 一个动态库*.so或*.dll：  将SDK库每一个待测接口都封装为8个i64输入并返回i32的接口并导出，无特殊约定时以返回0代表成功。
    - 例如libmalloc.dll是一个封装了libc的malloc和free的demo：
        ```C
        int my_malloc(int64_t len, int64_t out_idx, int64_t p2, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7);
        int my_free(int64_t in_idx, int64_t offset, int64_t p2, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7);
        int my_read32(int64_t in_idx, int64_t offset, int64_t p2, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7);
        int my_write32(int64_t in_idx, int64_t offset, int64_t val, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7);
        ```
    2. 一个配置文件*.toml: 包含该动态库提供的所有接口以及接口的有效参数。
    - 配置文件对应的数据结构：
        ```Rust
        struct LibFunc {
            name: String,
            paras: Vec<String>,
        }

        struct Lib {
            path: String,
            funcs: Vec<LibFunc>,
        }

        struct LibConfig {
            libs: Vec<Lib>,
        }

        ```
    
    - 例如libmalloc.dll配置文件的例子如下

        ```toml
        [[libs]]
        path = "./sample/libmalloc.dll"
        funcs = [
            { name = "my_malloc", paras = ["len", "mem_idx"] }, 
            { name = "my_free", paras = ["mem_idx"] },
            { name = "my_read32", paras = ["mem_idx", "offset"] },
            { name = "my_write32", paras = ["mem_idx", "offset", "val"] }
        ]
        ```

- **测试人员需提供**
    1. 测试用例配置文件： 按业务逻辑调用library提供的接口，并指定预期返回值。
    - 测试用例对应数据结构如下：
        ```Rust
        struct Cmd {
            opfunc: String,
            expect_res: i32,
            args: Vec<String>,
        }
        struct Test {
            name: String,
            cmds: Vec<Cmd>,
        }
        struct Config {
            tests: Vec<Test>,
        }
        ```
    - 测试用例通过opfunc指定调用的接口，params字段指定参数。其中，参数需要为"param_name=param_value"的形式。例如：
        ```toml
        [[tests]]
        name = "Test_Case_1"
        cmds = [
            { opfunc = "my_malloc", expect_res = 0, args = ["len=4", "mem_idx=1"] }, # 申请4bytes内存，首地址存入mem_idx=1的数组，预期返回0代表成功
            { opfunc = "my_write32", expect_res = 0, args = ["mem_idx=1", "offset=0", "val=888"] }, # 从mem_idx=1处取出一个地址，向该地址偏移0byte的地址写入32bit，值为888，预期返回0代表成功
            { opfunc = "my_read32", expect_res = 888, args = ["mem_idx=1", "offset=0"] }, # 从mem_idx=1处取出一个地址，从该地址读出32bit，预期读取到888
            { opfunc = "my_free", expect_res = 0, args = ["mem_idx=1"] } # 从mem_idx=1处取出一个地址，释放该地址对应内存，预期返回0代表成功
        ]

        [[tests]]
        name = "Test_Case_2"
        cmds = [
            { opfunc = "my_malloc", expect_res = 0, args = ["len=100", "mem_idx=2"] }, # 申请100bytes内存，首地址存入mem_idx=2的数组，预期返回0代表成功
            { opfunc = "my_free", expect_res = -12, args = ["mem_idx=5"] }  # 从mem_idx=5处取出一个地址，释放该地址，预期返回-12代表失败
        ]
        ```

#### 使用说明

- `hitest -h` 查看帮助
- `hitest --sample` 运行libmalloc的测试用例，可以在./sample下看到动态库封装方式、以及配置文件编写格式
- `hitest -l libs.toml -t test_case.toml` 从libs.toml中获取动态库信息，从test_case.toml中获取测试用例，执行测试用例
- `hitest -l libs.toml -t test_case.toml --log debug` 从libs.toml中获取动态库信息，从test_case.toml中获取测试用例，执行测试用例, 日志级别为debug级（允许的日志界别有: info, debug, error, 默认的info级别）

#### 参与贡献

1.  Fork 本仓库
2.  新建 Feat_xxx 分支
3.  提交代码
4.  新建 Pull Request