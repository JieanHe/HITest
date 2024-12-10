#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e

# Build the project in release mode
cargo build --release

# Create a temporary directory
mkdir -p temp/sample

# Copy the libmalloc.c file to the temp/sample directory
cp sample/libmalloc.c temp/sample/libmalloc.c

# Copy the hitest executable to the temp directory
cp target/release/hitest.exe temp/

# Create a tar.gz package
tar -czf target/hitest.tar.gz -C temp .

# Clean up the temporary directory
rm -rf temp

echo "Successfully created target/hitest.tar.gz"