#!/usr/bin/env python3
import sys
import tomlkit
from pathlib import Path
import argparse

def parse_export_funcs(source_path: str) -> list:
    """Parse EXPORT_FUNC macros from source file and extract function details."""
    configs = []
    content = Path(source_path).read_text()
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
        lib_name = f"{lib_dir}/{lib_name}"
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

    configs = parse_export_funcs(args.file)

    # 如果有指定库目录，修改路径格式
    file_name = Path(args.file).stem
    generate_toml(configs, args.output, file_name, args.libdir)
    print(f"Generated {len(configs)} function configurations to {args.output}")
