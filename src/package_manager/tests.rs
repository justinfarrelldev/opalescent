//! Tests for the package manager modules.

#[cfg(test)]
mod package_manager_tests {
    extern crate alloc;

    use crate::build_system::targets::{dynamic_lib_extension, parse_target_triple, Platform};
    use crate::package_manager::commands::{dispatch_pkg_command, PkgCommand};
    use crate::package_manager::installer::{FailingDownloader, Installer, MockDownloader};
    use crate::package_manager::manifest::{parse_manifest, serialize_manifest};
    use crate::package_manager::publisher::{FailingUploader, MockUploader, Publisher};
    use crate::package_manager::registry::{MockRegistry, PackageEntry, Registry};
    use crate::package_manager::resolver::{parse_constraint, resolve_manifest_deps};
    use alloc::string::String;
    use alloc::vec;

    // ─── manifest tests ──────────────────────────────────────────────────────

    #[test]
    fn parse_manifest_reads_required_fields() {
        let toml = r#"
name = "my-package"
version = "1.2.3"
"#;
        let manifest = parse_manifest(toml).expect("valid manifest should parse");
        assert_eq!(manifest.name, "my-package");
        assert_eq!(manifest.version, "1.2.3");
        assert!(manifest.author.is_none());
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn parse_manifest_reads_optional_fields() {
        let toml = r#"
name = "pkg"
version = "0.1.0"
author = "Alice"
description = "A test package"
"#;
        let manifest = parse_manifest(toml).expect("valid manifest should parse");
        assert_eq!(manifest.author.as_deref(), Some("Alice"));
        assert_eq!(manifest.description.as_deref(), Some("A test package"));
    }

    #[test]
    fn parse_manifest_reads_dependencies_section() {
        let toml = r#"
name = "pkg"
version = "0.1.0"

[dependencies]
serde = ">=1.0.0"
tokio = "=0.2.3"
"#;
        let manifest = parse_manifest(toml).expect("manifest with deps should parse");
        assert_eq!(manifest.dependencies.len(), 2);
        let serde = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "serde")
            .expect("serde dep should exist");
        assert_eq!(serde.version_constraint, ">=1.0.0");
    }

    #[test]
    fn parse_manifest_returns_error_when_name_missing() {
        let toml = "version = \"1.0.0\"\n";
        let result = parse_manifest(toml);
        assert!(result.is_err(), "missing name should fail");
    }

    #[test]
    fn parse_manifest_returns_error_when_version_missing() {
        let toml = "name = \"pkg\"\n";
        let result = parse_manifest(toml);
        assert!(result.is_err(), "missing version should fail");
    }

    #[test]
    fn serialize_manifest_round_trips() {
        let toml = r#"name = "pkg"
version = "0.1.0"
author = "Bob"

[dependencies]
lib = ">=2.0.0"
"#;
        let manifest = parse_manifest(toml).expect("parse should succeed");
        let serialized = serialize_manifest(&manifest);
        let reparsed = parse_manifest(&serialized).expect("re-parse should succeed");
        assert_eq!(manifest.name, reparsed.name);
        assert_eq!(manifest.version, reparsed.version);
        assert_eq!(manifest.dependencies.len(), reparsed.dependencies.len());
    }

    // ─── registry tests ───────────────────────────────────────────────────────

    #[test]
    fn mock_registry_lists_registered_versions() {
        let mut registry = MockRegistry::new();
        registry.register(PackageEntry {
            name: String::from("alpha"),
            version: String::from("1.0.0"),
            url: String::from("https://example.com/alpha-1.0.0.tar"),
            checksum: String::from("abc"),
        });
        registry.register(PackageEntry {
            name: String::from("alpha"),
            version: String::from("2.0.0"),
            url: String::from("https://example.com/alpha-2.0.0.tar"),
            checksum: String::from("def"),
        });

        let versions = registry.list_versions("alpha").expect("should find alpha");
        assert_eq!(versions.len(), 2);
    }

    #[test]
    fn mock_registry_returns_not_found_for_unknown_package() {
        let registry = MockRegistry::new();
        let result = registry.list_versions("ghost");
        assert!(result.is_err(), "unknown package should return error");
    }

    #[test]
    fn mock_registry_fetch_metadata_succeeds_for_known_entry() {
        let mut registry = MockRegistry::new();
        registry.register(PackageEntry {
            name: String::from("beta"),
            version: String::from("3.1.0"),
            url: String::from("https://example.com/beta-3.1.0.tar"),
            checksum: String::from("xyz"),
        });

        let entry = registry
            .fetch_metadata("beta", "3.1.0")
            .expect("should fetch beta 3.1.0");
        assert_eq!(entry.version, "3.1.0");
    }

    // ─── resolver tests ───────────────────────────────────────────────────────

