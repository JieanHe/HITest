# HiTest 测试框架

## 概述

hitest是一个通用的读配置文件调接口校验返回值的工具。 与业务逻辑完全解耦。支持按照配置文件指定的接口顺序调用指定接口并校验返回值，支持并发测试，性能测试，死亡测试。可用于SDK库、以及驱动程序的测试等。

业务SDK开发人员你需要提供

- 一个wrarper库， 包含待测SDK的所有待测接口，并封装成统一的形式： `s64 *(api_name)(u64 *param_page, s64 *params, s64 param_len)`. 其中
  - `param_page` 是框架提供的线程本地内存，可容纳64个DWORD，用于各个API之间交换信息。
  - `prarams` 是测试用例调用时从配置文件获取的参数
  - `param_len` 是测试用例提供的参数数量
  - 返回值为64位有符号数
- 一份wrarper库配置文件，指明包装的接口名，和所有参数。该配置文件需要为toml格式。

测试用例开发人员需要

- 与SDK开发人员确定各个参数的含义，进一步确定调用API时如何给参数
- 依据业务逻辑编写测试用例, 测试用例需要为toml格式。

### 核心名词解释

- Condition： 对接口调用返回值的断言，允许使用：

  - expect_eq  断言API返回值等于
  - expect_ne  断言API返回值不等于
- Cmd：Cmd是对一个待测接口的最小封装，指定一个API的调用参数和预期返回值。有如下属性：

  - opfunc:  调用接口的名称，来自SDK开发人员提供的wrarper库。
  - args: 参数列表， 每一个参数使用 "\$parm\_name=\$pram_val"的形式, 其中param_val可以是
    - 任意数字  输入的数字，比如memset需要填充的初始值
    - 数组下标
      - 当API需要向其他API输出信息时，wrarper库需要将资源地址存入param_page的该下标。
      - 当API需要从其它API获取信息时，wrarper库需要从pram_page的该下标处获取资源地址。
    - 字符串  也是作为纯输入，输入字符串时用单引号将字符串内容包裹起来。如 `"str_param='a str demo'"`
  - perf: 是否统计性能，当此字段设为true时，框架会统计调用opfunc指向的API的耗时并report出来。
- Test： Test是一个测试用例存在，内含有一组Cmd。有如下属性:

  - name:   <必须>测试用例名，用于report信息
  - cmds： <必须>一组Cmd的列表，指定调用API的顺序。
  - thread_num：<可选> 启用多少个线程运行，不指定时默认为1
  - should_panic: <可选> 改Test是否预期会Crash，不指定时默认为false
  - break_if_fail: <可选> cmds组中某一个Cmd执行失败是否打断后续cmd执行。不指定时默认为1
  - inputs： 高级功能，允许使用多组输入参数。
  - ref_inputs： 高级功能，允许在cmds的头和尾增加其他Cmd 列表做资源的初始化和清理。
- Env： Env是一个多个测试用例公共的资源初始化和资源释放Cmd列表的封装。包含

  - name:  一个字符串， 用于report信息
  - init：一个Cmd的列表，存放资源初始化的Cmds
  - exit：一个Cmd列表，存放资源注销的Cmds
  - tests：一个字符串列表，存放需要使用此env的Test.name.注意：

    - Env.test 是空数组时代表这是个全局Env，本配置文件的所有Test调用前都会先执行此Env的init，所有Test执行完自己的最后一个Cmd之后都会执行此Env的exit。
    - 全局env只允许一个
    - 一个Test至多使用一个全局Env和一个局部Env，全局env的init最先执行，且exit最后执行
    - 当使用Env时，Test内需要注意不要使用到所用Env使用到的资源下标。
- InputGroup： 输入参数的封装，用于不同输入执行同一个用例的场景。含有如下字段

  - name： 一个字符串，用于引用report信息
  - args： 包含多个键值对的哈希表，指定各个参数名对应的参数值。
  - shoud_panic： 使用这一组参数是否会crash，当设置为true，应用这一组输入时会将Test的should_panic属性设为true。
  - break_if_fail：使用这一组参数当某个cmd执行失败是否打断后续Cmd执行。
- Config： 一个完整的测试用例配置文件被包装成一个Config。含有如下字段

  - tests：<必须> 包含多个Test的列表
  - envs：<可选> 包含多个Env的列表。不配置时默认无Env
  - shared_inputs： <可选> 包含多个Input，用于在多个测试用例之间共享Input组
  - concurrences: <可选> 可以指定多个并发组，一个并发组内是多个需要并发执行的Test的name。

## 核心功能

