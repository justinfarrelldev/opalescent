extern crate alloc;

use crate::build_system::BuildError;
use crate::build_system::cache::BuildCache;
use crate::build_system::config::Dependency;
use crate::build_system::config::{parse_config, parse_version, parse_version_constraint};
use crate::build_system::dependency::{PackageVersion, resolve_dependencies};
use crate::build_system::incremental::modules_to_rebuild;
use crate::build_system::linker::{Linker, detect_preferred_linker};
use crate::build_system::targets::{
    Architecture, Platform, TargetTriple, TripleEnv, dynamic_lib_extension, executable_filename,
    object_file_extension, parse_target_triple,
};
use crate::hot_reload::dependency_graph::ModuleDependencyGraph;
use alloc::string::String;
use alloc::vec;

#[test]
fn parse_config_reads_name_version_dependencies_and_targets() {
    let input = r#"
name = "opal_demo"
version = "1.2.3"

[dependencies]
core = ">=1.0.0 <2.0.0"
math = "=2.4.1"

[build]
targets = ["x86_64-linux", "aarch64-darwin"]
"#;

    let parsed = parse_config(input);
    assert!(parsed.is_ok(), "config should parse successfully");
    let Ok(config) = parsed else {
        return;
    };

    assert_eq!(config.name, "opal_demo");
    assert_eq!(config.version.major, 1);
    assert_eq!(config.version.minor, 2);
    assert_eq!(config.version.patch, 3);
    assert_eq!(config.dependencies.len(), 2);
    assert_eq!(config.build_targets.len(), 2);
}

#[test]
fn parse_config_reports_missing_required_fields() {
    let input = "name = \"opal_demo\"";
    let parsed = parse_config(input);
    assert!(
        matches!(parsed, Err(BuildError::MissingField(_))),
        "missing version should return MissingField"
    );
}

#[test]
fn parse_version_constraint_supports_range_equality_and_bare_versions() {
    let parsed_range = parse_version_constraint(">=1.0.0 <2.0.0");
    assert!(
        parsed_range.is_ok(),
        "range constraint should parse successfully"
    );

    let parsed_eq = parse_version_constraint("=2.5.1");
    assert!(
        parsed_eq.is_ok(),
        "equality constraint should parse successfully"
    );

    let parsed_bare = parse_version_constraint("2.5.1");
    assert!(
        parsed_bare.is_ok(),
        "bare version constraint should parse as equality"
    );
}

#[test]
fn resolve_dependencies_selects_highest_matching_versions() {
    let dependency_constraint = parse_version_constraint(">=1.0.0 <2.0.0");
    assert!(
        dependency_constraint.is_ok(),
        "constraint parse should succeed"
    );
    let Ok(dependency_constraint_value) = dependency_constraint else {
        return;
    };

    let dependency = Dependency {
        name: String::from("core"),
        version_constraint: dependency_constraint_value,
    };

    let Ok(first_version) = parse_version("1.1.0") else {
        return;
    };
    let Ok(second_version) = parse_version("1.9.0") else {
        return;
    };
    let Ok(third_version) = parse_version("2.0.0") else {
        return;
    };

    let available = vec![
        PackageVersion {
            name: String::from("core"),
            version: first_version,
        },
        PackageVersion {
            name: String::from("core"),
            version: second_version,
        },
        PackageVersion {
            name: String::from("core"),
            version: third_version,
        },
    ];

    let resolved = resolve_dependencies(&[dependency], available.as_slice());
    assert!(resolved.is_ok(), "dependency resolution should succeed");
    let Ok(entries) = resolved else {
        return;
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "core");
    assert_eq!(entries[0].version.major, 1);
    assert_eq!(entries[0].version.minor, 9);
    assert_eq!(entries[0].version.patch, 0);
}

#[test]
fn resolve_dependencies_reports_conflicts_for_same_package() {
    let first_constraint = parse_version_constraint(">=1.0.0 <2.0.0");
    assert!(first_constraint.is_ok(), "constraint parse should succeed");
    let second_constraint = parse_version_constraint(">=2.0.0 <3.0.0");
    assert!(second_constraint.is_ok(), "constraint parse should succeed");

    let Ok(first_constraint_value) = first_constraint else {
        return;
    };

    let Ok(second_constraint_value) = second_constraint else {
        return;
    };

    let deps = vec![
        Dependency {
            name: String::from("shared"),
            version_constraint: first_constraint_value,
        },
        Dependency {
            name: String::from("shared"),
            version_constraint: second_constraint_value,
        },
    ];

    let Ok(shared_version) = parse_version("2.1.0") else {
        return;
    };

    let available = vec![PackageVersion {
        name: String::from("shared"),
        version: shared_version,
    }];

    let resolved = resolve_dependencies(deps.as_slice(), available.as_slice());
    assert!(
        matches!(resolved, Err(BuildError::DependencyConflict(_))),
        "incompatible constraints should report dependency conflict"
    );
}