    #[test]
    fn resolver_picks_highest_compatible_version() {
        let mut registry = MockRegistry::new();
        for v in ["1.0.0", "1.5.0", "2.0.0"] {
            registry.register(PackageEntry {
                name: String::from("libx"),
                version: v.to_owned(),
                url: String::from("https://example.com/libx"),
                checksum: String::new(),
            });
        }

        let toml = "name = \"app\"\nversion = \"1.0.0\"\n\n[dependencies]\nlibx = \">=1.0.0\"\n";
        let manifest = parse_manifest(toml).expect("manifest ok");
        let graph = resolve_manifest_deps(&manifest, &registry).expect("resolve ok");

        let node = graph.nodes.get("libx").expect("libx should be resolved");
        assert_eq!(node.version, "2.0.0", "highest compatible version selected");
    }

    #[test]
    fn resolver_returns_error_when_no_version_matches() {
        let mut registry = MockRegistry::new();
        registry.register(PackageEntry {
            name: String::from("old"),
            version: String::from("0.1.0"),
            url: String::from("https://example.com/old"),
            checksum: String::new(),
        });

        let toml = "name = \"app\"\nversion = \"1.0.0\"\n\n[dependencies]\nold = \">=5.0.0\"\n";
        let manifest = parse_manifest(toml).expect("manifest ok");
        let result = resolve_manifest_deps(&manifest, &registry);
        assert!(result.is_err(), "no matching version should fail");
    }

    #[test]
    fn resolver_returns_error_when_package_not_in_registry() {
        let registry = MockRegistry::new();
        let toml = "name = \"app\"\nversion = \"1.0.0\"\n\n[dependencies]\nghost = \">=1.0.0\"\n";
        let manifest = parse_manifest(toml).expect("manifest ok");
        let result = resolve_manifest_deps(&manifest, &registry);
        assert!(result.is_err(), "missing package should fail");
    }

    #[test]
    fn parse_constraint_parses_gte_operator() {
        let constraint = parse_constraint(">=1.2.3").expect("valid constraint");
        assert_eq!(constraint.clauses.len(), 1);
    }

    #[test]
    fn parse_constraint_parses_exact_equality() {
        let constraint = parse_constraint("=0.5.0").expect("valid constraint");
        assert_eq!(constraint.clauses.len(), 1);
    }

    #[test]
    fn parse_constraint_parses_bare_version_as_exact() {
        let constraint = parse_constraint("1.0.0").expect("bare version");
        assert_eq!(constraint.clauses.len(), 1);
    }

    // ─── installer tests ──────────────────────────────────────────────────────

    #[test]
    fn installer_executes_plan_with_mock_downloader() {
        use crate::package_manager::installer::InstallPlan;

        let downloader = MockDownloader::new();
        let pkg_installer = Installer::new(&downloader);
        let plan = vec![InstallPlan {
            name: String::from("alpha"),
            version: String::from("1.0.0"),
            url: String::from("https://example.com/alpha"),
        }];
        let names = pkg_installer
            .execute(&plan)
            .expect("install should succeed");
        assert_eq!(names, vec!["alpha"]);
    }

    #[test]
    fn installer_returns_error_when_downloader_fails() {
        use crate::package_manager::installer::InstallPlan;

        let downloader = FailingDownloader::new();
        let pkg_installer = Installer::new(&downloader);
        let plan = vec![InstallPlan {
            name: String::from("pkg"),
            version: String::from("1.0.0"),
            url: String::from("https://example.com/pkg"),
        }];
        assert!(
            pkg_installer.execute(&plan).is_err(),
            "failing downloader should error"
        );
    }