- **参数化测试用例支持** 可以依据配置文件load待测业务的so，并依据测试用例指定的接口和调用配置调用这些接口，所有用例的新增修改无需修改代码。
- **顺序接口测试** 可以将所需调用的接口组装成一个个Cmd按照业务逻辑放到一个Test中，按照Test中的顺序按需调用这些接口并校验返回值。
- **多线程并发测试**  支持某个测试用例以指定线程数并发执行，以及将多个测试用例放到某一个并发组内并发的执行。
- **高并发度测试** 接口之间传递参数使用线程本地内存，多线程测试场景下，测试框架本身无锁。
- **性能测试**  支持调用接口时指定`perf`参数, 这将会统计出调用此接口花费的时间。
- **测试环境配置** 可以将多个测试用例的公共init和exit部分放到

## SDK Wrapper编写

### 基础写法

HiTest内部存储了 API名-函数指针  的哈希表用来实现执行配置文件指定的操作，所以所有函数指针都需要满足`s64*(func_name)(u64 *param_page, const u64 *params, s64 params_len)` 的形式。

- `param_page` 是框架提供的线程本地内存，可容纳64个DWORD，用于各个API之间交换信息。
- `prarams` 是测试用例调用时从配置文件获取的参数
- `param_len` 是测试用例提供的参数数量

以libc的malloc和free为例说明：

```c
long Call_malloc(uint64_t *param_page, const uint64_t *params, long param_len)
{
    if (param_len != 2)
    {
        printf("this function need 2 param!\n");
        return -1;
    }
    size_t len = (size_t)params[0];
    int mem_idx = (int)params[1];
    void *ptr = malloc(len);
    if (!ptr)
    {
        printf("malloc failed!\n");
        return -1;
    }
    param_page[mem_idx] = (uint64_t)(uint64_t)ptr; // output memory address to param_page
    return 0;
}

long Call_free(uint64_t *param_page, const uint64_t *params, long param_len)
{
    if (param_len != 1)
    {
        printf("this function need 1 param!\n");
        return -1;
    }
    int __idx = params[0];
    void *ptr = (void *)param_page[__idx]; // get memory address from param_page
    if (ptr == 0)
    {
        printf("input idx[%d] got null ptr \n", "ptr", __idx);
        return -1;
    }
    free(ptr);
    return 0;
}


```

当编写完此wrarper后，还需提供配置文件指明这个库的所有导出函数名和各个参数的含义。配置文件为toml格式，好函一个Library列表。 Library有两个字段：

- path： wrarper库文件的路径
- funcs：所有导出函数的列表。其中funcs的每一个func又有如下字段：
  - name：一个字符串，是API的函数名
  - paras： 一个字符串列表，调用该API需要指明的参数名列表

比如上面的例子中Call_malloc使用了两个参数，第一个是输出内存下标 out_mem_idx，第二个是申请内存的长度 alloc_size。Call_free使用了一个参数，是param_page内malloc输出的内存地址所在位置in_mem_idx。于是就可以做如下配置文件

```toml
# libc_wrarper.toml
[[libs]]
path = "libc_wrarper.so"
funcs = [
    { name = "Call_malloc", paras = [
        "alloc_size",
        "out_mem_idx",
    ] },
    { name = "Call_free", paras = [
        "in_mem_idx",
    ] },
]
```

