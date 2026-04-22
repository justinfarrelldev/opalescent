//! Linker detection and selection based on target triple.

use crate::build_system::targets::{Architecture, Platform, TargetTriple, TripleEnv};

/// Supported linker variants for different platforms and toolchains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linker {
    /// Microsoft Visual C++ linker (link.exe).
    Msvc,
    /// MinGW-w64 linker (x86_64-w64-mingw32-gcc).
    MinGw,
    /// Clang linker (clang).
    Clang,
    /// Generic C compiler linker (cc/gcc).
    Cc,
}

impl Linker {
    /// Return human-readable binary name for this linker.
    #[must_use]
    pub const fn binary_name(&self) -> &'static str {
        match self {
            Self::Msvc => "link.exe",
            Self::MinGw => "x86_64-w64-mingw32-gcc",
            Self::Clang => "clang",
            Self::Cc => "cc",
        }
    }
}

/// Detect the preferred linker for a given target triple.
///
/// Returns the linker variant based on the target platform and environment:
/// - `*-windows-msvc` → `Linker::Msvc`
/// - `*-windows-gnu` → `Linker::MinGw`
/// - `*-linux-gnu` / `*-linux-musl` → `Linker::Cc`
/// - `*-darwin` → `Linker::Clang`
///
/// # Examples
///
/// ```ignore
/// let msvc = parse_target_triple("x86_64-pc-windows-msvc").unwrap();
/// assert_eq!(detect_preferred_linker(&msvc), Linker::Msvc);
///
/// let linux = parse_target_triple("x86_64-linux").unwrap();
/// assert_eq!(detect_preferred_linker(&linux), Linker::Cc);
/// ```
#[must_use]
pub fn detect_preferred_linker(target: &TargetTriple) -> Linker {
    match (target.platform, target.env) {
        (Platform::Windows, Some(TripleEnv::Gnu)) => Linker::MinGw,
        (Platform::Windows, _) => Linker::Msvc,
        (Platform::Linux, _) => Linker::Cc,
        (Platform::MacOs, _) => Linker::Clang,
    }
}

/// A builder for constructing linker commands with proper platform-specific arguments.
///
/// This struct encapsulates the logic for building platform-specific linker invocations,
/// handling differences between MSVC, MinGW, Clang, and GCC linkers.
#[derive(Debug, Clone)]
pub struct LinkerCommand {
    linker: Linker,
    inputs: Vec<std::path::PathBuf>,
    output: std::path::PathBuf,
    runtime: Option<std::path::PathBuf>,
}

impl LinkerCommand {
    /// Create a new linker command for the given target and output path.
    ///
    /// The linker variant is automatically detected based on the target triple.
    #[must_use]
    pub fn new(target: &TargetTriple, output: std::path::PathBuf) -> Self {
        Self {
            linker: detect_preferred_linker(target),
            inputs: Vec::new(),
            output,
            runtime: None,
        }
    }

    /// Add an input object file to the linker command.
    #[must_use]
    pub fn with_input(mut self, path: std::path::PathBuf) -> Self {
        self.inputs.push(path);
        self
    }

    /// Add the runtime library path to the linker command.
    #[must_use]
    pub fn with_runtime(mut self, path: std::path::PathBuf) -> Self {
        self.runtime = Some(path);
        self
    }

