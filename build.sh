#!/bin/bash

# 设置变量
PROJECT_ROOT=$(pwd)
RELEASE_DIR="${PROJECT_ROOT}/release"
TARGET="release"

# 创建release目录
echo "创建release目录..."
mkdir -p "${RELEASE_DIR}"

# 构建release版本
echo "构建release版本..."
cargo build --release

# 拷贝二进制文件
echo "拷贝二进制文件..."
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    cp "${PROJECT_ROOT}/target/${TARGET}/hitest" "${RELEASE_DIR}/"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    cp "${PROJECT_ROOT}/target/${TARGET}/hitest" "${RELEASE_DIR}/"
elif [[ "$OSTYPE" == "msys"* ]]; then
    cp "${PROJECT_ROOT}/target/${TARGET}/hitest.exe" "${RELEASE_DIR}/"
fi

# 拷贝sample和scripts目录
echo "拷贝sample和scripts目录..."
cp -r "${PROJECT_ROOT}/sample" "${RELEASE_DIR}/"
cp -r "${PROJECT_ROOT}/scripts" "${RELEASE_DIR}/"

# 设置执行权限
echo "设置执行权限..."
if [[ "$OSTYPE" != "msys"* ]]; then
    chmod +x "${RELEASE_DIR}/hitest"
    chmod +x "${RELEASE_DIR}/scripts/"*.py
fi

echo "Release构建完成！目录: ${RELEASE_DIR}"
