#!/bin/bash
# Verify prerequisites for Windows/MSVC cross-compilation testing with wine
# Checks: wine, clang-cl, XWIN_CACHE, LLVM_SYS_140_PREFIX
# Exits 0 with OK or SKIP message; never exits non-zero for missing prereqs

set -e

XWIN_EXPECTED_VERSION="0.9.0"
WINE_MIN_MAJOR=8
DEFAULT_LLVM_PREFIX="/usr/lib/llvm-14"
DEFAULT_XWIN_CACHE="$HOME/.xwin"

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

first_line() {
    "$@" 2>/dev/null | sed -n '1p'
}

dir_exists() {
    [ -d "$1" ]
}

resolve_llvm_tool() {
    tool_name="$1"
    explicit_var="$2"
    if [ -n "$explicit_var" ] && [ -x "$explicit_var" ]; then
        printf '%s\n' "$explicit_var"
        return 0
    fi
    if command_exists "$tool_name"; then
        command -v "$tool_name"
        return 0
    fi
    if [ -n "$LLVM_SYS_140_PREFIX" ] && [ -x "$LLVM_SYS_140_PREFIX/bin/$tool_name" ]; then
        printf '%s\n' "$LLVM_SYS_140_PREFIX/bin/$tool_name"
        return 0
    fi
    return 1
}

parse_wine_major() {
    printf '%s' "$1" | sed -E 's/^wine-([0-9]+)\..*/\1/'
}

wine_version=""
wine_major=""
clang_cl_version=""
clang_cl_path=""
xwin_version="unknown"
xwin_path=""
llvm_path=""

if ! command_exists wine; then
    echo "SKIP: wine not found in PATH (minimum required major version: ${WINE_MIN_MAJOR}+)"
    exit 0
fi
wine_version=$(wine --version 2>/dev/null || echo "unknown")
if printf '%s' "$wine_version" | grep -qi "could not load"; then
    echo "SKIP: wine is installed but not runnable on this host ($wine_version)"
    exit 0
fi
wine_major=$(parse_wine_major "$wine_version")
if ! [ "$wine_major" -ge "$WINE_MIN_MAJOR" ] 2>/dev/null; then
    echo "SKIP: wine version $wine_version is below minimum ${WINE_MIN_MAJOR}+"
    exit 0
fi

if [ -z "$LLVM_SYS_140_PREFIX" ] && [ -d "$DEFAULT_LLVM_PREFIX" ]; then
    LLVM_SYS_140_PREFIX="$DEFAULT_LLVM_PREFIX"
fi

clang_cl_path=$(resolve_llvm_tool "clang-cl" "$OPAL_MSVC_CC") || {
    echo "SKIP: clang-cl not found in PATH, OPAL_MSVC_CC, or LLVM_SYS_140_PREFIX/bin"
    exit 0
}
clang_cl_version=$(first_line "$clang_cl_path" --version || echo "unknown")

if ! command_exists xwin; then
    echo "SKIP: xwin not found in PATH (expected cargo install xwin --version ${XWIN_EXPECTED_VERSION} --locked)"
    exit 0
fi
xwin_version=$(xwin --version 2>/dev/null | head -1 || echo "unknown")
if ! printf '%s' "$xwin_version" | grep -Fq "$XWIN_EXPECTED_VERSION"; then
    echo "SKIP: xwin version $xwin_version does not match expected ${XWIN_EXPECTED_VERSION}"
    exit 0
fi

if [ -z "$XWIN_CACHE" ] && [ -d "$DEFAULT_XWIN_CACHE/crt/include" ]; then
    XWIN_CACHE="$DEFAULT_XWIN_CACHE"
fi
if [ -z "$XWIN_CACHE" ]; then
    echo "SKIP: XWIN_CACHE environment variable not set (expected xwin ${XWIN_EXPECTED_VERSION})"
    exit 0
fi
if ! dir_exists "$XWIN_CACHE/crt/include"; then
    echo "SKIP: XWIN_CACHE/crt/include directory does not exist at $XWIN_CACHE/crt/include (expected xwin ${XWIN_EXPECTED_VERSION})"
    exit 0
fi
xwin_path="$XWIN_CACHE"

if [ -z "$LLVM_SYS_140_PREFIX" ]; then
    echo "SKIP: LLVM_SYS_140_PREFIX environment variable not set"
    exit 0
fi
if ! dir_exists "$LLVM_SYS_140_PREFIX"; then
    echo "SKIP: LLVM_SYS_140_PREFIX directory does not exist at $LLVM_SYS_140_PREFIX"
    exit 0
fi
llvm_path="$LLVM_SYS_140_PREFIX"

echo "OK: wine=$wine_version (minimum ${WINE_MIN_MAJOR}+) clang-cl=$clang_cl_version clang_cl_path=$clang_cl_path xwin=$xwin_version (expected ${XWIN_EXPECTED_VERSION}) xwin_cache=$xwin_path llvm=$llvm_path"
exit 0