这里的参数名，尽量按照API使用的真正含义来命名，但不强制。只要与测试用例编写人员约定好即可。调用这两个API的测试用例见[这里](https://)

### 快速编写wrarper库

由于编写此库的所有函数有非常相似的代码，为避免重复工作，在sample目录下提供了一个exprot_function.h。 这里面定义了一些方便的宏。并且在scripts内提供了generate_config.py， 使用exprot_function.h下面提供的EXPORT_FUNC宏定义的API可以使用python运行这个脚本自动生成对应的配置文件。

#### export_function.h

提供了如下宏：

- EXPORT_FUNC(func_name, param1, param2, ...) 定义一个满足要求的导出函数， param是一个逗号分隔的参数列表标识符， 测试用例需要使用相同的标识符传递， 导出的函数会自动在函数签名加上前缀‵Call_`比如：

  ```c
  EXPORT_FUNC(malloc, len, mem_idx)  // 定义一个导出函数Call_malloc， 该函数需要两个参数， 第一个参数是len， 第二个参数是mem_idx。
  // 对应测试用例为： { opfunc = "Call_malloc", expect_eq = 0, args = ["len=100", "mem_idx=1"] }, 这里的100和1是参数的值， 由测试用例编写者确定
  ```
- `CHECK_PARAM_LEN(len)` 检查参测试用例的参数数量是否等于len， 如果不等于则返回-12。
- `GET_INPUT_IDX_NZ(type, name, param_idx)` 获取测试用例的参数中的第param_idx个参数的值， 这个数值要求非0， 且必须是其他导出函数写入之后的。
- `GET_INPUT_IDX(type, name, param_idx)` 获取测试用例的参数中的第param_idx个参数的值， 这个数值可以是0， 也是其他导出函数写入的。
- `GET_VALUE(type, name, param_idx)` 获取测试用例的参数中的name参数的值， 这个数值是由测试用例直接指定的。
- `SET_OUTPUT_IDX(idx, value)` 把输出数值写入到共享内存页`param_page`的第idx个dword中。用于给其他导出函数使用。具体这些宏的使用可以参考sample/export_function.h和sample/libmalloc.c

#### generate_configs.py

一个自动解析EXPORT_FUNC宏生成toml配置文件的python脚本，可以适配运行平台和条件编译命令。使用方法：

```bash
python generate_config.py -f src_file.c -o output_name.toml -l library_name
```

注意： 测试用例需要严格按照参数列表的参数名字提供调用函数的参数。 可以运行 `hitest --sample` 之后，在cfgs目录下查看tc_libmalloc.toml的内容, 这个文件是一个简单的测试用例。

## 测试用例编写

用例需按照业务逻辑，以及与库约定的参数含义，指明调用库文件接口的顺序和参数，并指明期望的接口返回值。
配置文件需要是可以直接解析出如下数据结构的toml文件：

### 普通测试用例

普通测试用例指明用例名称和需要调用的接口列表，以及调用每一个接口使用的参数列，预期相等的返回值或者预期不等的返回值。如：

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
# expect not equal to 888 but get 888, and break_if_fail == false, so will not break the test case
{ opfunc = "Call_read32", expect_ne = 888, args = ["addr_idx=1", ] },
{ opfunc = "Call_write32", expect_eq = 0, args = ["addr_idx=1",  "val=444"] },
{ opfunc = "Call_read32", expect_eq = 444, args = ["addr_idx=1", ] },
{ opfunc = "Call_free", expect_eq = 0, args = ["mem_idx=1"] },
]
```

**注意**： expect_eq和expect_ne必须指定其中一个且不能同时指定。

### 并发测试用例

支持两种并发模式，一种是同一个用例指定并发数，另一种是指定不同的测试用例之间并发。

1. 用一个test并发，在test下指定thread_num参数即可，如线面的用例在执行时将以100个线程并发执行test_rw_u32用例。

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

2. 不同的Test之间并发，在全局使用concurrences参数指定并发的用例列表，如下面的配置执行时将以不同的线程并发执行test_rw_u32和Test_str_fill用例然后用不同的线程并发执行test1和test2用例。

```toml
concurrences = [
{ tests = ["test_rw_u32", "Test_str_fill"], name = "group1" },
{ tests = ["test1", "test2"], name = "group2"},
]
```

**注意**

1. 这里的用例名称必须与tests下的用例名称一致。
2. 放到concurrences的用例只在concurrency环境下执行，不会另外执行。

### 死亡测试

用于测试程序的崩溃情况，可以指定should_panic参数，当should_panic=true时，测试用例将被认为是一个死亡测试用例，
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

### 性能测试

可以在cmds中需要统计性能的cmd内指定perf=true, 此时会报告该cmd的执行时间。

```toml
[[tests]]
name = "test_rw_u32"
should_panic = true
cmds = [
{ opfunc = "Call_malloc", expect_eq = 0, perf=true, args = ["len=100", "mem_idx=1"] },
{ opfunc = "Call_write32", expect_eq = 0, perf=true, args = ["addr_idx=1",  "val=888"] },
{ opfunc = "Call_read32", expect_eq = 888,  perf=true, args = ["addr_idx=1", ] },
{ opfunc = "Call_free", expect_eq = 0, perf=true, args = ["mem_idx=1"] },
]
```

**注**： 可以向C库直接输入C风格字符串，方法为参数的‘=’后面用单引号包裹想输入的字符串。 参见sample模式的`Test_str_fill`。

### 多组输入测试

可以在Test中新增通过inputs参数指定多组输入，并且在cmds中使用${para_name}来引用输入参数。如此可以复用同一组Cmd的调用顺序配置。
每一组参数可以指定多个输入参数，以及本组参数的组名用于报告执行结果，参数值支持单个值、指定列表和列表生成式。

- 单个值， 在cmd中的参数使用在cmds中使用${para_name}来引用输入参数， inputs的args里面给定param_name=“param_value”即可。

```toml
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
{ args = { alloc_size = "200", write_val = "999" } },   # 自动命名为default1
{ args = { alloc_size = "50", write_val = "555" } }    # 自动命名为default2
]
```

- 指定列表。 当inputs的args里面参数给了列表时，列表里面的每一个元素会被应用到cmd里面，生成独立的test备份，这些不同参数的test会被并发的执行。比如下面的例子会有三个Test并发执行，他们分别使用不同的off参数

```toml
[[tests]]
name = "test_rw_u32"
thread_num=2
cmds = [
{ opfunc = "Call_malloc", expect_eq = 0, perf=true, args = ["len=$alloc_size", "mem_idx=1"] },
{ opfunc = "Call_write32", expect_eq = 0, perf=true, args = ["addr_idx=1", "val=12345678", off="$off"] },
{ opfunc = "Call_read32", expect_eq = "12345678", perf=true, args = ["addr_idx=1", off="$off"] },
{ opfunc = "Call_free", expect_eq = 0, perf=true, args = ["mem_idx=1"] },
]
inputs = [
{name = "ipt1", args = { alloc_size = "100", off= ["0", "50", "96" ]} },
]
```

- 列表生成式，可以使用 start,end, step三个参数来指定生成一个参数序列，序列中的每一个参数都会被应用到cmds里面生成多个test备份，这些test会被并发执行。比如下面的例子将会生成0，10，20，30，40，50，60，70，80，90的off序列，进一步生成10个test实例并发执行。

```toml
[[tests]]
name = "test_rw_u32"
thread_num=2
cmds = [
{ opfunc = "Call_malloc", expect_eq = 0, perf=true, args = ["len=$alloc_size", "mem_idx=1"] },
{ opfunc = "Call_write32", expect_eq = 0, perf=true, args = ["addr_idx=1", "val=12345678", off="$off"] },
{ opfunc = "Call_read32", expect_eq = "12345678", perf=true, args = ["addr_idx=1", off="$off"] },
{ opfunc = "Call_free", expect_eq = 0, perf=true, args = ["mem_idx=1"] },
]
inputs = [
{name = "ipt1", args = { alloc_size = "100", off= {start=0, end=96, step=10}} },
]
```

**注意**： 需要与其他cmd进行通信的idx位置，最好在cmds中指定而不要放到inputs里面，否则你需要非常小心的保证idx的正确性，当cmds很多的时候这会变得不好维护。

### 预设环境测试

可以在config中通过envs参数指定预设环境，以及使用此环境的所有测试用例列表。以此来复用公共的资源创建和注销Cmd列表。使用env的测试用例在执行时会先执行env的init内的Cmds, 并且Test自己的Cmds执行完之后会执行Env的exit下的Cmds。

```toml
[[envs]]
name = "memory_env"
init = { opfunc = "Call_malloc", expect_eq = 0, args=["len=10000", "mem_idx=50"] }
exit = { opfunc = "Call_free", expect_eq = 0, args=["mem_idx=50"] }
tests = ["test_rw_u32", "test_rw_u32_ne"]
```

**注意**

- env使用的idx与属于测试用例线程，保证tests列表的测试用例使用的idx不会与init和exit使用的冲突。
- 当env的tests列表为空时，这个Env为全局env，所有Test都会应用这个Env，一份配置文件内全局env至多一个。
- 可以同时使用全局env和非全局的env，这是全局env最先初始化最后注销。但是不允许同时使用两个非全局的env。

### 复用输入参数组

InputGroup也支持复用， 使用方法是在配置文件顶层新增`shared_inputs`字段，这是个哈希表，每一个key对应一个InputGroup列表。然后再需要使用这个输入组的用例内，通过ref_inputs参数引用这个key即可。如：

```toml
[shared_inputs]
common1 = [
    { name = "ipt1", args = {  write_val = "888", off = "0x0" } },
    { name = "ipt2", args = {  write_val = "999", off = "0x10" } },
]
common2 = [
    { name = "ipt3", args = {  write_val = "555", off = "0x100" } }
]

[[tests]]
name = "test_rw_u32"
thread_num = 2
cmds = [
    { opfunc = "Call_write32", expect_eq = 0, args = [
        "addr_idx=0",
        "val=$write_val",
        "off=$off",
    ] },
    { opfunc = "Call_read32", expect_eq = "$write_val", args = [
        "addr_idx=0",
        "off=$off",
    ] },

]
inputs = [ { name = "ipt4", args = { alloc_size = "0x1000", write_val = "888", off = "0xffc" } }]
ref_inputs = ["common1", "common2"]
```

这个用例会依次想addr_idx=0代表的内存偏移0处写入888并都会，向偏移0x10处写入999并读回，向偏移0x100处写入555并都会，向偏移0xffc处写入888并读回来。


## 使用说明

可以安装rust后重新编译运行，也可以直接使用构建好的二进制直接运行。

命令行参数：

- -h<--help>           显示帮助信息
- --sample             运行libmalloc的例子
- -i, <--lib>          指定库文件的配置文件
- -t, <--test>         指定用例的配置文件
- -l [LEVEL]        设置日志级别（error，warn, info, debug, 或 1 2 3 4 默认为info(3)）

## 参与贡献

1. Fork 本仓库
2. 新建 Feat_xxx 分支
3. 提交代码
4. 新建 Pull Request

## 版本更新
