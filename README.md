# HITest

#### 介绍
hitest是一个通用的读配置文件调接口校验返回值的工具, 可用于SDK库、以及驱动程序的测试等。配置文件为toml格式，需两个配置文件（通过命令行参数提供）：
1. -l libs.toml: 指定库文件的路径和提供的所有函数。
2. -t test_case.toml: 指定接口调用的顺序和参数。
**注意** 接口全部为 `int (*func)(const long *params, int para_len);`的形式, 需要在libs.toml中指定parameter_len的最大长度(即所有库文件提供的所有函数中，parameters数组的最大长度，且library需保证每一个函数不会使用超过该长度的内存，否则会panic)**（例如： 当指定para_len=6时，库中的函数使用了params[6]则会panic）**

#### 库
库文件提供方需提供库的二进制文件和包含该二进制元信息的配置文件。库文件支持C/C++导出的*.so或者*.dll。
- **二进制文件**

    二进制中将需要调的接口封装成`int (*func)(const long *parameters, int parameter_len ); `的形式。建议parameters里面存放资源的位置（例如数组下标），来与测试用例通信，以及在多个接口之间传递资源等。

    以一个封装了libc的malloc、free的库libmalloc举例：
    ```C
    // libmalloc.c
    static uint64_t ADDR_ARR[MAX_ARR_LEN] = {0};
    int my_malloc(const long *params, int params_len)
    {
        long len = params[0];
        long out_idx = params[1];
        void *addr = malloc(len);
        if (!addr)
            return -22;

        ADDR_ARR[out_idx] = (uint64_t)addr;
        return 0;
    }

    int my_free(const long *params, int params_len)
    {
        long in_idx = params[0];
        if (ADDR_ARR[in_idx] == 0)
        {
            printf("invalid input idx %d\n", in_idx);
            return -12;
        }

        free((void *)ADDR_ARR[in_idx]);
        ADDR_ARR[in_idx] = 0;
        return 0;
    }
    ```

- **配置文件** 
    配置文件需指明： 
    1. para_len: 最多参数数量,所用库文件共用，**库本身需保证不会使用超出para_len个params的元素。（例如： 当指定para_len=6时，库中的函数使用了paras[6]则会panic）**
    2. 库文的具体信息，包括文件路径和所有导出函数以及参数数组中各个元素的含义。
    其中配置文件需要为可以直接解析出如下数据结构的toml文件：
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

        例如libmalloc的配置文件应该为：
        ```toml
        para_len = 2           
        [[libs]]
        path = "./libmalloc.dll"
        funcs = [
            { name = "my_malloc", paras = ["len", "mem_idx"] }, # 库文件内， params[0]为len， params[1]为mem_idx，
            { name = "my_free", paras = ["mem_idx"] }, # 库文件内，params[0]为mem_idx
            ， # 其他函数
        ]
        # 其他库文件
        # [[libs]]
        # path = "other_path"
        # funcs = [ ... ]
        ```

#### 用例
用例需按照业务逻辑，以及与库约定的参数含义，指明调用库文件接口的顺序和参数，并指明期望的接口返回值。
配置文件需要是可以直接解析出如下数据结构的toml文件：

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

用例通过opfunc指定调用的接口，params字段指定参数。其中，参数需要为"param_name=param_value"的形式。例如：
```toml
[[tests]]
name = "Test_Case_1"
cmds = [
    { opfunc = "my_malloc", expect_res = 0, args = ["len=4", "mem_idx=1"] }, # 申请4bytes内存，首地址存入mem_idx=1的数组，预期返回0代表成功
    { opfunc = "my_free", expect_res = 0, args = ["mem_idx=1"] } # 从mem_idx=1处取出一个地址，释放该地址对应内存，预期返回0代表成功
]

[[tests]]
name = "Test_Case_2"
cmds = [
    { opfunc = "my_malloc", expect_res = 0, args = ["len=100", "mem_idx=2"] }, # 申请100bytes内存，首地址存入mem_idx=2的数组，预期返回0代表成功
    { opfunc = "my_free", expect_res = -12, args = ["mem_idx=5"] }  # 从mem_idx=5处取出一个地址，释放该地址，预期返回-12代表失败
    { opfunc = "my_free", expect_res = 0, args = ["mem_idx=1"] }  # 从mem_idx=1处取出一个地址，释放该地址，预期返回0代表成功
]
```

#### 使用说明
可以安装rust后重新编译运行，也可以直接使用构建好的二进制直接运行。

命令行参数：
- -h<--help>           显示帮助信息
- --sample             运行libmalloc的例子
- -l, <--lib>          指定库文件的配置文件
- -t, <--test>         指定用例的配置文件
- --log [LEVEL]        设置日志级别（info, debug, error，默认为info）

#### 参与贡献

1.  Fork 本仓库
2.  新建 Feat_xxx 分支
3.  提交代码
4.  新建 Pull Request

#### 版本更新