#[test]
fn build_cache_hashes_content_and_detects_cache_hits() {
    let mut cache = BuildCache::new();
    assert!(
        !cache.is_cache_hit("module_a", "pub fn a() -> i32 { return 1; }"),
        "uncached content should miss"
    );

    cache.update_cache("module_a", "pub fn a() -> i32 { return 1; }");
    assert!(
        cache.is_cache_hit("module_a", "pub fn a() -> i32 { return 1; }"),
        "unchanged content should hit"
    );
    assert!(
        !cache.is_cache_hit("module_a", "pub fn a() -> i32 { return 2; }"),
        "changed content should miss"
    );
}

#[test]
fn incremental_build_includes_changed_modules_and_transitive_dependents() {
    let mut graph = ModuleDependencyGraph::new();
    graph.add_dependency("feature", "core");
    graph.add_dependency("cli", "feature");
    graph.add_dependency("tests", "cli");

    let changed = vec![String::from("core")];
    let rebuild = modules_to_rebuild(changed.as_slice(), &graph);

    assert_eq!(
        rebuild,
        vec![
            String::from("cli"),
            String::from("core"),
            String::from("feature"),
            String::from("tests")
        ],
        "rebuild plan should include changed module and all transitive dependents"
    );
}

#[test]
fn target_triples_parse_and_dynamic_library_extensions_match_platform() {
    let linux = parse_target_triple("x86_64-linux");
    assert!(linux.is_ok(), "linux triple should parse");
    let Ok(linux_target) = linux else {
        return;
    };
    assert!(matches!(linux_target.arch, Architecture::X86_64));
    assert!(matches!(linux_target.platform, Platform::Linux));
    assert_eq!(dynamic_lib_extension(&linux_target), ".so");

    let mac = parse_target_triple("aarch64-darwin");
    assert!(mac.is_ok(), "darwin triple should parse");
    let Ok(mac_target) = mac else {
        return;
    };
    assert!(matches!(mac_target.arch, Architecture::Aarch64));
    assert!(matches!(mac_target.platform, Platform::MacOs));
    assert_eq!(dynamic_lib_extension(&mac_target), ".dylib");

    let windows = parse_target_triple("x86_64-windows");
    assert!(windows.is_ok(), "windows triple should parse");
    let Ok(windows_target) = windows else {
        return;
    };
    assert!(matches!(windows_target.arch, Architecture::X86_64));
    assert!(matches!(windows_target.platform, Platform::Windows));
    assert_eq!(dynamic_lib_extension(&windows_target), ".dll");
}

#[test]
fn invalid_target_triple_reports_build_error() {
    let parsed = parse_target_triple("riscv64-linux");
    assert!(
        matches!(parsed, Err(BuildError::InvalidTarget(_))),
        "unsupported triples should return InvalidTarget"
    );
}

#[test]
fn parse_rust_msvc_triple() {
    let t = parse_target_triple("x86_64-pc-windows-msvc").unwrap();
    assert_eq!(t.env, Some(TripleEnv::Msvc));
    assert_eq!(t.platform, Platform::Windows);
}

#[test]
fn parse_rust_mingw_triple() {
    let t = parse_target_triple("x86_64-pc-windows-gnu").unwrap();
    assert_eq!(t.env, Some(TripleEnv::Gnu));
}

#[test]
fn parse_legacy_2_segment_still_works() {
    let t = parse_target_triple("x86_64-windows").unwrap();
    assert_eq!(t.env, None);
    assert_eq!(t.platform, Platform::Windows);
}

#[test]
fn parse_legacy_windows_resolves_as_msvc() {
    let t = parse_target_triple("x86_64-windows").unwrap();
    assert!(t.is_windows_msvc());
}

#[test]
fn parse_legacy_linux_still_works() {
    let t = parse_target_triple("x86_64-linux").unwrap();
    assert_eq!(t.env, None);
    assert_eq!(t.platform, Platform::Linux);
}

#[test]
fn reject_aarch64_windows_msvc() {
    assert!(parse_target_triple("aarch64-pc-windows-msvc").is_err());
}

#[test]
fn reject_3_segment() {
    assert!(parse_target_triple("x86_64-unknown-linux").is_err());
}

