#!/usr/bin/env python3
import sys
import tomlkit
from pathlib import Path
import argparse
import tempfile
import os

def preprocess_file(source_path: str) -> str:
    """预处理C文件，移除不匹配当前平台的条件编译块，返回临时文件路径"""
    platform_macros = {
        'linux': '__linux__',
        'win32': '_WIN32',
        'darwin': '__APPLE__'
    }
    current_platform = platform_macros.get(sys.platform, None)

    content = Path(source_path).read_text()
    lines = content.splitlines()
    temp_path = os.path.join(os.path.dirname(source_path), '.temp.c')

    with open(temp_path, 'w') as f:
        skip_depth = 0
        skip_stack = []

        for line in lines:
            stripped = line.strip()

            # 处理条件编译开始
            if stripped.startswith('#if'):
                macro = stripped[3:].strip()
                should_skip = current_platform and current_platform not in macro
                skip_stack.append(should_skip)
                if should_skip:
                    skip_depth += 1
                continue

            # 处理条件编译结束
            if stripped.startswith('#endif'):
                if skip_stack:
                    if skip_stack.pop():
                        skip_depth -= 1
                continue

            # 跳过不匹配平台的代码块
            if skip_depth > 0:
                continue

            f.write(line + '\n')

    return temp_path

def parse_export_funcs(source_path: str) -> list:
    """解析C文件中的EXPORT_FUNC宏"""
    # 预处理文件
    temp_file = preprocess_file(source_path)

    try:
        content = Path(temp_file).read_text()
        configs = []
        pos = 0
        len_content = len(content)

        while pos < len_content:
            macro_start = content.find("EXPORT_FUNC(", pos)
            if macro_start == -1:
                break

            # Skip processing if the macro is part of a #define
            line_start = content.rfind('\n', 0, macro_start) + 1
            if content[line_start:macro_start].strip().startswith("#define"):
                pos = macro_start + len("EXPORT_FUNC(")
                continue

            # Parse arguments within parentheses
            pos = macro_start + len("EXPORT_FUNC(")
            depth = 1
            while pos < len_content and depth > 0:
                char = content[pos]
                if char == '(':
                    depth += 1
                elif char == ')':
                    depth -= 1
                pos += 1

            # Extract and process raw arguments
            raw_args = content[macro_start+len("EXPORT_FUNC("):pos-1].strip()
            if ',' not in raw_args:
                func_name = raw_args
                params = []
            else:
                func_name, params_str = raw_args.split(',', 1)
                func_name = func_name.strip()
                params = [p.strip() for p in params_str.split(',') if p.strip()]

            configs.append({
                "name": f"Call_{func_name}",
                "params": params  # Preserve original parameter names without quotes
            })

        return configs
    finally:
        # 删除临时文件
        if os.path.exists(temp_file):
            os.remove(temp_file)

def generate_toml(configs: list, output_path: str, file_name: str, lib_dir: str = None):
    """Generate TOML configuration with proper string quoting."""
    doc = tomlkit.document()

    # Generate platform-specific library name
    libs = tomlkit.table()
    lib_ext = {
        "win32": ".dll",
        "darwin": ".dylib"
    }.get(sys.platform, ".so")


    lib_name = f"{file_name}{lib_ext}" if sys.platform == "win32" else f"lib{file_name}{lib_ext}"

    if lib_dir:
        lib_name = lib_dir

    libs.add("path", lib_name)
    print(f'auto set lib path as {lib_name}')
    # Build functions array with parameters
    funcs = tomlkit.array()
    for cfg in configs:
        func = tomlkit.inline_table()
        func.add("name", cfg["name"])

        # Process parameters with TOML-compatible quoting
        paras = tomlkit.array()
        for p in cfg["params"]:
            paras.append(tomlkit.string(p))  # Let tomlkit handle string quoting
        func.append("paras", paras)

        funcs.append(func)

    libs.add("funcs", funcs)
    doc.add("libs", [libs])

    with open(output_path, "w") as f:
        f.write(tomlkit.dumps(doc))

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Generate TOML configuration from C source file')
    parser.add_argument('-f', '--file', required=True, help='Source C file')
    parser.add_argument('-o', '--output', required=True, help='Output TOML file')
    parser.add_argument('-l', '--libdir', help='Library directory (optional)')

    args = parser.parse_args()

    # 解析并生成配置
    configs = parse_export_funcs(args.file)

    # 生成TOML配置
    file_name = Path(args.file).stem
    generate_toml(configs, args.output, file_name, args.libdir)
    print(f"Generated {len(configs)} function configurations to {args.output}")
