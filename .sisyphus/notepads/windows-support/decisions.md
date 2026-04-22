# Decisions — windows-support

## [2026-04-21] Session start

### Locked interview decisions
1. MSVC primary (link.exe + MSVC CRT), MinGW-w64 best-effort
2. Linux→Windows cross-compile: HARD requirement
3. aarch64-pc-windows-*: OUT OF SCOPE
4. Min Windows: Win10 any + Server 2016+
5. Hot-reload day-one on Windows (.dll + LoadLibraryW)
6. Static LLVM on ALL platforms
7. Code signing: OUT OF SCOPE
8. zig cc: BANNED

### TargetTriple model (Task 0.5)
- Add TripleEnv { Msvc, Gnu, Musl } enum (exactly 3 variants)
- Add env: Option<TripleEnv> to TargetTriple (additive)
- Legacy x86_64-windows: env=None, resolves as MSVC downstream, emits deprecation warning
- 4-segment form: x86_64-pc-windows-msvc is canonical

### Linker strategy (Tasks 10-12)
- Windows host: cc::windows_registry::find_tool (gated #[cfg(windows)])
- Linux host → MSVC target: clang-cl + lld-link + xwin sysroot
- Linux host → MinGW target: x86_64-w64-mingw32-gcc
- Linux host → Linux target: cc / gcc
