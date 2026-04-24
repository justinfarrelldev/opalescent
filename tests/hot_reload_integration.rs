#![cfg(feature = "integration")]

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn hot_reload_demo_project_exists() {
        assert!(
            Path::new("test-projects/hot-reload-demo/opal.toml").exists(),
            "hot-reload-demo opal.toml must exist"
        );
        assert!(
            Path::new("test-projects/hot-reload-demo/src/greeting.op").exists(),
            "hot-reload-demo src/greeting.op must exist"
        );
    }

    #[cfg(unix)]
    #[test]
    fn hot_reload_fs_loader_loads_and_unloads_shared_library() {
        use opalescent::hot_reload::loader::{FsModuleLoader, ModuleLoader};
        use std::fs;
        use std::process::Command;

        let temp_dir = std::env::temp_dir();
        let lib_path = temp_dir.join(format!(
            "opalescent_hot_reload_integration_{}.so",
            std::process::id()
        ));
        let c_path = lib_path.with_extension("c");

        let c_source = "void module_entry(void) {}\n";
        fs::write(&c_path, c_source).expect("write C source");

        let output = Command::new("cc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-o")
            .arg(&lib_path)
            .arg(&c_path)
            .output()
            .expect("invoke cc");

        drop(fs::remove_file(&c_path));

        assert!(
            output.status.success(),
            "cc compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(lib_path.exists(), "compiled shared library must exist");

        let mut loader = FsModuleLoader::new();
        let module_name = lib_path.to_string_lossy().to_string();

        let load_result = loader.load_module(&module_name);
        assert!(
            load_result.is_ok(),
            "FsModuleLoader should load real shared library: {load_result:?}"
        );

        let loaded_module = load_result.unwrap();
        assert_eq!(
            loaded_module.module_name, module_name,
            "loaded module name should match input path"
        );

        let unload_result = loader.unload_module(&module_name);
        assert!(
            unload_result.is_ok(),
            "FsModuleLoader should unload module without error: {unload_result:?}"
        );

        drop(fs::remove_file(&lib_path));
    }

    #[cfg(unix)]
    #[test]
    fn hot_reload_fs_loader_returns_error_for_missing_library() {
        use opalescent::hot_reload::loader::{FsModuleLoader, HotReloadError, ModuleLoader};

        let mut loader = FsModuleLoader::new();
        let result = loader.load_module("/definitely/missing/opalescent_hot_reload_integration.so");
        assert!(
            matches!(result, Err(HotReloadError::ModuleLoadFailed { .. })),
            "missing library path must return ModuleLoadFailed"
        );
    }
}