    /// Build the final `std::process::Command` with all platform-specific arguments.
    ///
    /// Dispatches on the linker variant and constructs the appropriate command line.
    /// Paths containing spaces are automatically quoted.
    pub fn build(self) -> std::process::Command {
        let mut cmd = std::process::Command::new(self.linker.binary_name());

        match self.linker {
            Linker::Msvc => {
                // MSVC: link.exe /OUT:<output> <inputs...> <runtime>
                cmd.arg(format!("/OUT:{}", self.output.display()));
                for input in &self.inputs {
                    let arg = self.quote_if_needed(input.display().to_string());
                    cmd.arg(arg);
                }
                if let Some(runtime) = &self.runtime {
                    cmd.arg(runtime);
                }
            }
            Linker::MinGw => {
                // MinGW: x86_64-w64-mingw32-gcc <inputs...> <runtime> -o <output>
                for input in &self.inputs {
                    let arg = self.quote_if_needed(input.display().to_string());
                    cmd.arg(arg);
                }
                if let Some(runtime) = &self.runtime {
                    cmd.arg(runtime);
                }
                cmd.arg("-o").arg(self.quote_if_needed(self.output.display().to_string()));
            }
            Linker::Clang => {
                // Clang: clang <inputs...> <runtime> -o <output>
                for input in &self.inputs {
                    let arg = self.quote_if_needed(input.display().to_string());
                    cmd.arg(arg);
                }
                if let Some(runtime) = &self.runtime {
                    cmd.arg(runtime);
                }
                cmd.arg("-o").arg(self.quote_if_needed(self.output.display().to_string()));
            }
            Linker::Cc => {
                // GCC/cc: cc <inputs...> <runtime> -no-pie -o <output>
                for input in &self.inputs {
                    let arg = self.quote_if_needed(input.display().to_string());
                    cmd.arg(arg);
                }
                if let Some(runtime) = &self.runtime {
                    cmd.arg(runtime);
                }
                cmd.arg("-no-pie");
                cmd.arg("-o").arg(self.quote_if_needed(self.output.display().to_string()));
            }
        }

        cmd
    }

    /// Quote a path if it contains spaces.
    fn quote_if_needed(&self, path: String) -> String {
        if path.contains(' ') {
            format!("\"{}\"", path)
        } else {
            path
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linker_command_cc_builds_correct_args() {
        let target = TargetTriple {
            arch: Architecture::X86_64,
            platform: Platform::Linux,
            env: None,
        };
        let output = std::path::PathBuf::from("program");
        let input = std::path::PathBuf::from("main.o");
        let runtime = std::path::PathBuf::from("runtime.o");

        let cmd = LinkerCommand::new(&target, output)
            .with_input(input)
            .with_runtime(runtime)
            .build();

        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        assert_eq!(args[0], "main.o");
        assert_eq!(args[1], "runtime.o");
        assert_eq!(args[2], "-no-pie");
        assert_eq!(args[3], "-o");
        assert_eq!(args[4], "program");
    }

    #[test]
    fn linker_command_clang_builds_correct_args() {
        let target = TargetTriple {
            arch: Architecture::X86_64,
            platform: Platform::MacOs,
            env: None,
        };
        let output = std::path::PathBuf::from("program");
        let input = std::path::PathBuf::from("main.o");
        let runtime = std::path::PathBuf::from("runtime.o");

        let cmd = LinkerCommand::new(&target, output)
            .with_input(input)
            .with_runtime(runtime)
            .build();

        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        assert_eq!(args[0], "main.o");
        assert_eq!(args[1], "runtime.o");
        assert_eq!(args[2], "-o");
        assert_eq!(args[3], "program");
    }

    #[test]
    fn linker_command_mingw_builds_correct_args() {
        let target = TargetTriple {
            arch: Architecture::X86_64,
            platform: Platform::Windows,
            env: Some(TripleEnv::Gnu),
        };
        let output = std::path::PathBuf::from("program.exe");
        let input = std::path::PathBuf::from("main.o");
        let runtime = std::path::PathBuf::from("runtime.o");

        let cmd = LinkerCommand::new(&target, output)
            .with_input(input)
            .with_runtime(runtime)
            .build();

        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        assert_eq!(args[0], "main.o");
        assert_eq!(args[1], "runtime.o");
        assert_eq!(args[2], "-o");
        assert_eq!(args[3], "program.exe");
    }

    #[test]
    fn linker_command_msvc_builds_correct_args() {
        let target = TargetTriple {
            arch: Architecture::X86_64,
            platform: Platform::Windows,
            env: Some(TripleEnv::Msvc),
        };
        let output = std::path::PathBuf::from("program.exe");
        let input = std::path::PathBuf::from("main.obj");
        let runtime = std::path::PathBuf::from("runtime.obj");

        let cmd = LinkerCommand::new(&target, output)
            .with_input(input)
            .with_runtime(runtime)
            .build();

        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        assert_eq!(args[0], "/OUT:program.exe");
        assert_eq!(args[1], "main.obj");
        assert_eq!(args[2], "runtime.obj");
    }

    #[test]
    fn linker_command_quotes_paths_with_spaces() {
        let target = TargetTriple {
            arch: Architecture::X86_64,
            platform: Platform::Linux,
            env: None,
        };
        let output = std::path::PathBuf::from("my program");
        let input = std::path::PathBuf::from("my object.o");
        let runtime = std::path::PathBuf::from("my runtime.o");

        let cmd = LinkerCommand::new(&target, output)
            .with_input(input)
            .with_runtime(runtime)
            .build();

        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        assert_eq!(args[0], "\"my object.o\"");
        assert_eq!(args[1], "my runtime.o");
        assert_eq!(args[4], "\"my program\"");
    }

    #[test]
    fn linker_command_multiple_inputs() {
        let target = TargetTriple {
            arch: Architecture::X86_64,
            platform: Platform::Linux,
            env: None,
        };
        let output = std::path::PathBuf::from("program");
        let input1 = std::path::PathBuf::from("main.o");
        let input2 = std::path::PathBuf::from("lib.o");
        let runtime = std::path::PathBuf::from("runtime.o");

        let cmd = LinkerCommand::new(&target, output)
            .with_input(input1)
            .with_input(input2)
            .with_runtime(runtime)
            .build();

        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        assert_eq!(args[0], "main.o");
        assert_eq!(args[1], "lib.o");
        assert_eq!(args[2], "runtime.o");
        assert_eq!(args[3], "-no-pie");
        assert_eq!(args[4], "-o");
        assert_eq!(args[5], "program");
    }
}

    #[test]
    fn no_pie_flag_only_on_linux() {
        // Test 1: Linux GNU — should have -no-pie
        let linux_gnu = TargetTriple {
            arch: Architecture::X86_64,
            platform: Platform::Linux,
            env: Some(TripleEnv::Gnu),
        };
        let cmd = LinkerCommand::new(&linux_gnu, std::path::PathBuf::from("program"))
            .with_input(std::path::PathBuf::from("main.o"))
            .build();
        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"-no-pie".to_string()), "Linux GNU should have -no-pie");

