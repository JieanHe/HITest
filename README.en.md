# HITest

#### Introduction
An integration test tool for SDK libraries, which reads all external interfaces provided by the SDK library from a configuration file, executes test cases, and determines whether they are executed successfully.
- **SDK developers need to provide:**

    1. A dynamic library *.so or *.dll: Each interface to be tested in the SDK library is encapsulated as an interface with 8 i64 inputs and returns i32, and exported. Returns 0 by default to represent success.
    - For example, libmalloc.dll is a demo that encapsulates the malloc and free functions from the C standard library.
        ```C
        int my_malloc(int64_t len, int64_t out_idx, int64_t p2, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7);
        int my_free(int64_t in_idx, int64_t offset, int64_t p2, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7);
        int my_read32(int64_t in_idx, int64_t offset, int64_t p2, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7);
        int my_write32(int64_t in_idx, int64_t offset, int64_t val, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7);
        ```
    2. A configuration file *.toml: Contains all interfaces provided by the dynamic library and valid parameters for the interfaces. 
    - The corresponding data structure of the configuration file is:
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
    - An example of the configuration file for libmalloc.dll is as follows:
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

- **Testers need to provide:**
    1. Test case configuration file: Call the interfaces provided by the library according to business logic and specify the expected return value. 
    - The corresponding data structure for the test cases is as follows:
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
    - Test cases specify the interface to call via opfunc, and the params field specifies the parameters. The parameters need to be in the form of "param_name=param_value". For example:
        ```toml
        [[tests]]
        name = "Test_Case_1"
        cmds = [
            { opfunc = "my_malloc", expect_res = 0, args = ["len=4", "mem_idx=1"] }, # Apply for 4bytes of memory, store the first address in the array with mem_idx=1, and expect to return 0 to represent success
            { opfunc = "my_write32", expect_res = 0, args = ["mem_idx=1", "offset=0", "val=888"] }, # Take out an address from mem_idx=1, write 32bit, value 888 to the address offset 0, expect to return 0 to represent success
            { opfunc = "my_read32", expect_res = 888, args = ["mem_idx=1", "offset=0"] }, # Take out an address from mem_idx=1, read out 32bit from the address, expect to read 888
            { opfunc = "my_free", expect_res = 0, args = ["mem_idx=1"] } # Take out an address from mem_idx=1, release the memory corresponding to the address, expect to return 0 to represent success
        ]

        [[tests]]
        name = "Test_Case_2"
        cmds = [
            { opfunc = "my_malloc", expect_res = 0, args = ["len=100", "mem_idx=2"] }, # Apply for 100bytes of memory, store the first address in the array with mem_idx=2, and expect to return 0 to represent success
            { opfunc = "my_free", expect_res = -12, args = ["mem_idx=5"] }  # Take out an address from mem_idx=5, release the address, expect to return -12 to represent failure
        ]
        ```
#### Usage
- `hitest -h` to view help
- `hitest --sample` to run libmalloc test cases, you can see the dynamic library encapsulation method and configuration file writing format in the ./sample directory
- `hitest -l libs.toml -t test_case.toml` to get dynamic library information from libs.toml and test cases from test_case.toml, and execute test cases
- `hitest -l libs.toml -t test_case.toml --log debug` to get dynamic library information from libs.toml and test cases from test_case.toml, execute test cases, log level is debug (allowed log levels are: info, debug, error, the default is info level)
#### Contribution
1. Fork this repository
2. Create a new Feat_xxx branch
3. Submit code
4. Create a new Pull Request