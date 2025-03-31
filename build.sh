#!/bin/bash

# 设置变量
PROJECT_ROOT=$(pwd)
RELEASE_DIR="${PROJECT_ROOT}/release/hitest"

generate_cfg() {
    echo "拷贝配置文件...​"
    local cfgdir="${RELEASE_DIR}/cfgs"
    mkdir -p "${cfgdir}"
    python3 scripts/generate_config.py -f sample/libmalloc.c -o "${cfgdir}/libmalloc.toml" -l "libmalloc.so"
    cp -r scripts ${RELEASE_DIR}/
}

build_riscv() {
    local target="riscv64gc-unknown-linux-gnu"
    local target_dir="${RELEASE_DIR}/riscv"

    echo "构建 RISC-V 版本..."

    # 创建目录结构
    mkdir -p "${target_dir}/libs"

    # 构建共享库
    riscv_gcc="/home/e0007816/codes/win2030/buildroot/output/host/bin/riscv64-unknown-linux-gnu-gcc"
    "${riscv_gcc}" sample/libmalloc.c -shared -fPIC -o "${target_dir}/libs/libmalloc.so"

    # 构建主程序
    cargo build --release --target=${target}
    cp "${PROJECT_ROOT}/target/${target}/release/hitest" "${target_dir}/"
}

build_x86() {
    local target_dir="${RELEASE_DIR}/x86"
    echo "构建 x86 版本..."

    # 创建目录结构
    mkdir -p "${target_dir}/libs"

    # 构建共享库
    gcc sample/libmalloc.c -shared -fPIC -o "${target_dir}/libs/libmalloc.so"

    # 构建主程序
    cargo build --release
    cp "${PROJECT_ROOT}/target/release/hitest" "${target_dir}/"
}

# 创建release目录
echo "创建release目录..."
rm -rf "${RELEASE_DIR}"
mkdir -p "${RELEASE_DIR}"

generate_cfg
# 构建两个架构版本
build_x86
build_riscv

# 设置执行权限
echo "设置执行权限..."
find "${RELEASE_DIR}" -type f -name "hitest" -exec chmod +x {} \;
find "${RELEASE_DIR}" -type f -name "*.py" -exec chmod +x {} \;

echo "Release构建完成！目录: ${RELEASE_DIR}"