    #[test]
    fn installer_plan_from_graph_maps_nodes_to_steps() {
        let mut registry = MockRegistry::new();
        registry.register(PackageEntry {
            name: String::from("mylib"),
            version: String::from("1.0.0"),
            url: String::from("https://example.com/mylib"),
            checksum: String::new(),
        });

        let toml = "name = \"app\"\nversion = \"1.0.0\"\n\n[dependencies]\nmylib = \"=1.0.0\"\n";
        let manifest = parse_manifest(toml).expect("manifest ok");
        let graph = resolve_manifest_deps(&manifest, &registry).expect("resolve ok");

        let downloader = MockDownloader::new();
        let pkg_installer = Installer::new(&downloader);
        let plan = pkg_installer.plan_from_graph(&graph);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].name, "mylib");
    }

    // ─── publisher tests ──────────────────────────────────────────────────────

    #[test]
    fn publisher_publishes_valid_manifest_with_mock_uploader() {
        let toml = "name = \"mypkg\"\nversion = \"2.0.0\"\n";
        let manifest = parse_manifest(toml).expect("manifest ok");
        let uploader = MockUploader::new();
        let publisher = Publisher::new(&uploader);
        let size = publisher
            .publish_manifest(&manifest)
            .expect("publish should succeed");
        assert!(size > 0, "archive size should be non-zero");
    }

    #[test]
    fn publisher_returns_error_when_uploader_fails() {
        let toml = "name = \"mypkg\"\nversion = \"2.0.0\"\n";
        let manifest = parse_manifest(toml).expect("manifest ok");
        let uploader = FailingUploader::new();
        let publisher = Publisher::new(&uploader);
        assert!(
            publisher.publish_manifest(&manifest).is_err(),
            "failing uploader should error"
        );
    }

    #[test]
    fn publisher_plan_returns_error_for_empty_name() {
        use crate::package_manager::manifest::Manifest;
        use alloc::vec::Vec;

        let manifest = Manifest {
            name: String::new(),
            version: String::from("1.0.0"),
            author: None,
            description: None,
            dependencies: Vec::new(),
        };
        let uploader = MockUploader::new();
        let publisher = Publisher::new(&uploader);
        assert!(publisher.plan(&manifest).is_err(), "empty name should fail");
    }

    // ─── commands tests ───────────────────────────────────────────────────────

    #[test]
    fn cmd_init_returns_success_with_manifest_text() {
        let registry = MockRegistry::new();
        let cmd = PkgCommand::Init {
            name: String::from("new-project"),
        };
        let result = dispatch_pkg_command(&cmd, &registry);
        assert!(result.success, "init should succeed");
        assert!(
            result.output.iter().any(|l| l.contains("new-project")),
            "output should mention project name"
        );
    }

    #[test]
    fn cmd_add_returns_dependency_line() {
        let registry = MockRegistry::new();
        let cmd = PkgCommand::Add {
            package: String::from("serde"),
            version_constraint: String::from(">=1.0.0"),
        };
        let result = dispatch_pkg_command(&cmd, &registry);
        assert!(result.success, "add should succeed");
        assert!(
            result.output.iter().any(|l| l.contains("serde")),
            "output should mention package name"
        );
    }

    #[test]
    fn cmd_remove_returns_confirmation() {
        let registry = MockRegistry::new();
        let cmd = PkgCommand::Remove {
            package: String::from("tokio"),
        };
        let result = dispatch_pkg_command(&cmd, &registry);
        assert!(result.success, "remove should succeed");
        assert!(
            result.output.iter().any(|l| l.contains("tokio")),
            "output should mention package"
        );
    }

    #[test]
    fn cmd_install_succeeds_with_registry_packages() {
        let mut registry = MockRegistry::new();
        registry.register(PackageEntry {
            name: String::from("utils"),
            version: String::from("1.0.0"),
            url: String::from("https://example.com/utils"),
            checksum: String::new(),
        });

        let toml = "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nutils = \"=1.0.0\"\n";
        let cmd = PkgCommand::Install {
            manifest_toml: toml.to_owned(),
        };
        let result = dispatch_pkg_command(&cmd, &registry);
        assert!(result.success, "install should succeed");
        assert!(
            result.output.iter().any(|l| l.contains("utils")),
            "output should list installed package"
        );
    }

    #[test]
    fn cmd_install_fails_when_package_not_in_registry() {
        let registry = MockRegistry::new();
        let toml = "name = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nmissing = \">=1.0.0\"\n";
        let cmd = PkgCommand::Install {
            manifest_toml: toml.to_owned(),
        };
        let result = dispatch_pkg_command(&cmd, &registry);
        assert!(!result.success, "install with missing dep should fail");
    }

    #[test]
    fn cmd_publish_succeeds_for_valid_manifest() {
        let registry = MockRegistry::new();
        let toml = "name = \"mypkg\"\nversion = \"1.0.0\"\n";
        let cmd = PkgCommand::Publish {
            manifest_toml: toml.to_owned(),
        };
        let result = dispatch_pkg_command(&cmd, &registry);
        assert!(result.success, "publish should succeed");
    }

    // ─── platform / target tests ──────────────────────────────────────────────

    #[test]
    fn platform_extension_linux_is_so() {
        let triple = parse_target_triple("x86_64-linux").expect("valid triple");
        assert_eq!(dynamic_lib_extension(&triple), ".so");
        assert_eq!(triple.platform, Platform::Linux);
    }

    #[test]
    fn platform_extension_macos_is_dylib() {
        let triple = parse_target_triple("aarch64-darwin").expect("valid triple");
        assert_eq!(dynamic_lib_extension(&triple), ".dylib");
        assert_eq!(triple.platform, Platform::MacOs);
    }

    #[test]
    fn platform_extension_windows_is_dll() {
        let triple = parse_target_triple("x86_64-windows").expect("valid triple");
        assert_eq!(dynamic_lib_extension(&triple), ".dll");
        assert_eq!(triple.platform, Platform::Windows);
    }

    #[test]
    fn all_three_platforms_have_correct_extensions() {
        let cases = [
            ("x86_64-linux", ".so"),
            ("aarch64-darwin", ".dylib"),
            ("x86_64-windows", ".dll"),
        ];
        for (triple_str, expected_ext) in cases {
            let parse_result = parse_target_triple(triple_str);
            assert!(parse_result.is_ok(), "triple {triple_str} should parse");
            if let Ok(triple) = parse_result {
                let ext = dynamic_lib_extension(&triple);
                assert_eq!(ext, expected_ext, "extension mismatch for {triple_str}");
            }
        }
    }
}
