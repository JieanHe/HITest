# HITest

### 介绍
hitest是一个通用的读配置文件调接口校验返回值的工具。 与业务逻辑完全解耦，支持接口顺序调用，并发测试，死亡测试。
可用于SDK库、以及驱动程序的测试等。只需SDK开发人员提供一个待测SDK的warrper动态库文件和此库文件的配置文件，以及测试人员按照
配置文件的约定编写测试用例，即可使用hitest工具进行测试。

配置文件为toml格式，需两个配置文件（通过命令行参数提供）：
1. -l libs.toml: 指定库文件的路径和提供的所有函数。
2. -t test_case.toml: 指定接口调用的顺序和参数。
按顺序使用test_case.toml指定的参数和接口调用lib提供的接口并校验返回值。

**注意**
- 接口全部为 `int (*func)(const long *params, int para_len);`的形式。 参见 sample/libmalloc.c 和 sample/export_function.h.
- 如果库文件严格使用sample/export_function.h 中的宏来编写，可以使用scripts/gen_libs_config.py 来自动生成库文件配置文件。
- 用法：python3 scripts/gen_libs_config.py -f sample/libmalloc.c -o sample/libmalloc.toml [-l ./sample]
    - -l参数指定库文件的路径，默认为当前目录。
    - -f 指定库函数定义的C文件
    - -o 指定输出的配置文件路径。
### 库文件编写
库文件提供方需提供库的二进制文件和包含该二进制元信息的配置文件。库文件支持C/C++导出的*.so或者*.dll。
- **二进制文件**

二进制中将需要调的接口封装成`int *(func_name)(uint64_t *param_page, const uint64_t *params, int params_len) `的形式。
- param_page是测试程序预先分配的一个本线程独占的内存页，用于多个接口之间通信。
- params是测试程序提供的参数数组，一般是：
    - 魔鬼数字， 比如长度、param_page的dword索引等。
    - 字符串地址，当测试用例的参数使用 ''包裹时，测试程序会将字符串写入该地址。
- params_len是测试用例提供的参数的数量，库文件编写人员可以使用此长度对测试用例的合法性做一个简单校验。
SDK 库warrper的编写参见sample/libmalloc.c， 这是一个简单的libc warrper库，提供了一些常用的内存管理函数。

**注意** 如果库文件依赖了其他库文件，则编译库文件时必须显式指明依赖（如gcc 编译必须使用-lotherlib），例如：
假设库文件liba.so依赖了libb.so，则编译liba.so时需要使用-lb。


测试用例编写方需提供测试用例的配置文件和测试用例的二进制文件。
- **二进制文件**
    二进制文件需包含测试用例的入口函数，该函数的原型为：`int test_main(const char *config_path)`。

- **配置文件**
    配置文件需指明：
    1. warrper库的二进制文件路径。
    2. 二进制库所有导出函数的信息，包括函数名、参数数组中各个参数的名字。
    **注意** 参数数组中各个参数的名字必须按照参数的顺序依次给出，并且不能有重复的名字。
    并且含义需与测试用例编写人员约定一致。测试用例需要严格按照参数列表的参数名字提供调用函数的参数。
    可以运行 `hitest --sample` 之后，在sample目录下查看libmalloc.toml的内容。

#### 用例
用例需按照业务逻辑，以及与库约定的参数含义，指明调用库文件接口的顺序和参数，并指明期望的接口返回值。
配置文件需要是可以直接解析出如下数据结构的toml文件：
1. 普通测试用例，指明用例名称和需要调用的接口列表，以及调用每一个接口使用的参数列，预期相等的返回值或者预期不等的返回值。如：
```toml
[[tests]]
name = "test_rw_u32"
cmds = [
{ opfunc = "Call_malloc", expect_eq = 0, args = ["len=100", "mem_idx=1"] },
{ opfunc = "Call_write32", expect_eq = 0, args = ["addr_idx=1",  "val=888"] },
{ opfunc = "Call_read32", expect_eq = 888, args = ["addr_idx=1", ] },
{ opfunc = "Call_write32", expect_eq = 0, args = ["addr_idx=1",  "val=444"] },
{ opfunc = "Call_read32", expect_eq = 444, args = ["addr_idx=1", ] },
{ opfunc = "Call_free", expect_eq = 0, args = ["mem_idx=1"] },
]
```
一个用例内的cmds之间串行执行，可以通过参数break_if_fail来控制是否在失败时退出用例。
由于SDK时长涉及资源管理，容易导致程序崩溃，默认是子命令失败时直接退出用例。
也可以指定失败不退出，如下面的用例在Call_read32失败时会继续执行后面的用例：
```toml
[[tests]]
name = "test_rw_u32"
break_if_fail = false
cmds = [
{ opfunc = "Call_malloc", expect_eq = 0, args = ["len=100", "mem_idx=1"] },
{ opfunc = "Call_write32", expect_eq = 0, args = ["addr_idx=1",  "val=888"] },
# expect 444, but get 888, and break_if_fail == false, so will not break the test case
{ opfunc = "Call_read32", expect_eq = 444, args = ["addr_idx=1", ] },
{ opfunc = "Call_write32", expect_eq = 0, args = ["addr_idx=1",  "val=444"] },
{ opfunc = "Call_read32", expect_eq = 444, args = ["addr_idx=1", ] },
{ opfunc = "Call_free", expect_eq = 0, args = ["mem_idx=1"] },
]
```
2. 并发测试用例，支持两种模式，一种是同一个用例指定并发数，另一种是指定不同的测试用例之间并发。

    1. 用一个test并发，在test下指定thread_num参数即可，如：
    ```toml
    [[tests]]
    name = "test_rw_u32"
    thread_num = 100
    cmds = [
    { opfunc = "Call_malloc", expect_eq = 0, args = ["len=100", "mem_idx=1"] },
    { opfunc = "Call_write32", expect_eq = 0, args = ["addr_idx=1",  "val=888"] },
    { opfunc = "Call_read32", expect_eq = 888, args = ["addr_idx=1", ] },
    { opfunc = "Call_write32", expect_eq = 0, args = ["addr_idx=1",  "val=444"] },
    { opfunc = "Call_read32", expect_eq = 444, args = ["addr_idx=1", ] },
    { opfunc = "Call_free", expect_eq = 0, args = ["mem_idx=1"] },
    ]
    ```
    执行时将以100个线程并发执行test_rw_u32用例。

    2. 不同的Test之间并发，新增concurrences参数，指定并发的用例列表，如：
    ```toml
    concurrences = [
    { tests = ["test_rw_u32", "Test_str_fill"], name = "group1" },
    ]
    ```
    执行时将以不同的线程并发执行test_rw_u32和Test_str_fill用例, 注意，这里的用例名称必须与tests下的用例名称一致。

3. 死亡测试
    死亡测试用例，用于测试程序的崩溃情况，可以指定should_panic参数，当should_panic=true时，测试用例将被认为是一个死亡测试用例，
    会将测试用例单独放入子进程执行 ，如果子进程崩溃，则认为测试用例通过。
    如：
    ```toml
    [[tests]]
    name = "test_rw_u32"
    should_panic = true
    cmds = [
    { opfunc = "Call_malloc", expect_eq = 0, args = ["len=100", "mem_idx=1"] },
    { opfunc = "Call_write32", expect_eq = 0, args = ["addr_idx=1",  "val=888"] },
    { opfunc = "Call_free", expect_eq = 0, args = ["mem_idx=1"] },
    # use after free, it will panic here
    { opfunc = "Call_read32", expect_eq = 888, args = ["addr_idx=1", ] },
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