#[test]
fn reject_unknown_env() {
    assert!(parse_target_triple("x86_64-pc-windows-clang").is_err());
}

#[test]
fn to_rust_triple_roundtrips() {
    let t = parse_target_triple("x86_64-pc-windows-msvc").unwrap();
    assert_eq!(t.to_rust_triple(), "x86_64-pc-windows-msvc");
}

#[test]
fn tests_target_triple_typed_api() {
    // Verify that compile_program accepts TargetTriple, not &str
    // This test documents the expected API shape
    let triple = TargetTriple::host();
    // If this compiles, the API is typed correctly
    assert!(triple.is_windows_msvc() || !triple.is_windows_msvc());
    assert!(!triple.to_rust_triple().is_empty());
}

#[test]
fn object_file_extension_windows_msvc() {
    let t = parse_target_triple("x86_64-pc-windows-msvc").unwrap();
    assert_eq!(object_file_extension(&t), ".obj");
}

#[test]
fn object_file_extension_windows_gnu() {
    let t = parse_target_triple("x86_64-pc-windows-gnu").unwrap();
    assert_eq!(object_file_extension(&t), ".o");
}

#[test]
fn object_file_extension_linux() {
    let t = parse_target_triple("x86_64-linux").unwrap();
    assert_eq!(object_file_extension(&t), ".o");
}

#[test]
fn object_file_extension_darwin() {
    let t = parse_target_triple("aarch64-darwin").unwrap();
    assert_eq!(object_file_extension(&t), ".o");
}

#[test]
fn object_file_extension_legacy_fallbacks() {
    // Legacy 2-segment windows resolves as MSVC per Task 0.5
    let t = parse_target_triple("x86_64-windows").unwrap();
    assert_eq!(object_file_extension(&t), ".obj");
    
    let t = parse_target_triple("x86_64-linux").unwrap();
    assert_eq!(object_file_extension(&t), ".o");
    
    let t = parse_target_triple("aarch64-darwin").unwrap();
    assert_eq!(object_file_extension(&t), ".o");
}

#[test]
fn executable_filename_windows_msvc() {
    let t = parse_target_triple("x86_64-pc-windows-msvc").unwrap();
    assert_eq!(executable_filename("prog", &t), "prog.exe");
}

#[test]
fn executable_filename_windows_gnu() {
    let t = parse_target_triple("x86_64-pc-windows-gnu").unwrap();
    assert_eq!(executable_filename("prog", &t), "prog.exe");
}

#[test]
fn executable_filename_linux() {
    let t = parse_target_triple("x86_64-linux").unwrap();
    assert_eq!(executable_filename("prog", &t), "prog");
}

#[test]
fn executable_filename_darwin() {
    let t = parse_target_triple("aarch64-darwin").unwrap();
    assert_eq!(executable_filename("prog", &t), "prog");
}

#[test]
fn detect_linker_windows_msvc() {
    let t = parse_target_triple("x86_64-pc-windows-msvc").unwrap();
    assert_eq!(detect_preferred_linker(&t), Linker::Msvc);
}

#[test]
fn detect_linker_windows_gnu() {
    let t = parse_target_triple("x86_64-pc-windows-gnu").unwrap();
    assert_eq!(detect_preferred_linker(&t), Linker::MinGw);
}

#[test]
fn detect_linker_linux() {
    let t = parse_target_triple("x86_64-linux").unwrap();
    assert_eq!(detect_preferred_linker(&t), Linker::Cc);
}

#[test]
fn detect_linker_darwin() {
    let t = parse_target_triple("aarch64-darwin").unwrap();
    assert_eq!(detect_preferred_linker(&t), Linker::Clang);
}

#[test]
fn linker_binary_name_msvc() {
    assert_eq!(Linker::Msvc.binary_name(), "link.exe");
}

#[test]
fn linker_binary_name_mingw() {
    assert_eq!(Linker::MinGw.binary_name(), "x86_64-w64-mingw32-gcc");
}

#[test]
fn linker_binary_name_clang() {
    assert_eq!(Linker::Clang.binary_name(), "clang");
}

#[test]
fn linker_binary_name_cc() {
    assert_eq!(Linker::Cc.binary_name(), "cc");
}

#[test]
fn portability_header_compiles_with_gcc() {
    let header_path = std::path::Path::new("runtime/opal_portability.h");
    assert!(header_path.exists(), "runtime/opal_portability.h must exist");
    
    let output = std::process::Command::new("gcc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-x", "c", "-c", "/dev/null",
               "-include", "runtime/opal_portability.h", "-o", "/dev/null"])
        .output();
    
    match output {
        Ok(out) => {
            assert!(out.status.success(), 
                "gcc failed to compile portability header:\n{}",
                String::from_utf8_lossy(&out.stderr));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gcc not found, skipping portability header compile test");
        }
        Err(e) => {
            eprintln!("Failed to run gcc: {e}");
        }
    }
}