        // Test 2: Linux MUSL — should have -no-pie
        let linux_musl = TargetTriple {
            arch: Architecture::X86_64,
            platform: Platform::Linux,
            env: Some(TripleEnv::Musl),
        };
        let cmd = LinkerCommand::new(&linux_musl, std::path::PathBuf::from("program"))
            .with_input(std::path::PathBuf::from("main.o"))
            .build();
        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"-no-pie".to_string()), "Linux MUSL should have -no-pie");

        // Test 3: Windows MSVC — should NOT have -no-pie
        let windows_msvc = TargetTriple {
            arch: Architecture::X86_64,
            platform: Platform::Windows,
            env: Some(TripleEnv::Msvc),
        };
        let cmd = LinkerCommand::new(&windows_msvc, std::path::PathBuf::from("program.exe"))
            .with_input(std::path::PathBuf::from("main.obj"))
            .build();
        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        assert!(!args.contains(&"-no-pie".to_string()), "Windows MSVC should NOT have -no-pie");

        // Test 4: macOS Darwin — should NOT have -no-pie
        let darwin = TargetTriple {
            arch: Architecture::X86_64,
            platform: Platform::MacOs,
            env: None,
        };
        let cmd = LinkerCommand::new(&darwin, std::path::PathBuf::from("program"))
            .with_input(std::path::PathBuf::from("main.o"))
            .build();
        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        assert!(!args.contains(&"-no-pie".to_string()), "macOS Darwin should NOT have -no-pie");
    }
}
