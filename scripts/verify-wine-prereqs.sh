#!/bin/bash
# Verify prerequisites for Windows/MSVC cross-compilation testing with wine
# Checks: wine, clang-cl, XWIN_CACHE, LLVM_SYS_140_PREFIX
# Exits 0 with OK or SKIP message; never exits non-zero for missing prereqs

set -e

# Helper to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Helper to check if a directory exists
dir_exists() {
    [ -d "$1" ]
}

# Collect version info and paths
wine_version=""
clang_cl_version=""
xwin_path=""
llvm_path=""

# Check wine
if ! command_exists wine; then
    echo "SKIP: wine not found in PATH"
    exit 0
fi
wine_version=$(wine --version 2>/dev/null || echo "unknown")

# Check clang-cl
if ! command_exists clang-cl; then
    echo "SKIP: clang-cl not found in PATH"
    exit 0
fi
clang_cl_version=$(clang-cl --version 2>/dev/null | head -1 || echo "unknown")

# Check XWIN_CACHE
if [ -z "$XWIN_CACHE" ]; then
    echo "SKIP: XWIN_CACHE environment variable not set"
    exit 0
fi
if ! dir_exists "$XWIN_CACHE/crt/include"; then
    echo "SKIP: XWIN_CACHE/crt/include directory does not exist at $XWIN_CACHE/crt/include"
    exit 0
fi
xwin_path="$XWIN_CACHE"

# Check LLVM_SYS_140_PREFIX
if [ -z "$LLVM_SYS_140_PREFIX" ]; then
    echo "SKIP: LLVM_SYS_140_PREFIX environment variable not set"
    exit 0
fi
if ! dir_exists "$LLVM_SYS_140_PREFIX"; then
    echo "SKIP: LLVM_SYS_140_PREFIX directory does not exist at $LLVM_SYS_140_PREFIX"
    exit 0
fi
llvm_path="$LLVM_SYS_140_PREFIX"

# All prerequisites present
echo "OK: wine=$wine_version clang-cl=$clang_cl_version xwin=$xwin_path llvm=$llvm_path"
exit 0