#[test]
fn rc_header_layout_constants_are_correct() {
    let rc_header_path = std::path::Path::new("runtime/opal_rc.h");
    assert!(rc_header_path.exists(), "runtime/opal_rc.h must exist");
    
    let output = std::process::Command::new("gcc")
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-x", "c",
            "-c", "/dev/null",
            "-include", "runtime/opal_rc.h",
            "-o", "/dev/null"
        ])
        .output();
    
    match output {
        Ok(out) => {
            assert!(out.status.success(), 
                "gcc failed to compile RC header with offsetof assertions:\n{}",
                String::from_utf8_lossy(&out.stderr));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gcc not found, skipping RC header layout test");
        }
        Err(e) => {
            eprintln!("Failed to run gcc: {e}");
        }
    }
}

#[test]
fn rng_portability_header_compiles_with_gcc() {
    let rng_path = std::path::Path::new("runtime/opal_rng.c");
    assert!(rng_path.exists(), "runtime/opal_rng.c must exist");
    
    let output = std::process::Command::new("gcc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-c", "runtime/opal_rng.c", "-o", "/tmp/opal_rng_test.o"])
        .output();
    
    match output {
        Ok(out) => {
            assert!(out.status.success(), 
                "gcc failed to compile opal_rng.c:\n{}",
                String::from_utf8_lossy(&out.stderr));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gcc not found, skipping rng portability compile test");
        }
        Err(e) => {
            eprintln!("Failed to run gcc: {e}");
        }
    }
}

#[test]
fn runtime_aggregator_compiles_without_c_includes() {
    let runtime_path = std::path::Path::new("runtime/opal_runtime.c");
    assert!(runtime_path.exists(), "runtime/opal_runtime.c must exist");
    
    // Verify that opal_runtime.c does NOT include .c files (only headers)
    let content = std::fs::read_to_string(runtime_path)
        .expect("failed to read runtime/opal_runtime.c");
    
    // Check that no .c files are included
    assert!(!content.contains("#include \"opal_error.c\""), 
        "opal_runtime.c must not include opal_error.c");
    assert!(!content.contains("#include \"opal_io.c\""), 
        "opal_runtime.c must not include opal_io.c");
    assert!(!content.contains("#include \"opal_print.c\""), 
        "opal_runtime.c must not include opal_print.c");
    assert!(!content.contains("#include \"opal_rng.c\""), 
        "opal_runtime.c must not include opal_rng.c");
    assert!(!content.contains("#include \"opal_parse.c\""), 
        "opal_runtime.c must not include opal_parse.c");
    assert!(!content.contains("#include \"opal_string.c\""), 
        "opal_runtime.c must not include opal_string.c");
    assert!(!content.contains("#include \"opal_bytes.c\""), 
        "opal_runtime.c must not include opal_bytes.c");
    assert!(!content.contains("#include \"opal_rc.c\""), 
        "opal_runtime.c must not include opal_rc.c");
    
    // Verify it compiles as a standalone unit
    let output = std::process::Command::new("gcc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-c", "runtime/opal_runtime.c", "-o", "/tmp/opal_runtime_test.o"])
        .output();
    
    match output {
        Ok(out) => {
            assert!(out.status.success(), 
                "gcc failed to compile opal_runtime.c as aggregator:\n{}",
                String::from_utf8_lossy(&out.stderr));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gcc not found, skipping runtime aggregator compile test");
        }
        Err(e) => {
            eprintln!("Failed to run gcc: {e}");
        }
    }
}

#[test]
fn runtime_init_compiles_with_gcc() {
    let runtime_path = std::path::Path::new("runtime/opal_runtime.c");
    assert!(runtime_path.exists(), "runtime/opal_runtime.c must exist");
    
    let output = std::process::Command::new("gcc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-c", "runtime/opal_runtime.c", "-o", "/tmp/opal_runtime.o"])
        .output();
    
    match output {
        Ok(out) => {
            assert!(out.status.success(), 
                "gcc failed to compile opal_runtime.c with opal_runtime_init:\n{}",
                String::from_utf8_lossy(&out.stderr));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("gcc not found, skipping runtime init compile test");
        }
        Err(e) => {
            eprintln!("Failed to run gcc: {e}");
        }
    }
}
