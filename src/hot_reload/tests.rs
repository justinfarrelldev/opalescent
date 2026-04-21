extern crate alloc;

use crate::hot_reload::abi::{
    ExportedFunction, FunctionSignature, ModuleVTable, PodLayout, generate_abi_signature,
    signatures_compatible,
};
use crate::hot_reload::cache::AbiSignatureCache;
use crate::hot_reload::change_detection::{
    FileChangeEvent, FileWatcher, MockFileWatcher, PollingFileWatcher,
};
use crate::hot_reload::classifier::{ChangeClassifier, HotReloadCategory};
use crate::hot_reload::dependency_graph::ModuleDependencyGraph;
use crate::hot_reload::guard::{AbiGuard, AbiGuardResult, FallbackRestartTrigger};
use crate::hot_reload::loader::{
    FsModuleLoader, HostProcess, HotReloadError, LoadedModule, ModuleLoader, hot_swap_module,
};
use crate::hot_reload::recovery::ErrorRecovery;
use crate::hot_reload::state::{HostState, StatePreserver};
use crate::hot_reload::version::{ModuleVersion, versioned_module_name};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use std::fs;
use std::path::Path;

extern "C" fn noop_entry() {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MockModuleLoader {
    modules: BTreeMap<String, LoadedModule>,
    unload_calls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveringMockLoader {
    modules: BTreeMap<String, LoadedModule>,
    fail_load_for: Option<String>,
    unload_calls: Vec<String>,
    load_calls: Vec<String>,
}

impl RecoveringMockLoader {
    fn with_modules(modules: BTreeMap<String, LoadedModule>) -> Self {
        Self {
            modules,
            fail_load_for: None,
            unload_calls: Vec::new(),
            load_calls: Vec::new(),
        }
    }

    fn fail_load_for(mut self, module_name: &str) -> Self {
        self.fail_load_for = Some(module_name.to_owned());
        self
    }
}

impl ModuleLoader for RecoveringMockLoader {
    fn load_module(&mut self, module_name: &str) -> Result<LoadedModule, HotReloadError> {
        self.load_calls.push(module_name.to_owned());

        if self
            .fail_load_for
            .as_ref()
            .is_some_and(|candidate| candidate == module_name)
        {
            return Err(HotReloadError::ModuleLoadFailed {
                module_name: module_name.to_owned(),
                reason: String::from("simulated load failure"),
            });
        }

        self.modules
            .get(module_name)
            .cloned()
            .ok_or_else(|| HotReloadError::ModuleLoadFailed {
                module_name: module_name.to_owned(),
                reason: String::from("module not found in recovery mock loader"),
            })
    }

    fn unload_module(&mut self, module_name: &str) -> Result<(), HotReloadError> {
        self.unload_calls.push(module_name.to_owned());
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MockHostState {
    values: BTreeMap<String, String>,
}

impl HostState for MockHostState {
    fn serialize(&self) -> Vec<u8> {
        let mut encoded = String::new();

        for (key, value) in &self.values {
            encoded.push_str(key);
            encoded.push('=');
            encoded.push_str(value);
            encoded.push('\n');
        }

        encoded.into_bytes()
    }

    fn deserialize(data: &[u8]) -> Result<Self, crate::hot_reload::state::StateError> {
        let input = core::str::from_utf8(data)
            .map_err(|_utf8_error| crate::hot_reload::state::StateError::InvalidEncoding)?;
        let mut values = BTreeMap::new();

        for line in input.lines() {
            let mut split = line.splitn(2, '=');
            let key = split
                .next()
                .ok_or(crate::hot_reload::state::StateError::InvalidFormat)?;
            let value = split
                .next()
                .ok_or(crate::hot_reload::state::StateError::InvalidFormat)?;
            values.insert(key.to_owned(), value.to_owned());
        }

        Ok(Self { values })
    }
}

impl MockModuleLoader {
    fn with_modules(modules: BTreeMap<String, LoadedModule>) -> Self {
        Self {
            modules,
            unload_calls: Vec::new(),
        }
    }
}

impl ModuleLoader for MockModuleLoader {
    fn load_module(&mut self, module_name: &str) -> Result<LoadedModule, HotReloadError> {
        self.modules
            .get(module_name)
            .cloned()
            .ok_or_else(|| HotReloadError::ModuleLoadFailed {
                module_name: module_name.to_owned(),
                reason: String::from("module not found in mock loader"),
            })
    }

    fn unload_module(&mut self, module_name: &str) -> Result<(), HotReloadError> {
        self.unload_calls.push(module_name.to_owned());
        Ok(())
    }
}

#[test]
fn abi_signature_generation_is_deterministic_for_same_inputs() {
    let exported_functions = [make_exported_function(
        "compute",
        vec![String::from("int32")],
        vec![String::from("int32")],
    )];

    let mut pod_layouts = BTreeMap::new();
    pod_layouts.insert(String::from("Point"), PodLayout { size: 8, align: 4 });

    let first = generate_abi_signature(&exported_functions, &pod_layouts);
    let second = generate_abi_signature(&exported_functions, &pod_layouts);

    assert_eq!(
        first.abi_hash, second.abi_hash,
        "ABI hash must be deterministic"
    );
    assert!(
        signatures_compatible(&first, &second),
        "identical signatures must be compatible"
    );
}

#[test]
fn abi_signature_detects_function_signature_changes() {
    let old_signature = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int32")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );
    let new_signature = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int64")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );

    assert_ne!(
        old_signature.abi_hash, new_signature.abi_hash,
        "function parameter changes should alter ABI hash"
    );
    assert!(
        !signatures_compatible(&old_signature, &new_signature),
        "changed signatures must be incompatible"
    );
}

#[test]
fn module_version_formats_zero_padded_identifier() {
    let version = ModuleVersion::new(1);
    assert_eq!(
        format!("{version}"),
        String::from("v0001"),
        "version format should be vNNNN"
    );

    let versioned_name = versioned_module_name("logic", version);
    let expected_extension = if cfg!(target_os = "windows") {
        ".dll"
    } else if cfg!(target_os = "macos") {
        ".dylib"
    } else {
        ".so"
    };
    assert_eq!(
        versioned_name,
        format!("logic_v0001{expected_extension}"),
        "versioned module file name must include suffix and extension"
    );
}

#[test]
fn polling_file_watcher_detects_file_creation_changes() {
    let mut test_file = std::env::temp_dir();
    test_file.push(format!(
        "opalescent_hot_reload_watcher_{}.tmp",
        std::process::id()
    ));
    if test_file.exists() {
        let _remove_existing = fs::remove_file(&test_file);
    }

    let watched_path = path_to_string(&test_file);
    let mut watcher = PollingFileWatcher::new(vec![watched_path.clone()]);
    let started = watcher.start();
    assert!(started.is_ok(), "polling watcher should start");

    let initial_changes = watcher.poll_changes();
    assert!(
        initial_changes.is_empty(),
        "no changes should be reported before file exists"
    );

    let write_result = fs::write(&test_file, b"hot reload");
    assert!(
        write_result.is_ok(),
        "test file write should succeed for polling change test"
    );

    let changed = watcher.poll_changes();
    assert!(
        changed.iter().any(|event| event.file_path == watched_path),
        "watcher should report created file as changed"
    );

    let _cleanup = fs::remove_file(&test_file);
}

#[test]
fn fs_module_loader_returns_load_error_for_missing_library_path() {
    let mut loader = FsModuleLoader::new();
    let result = loader.load_module("/definitely/missing/opalescent/lib_missing_hot_reload.so");
    assert!(
        matches!(result, Err(HotReloadError::ModuleLoadFailed { .. })),
        "missing library path must return ModuleLoadFailed"
    );
}

#[test]
fn host_process_hot_swap_replaces_old_module_when_abi_matches() {
    let abi = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int32")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );

    let mut modules = BTreeMap::new();
    modules.insert(
        String::from("logic_v0001.so"),
        LoadedModule {
            module_name: String::from("logic_v0001.so"),
            vtable: ModuleVTable {
                module_entry: noop_entry,
            },
            abi_signature: abi.clone(),
        },
    );
    modules.insert(
        String::from("logic_v0002.so"),
        LoadedModule {
            module_name: String::from("logic_v0002.so"),
            vtable: ModuleVTable {
                module_entry: noop_entry,
            },
            abi_signature: abi,
        },
    );

    let mut loader = MockModuleLoader::with_modules(modules);
    let mut host = HostProcess::new();

    let initial = hot_swap_module(&mut host, &mut loader, "logic_v0001.so");
    assert!(initial.is_ok(), "first load should succeed");

    let swap = hot_swap_module(&mut host, &mut loader, "logic_v0002.so");
    assert!(swap.is_ok(), "compatible module should hot swap");

    assert_eq!(
        loader.unload_calls,
        vec![String::from("logic_v0001.so")],
        "old module should be unloaded exactly once"
    );
}

#[test]
fn host_process_hot_swap_rejects_incompatible_abi() {
    let old_abi = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int32")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );
    let new_abi = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int64")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );

    let mut modules = BTreeMap::new();
    modules.insert(
        String::from("logic_v0001.so"),
        LoadedModule {
            module_name: String::from("logic_v0001.so"),
            vtable: ModuleVTable {
                module_entry: noop_entry,
            },
            abi_signature: old_abi,
        },
    );
    modules.insert(
        String::from("logic_v0002.so"),
        LoadedModule {
            module_name: String::from("logic_v0002.so"),
            vtable: ModuleVTable {
                module_entry: noop_entry,
            },
            abi_signature: new_abi,
        },
    );

    let mut loader = MockModuleLoader::with_modules(modules);
    let mut host = HostProcess::new();

    let initial = hot_swap_module(&mut host, &mut loader, "logic_v0001.so");
    assert!(initial.is_ok(), "initial module load should succeed");

    let swap = hot_swap_module(&mut host, &mut loader, "logic_v0002.so");
    assert!(swap.is_err(), "incompatible module should fail hot swap");

    let error = swap.err();
    assert!(
        matches!(error, Some(HotReloadError::RequiresFullRestart)),
        "swap failure should request full restart for incompatible ABI"
    );
    assert_eq!(
        loader.unload_calls,
        vec![String::from("logic_v0002.so")],
        "failed new module should be unloaded while old stays active"
    );
}

fn make_exported_function(
    name: &str,
    parameters: Vec<String>,
    return_types: Vec<String>,
) -> ExportedFunction {
    ExportedFunction {
        name: name.to_owned(),
        signature: FunctionSignature {
            parameters,
            return_types,
        },
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn change_classifier_marks_body_only_hash_change_as_hot_swappable() {
    let old_signature = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int32")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );
    let mut new_signature = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int32")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );
    new_signature.abi_hash = old_signature.abi_hash.wrapping_add(1);

    let category = ChangeClassifier::classify(&old_signature, &new_signature);
    assert_eq!(
        category,
        HotReloadCategory::HotSwappable,
        "body-only hash changes should remain hot-swappable"
    );

    assert_eq!(
        old_signature.exported_functions, new_signature.exported_functions,
        "test sanity: function signatures should match"
    );
}

#[test]
fn change_classifier_marks_function_signature_change_as_requires_restart() {
    let old_signature = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int32")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );
    let new_signature = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int64")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );

    let category = ChangeClassifier::classify(&old_signature, &new_signature);
    assert_eq!(
        category,
        HotReloadCategory::RequiresRestart,
        "function signature changes should require restart"
    );
}

#[test]
fn change_classifier_marks_type_layout_change_as_full_restart() {
    let mut old_pods = BTreeMap::new();
    old_pods.insert(String::from("Point"), PodLayout { size: 8, align: 4 });
    let mut new_pods = BTreeMap::new();
    new_pods.insert(String::from("Point"), PodLayout { size: 16, align: 8 });

    let old_signature = generate_abi_signature(&[], &old_pods);
    let new_signature = generate_abi_signature(&[], &new_pods);

    let category = ChangeClassifier::classify(&old_signature, &new_signature);
    assert_eq!(
        category,
        HotReloadCategory::FullRestart,
        "POD layout changes should force full restart"
    );
}

#[test]
fn dependency_graph_returns_transitive_dependents() {
    let mut dependency_graph = ModuleDependencyGraph::new();
    dependency_graph.add_dependency("api", "core");
    dependency_graph.add_dependency("ui", "api");
    dependency_graph.add_dependency("tests", "ui");
    dependency_graph.add_dependency("bench", "core");

    let dependents = dependency_graph.transitive_dependents("core");
    assert_eq!(
        dependents,
        vec![
            String::from("api"),
            String::from("bench"),
            String::from("tests"),
            String::from("ui"),
        ],
        "transitive dependents should include all downstream modules in stable order"
    );
}

#[test]
fn abi_signature_cache_reports_cache_hit_without_recompute() {
    let mut cache = AbiSignatureCache::new();
    let mut call_count = 0_u32;

    let first_signature = cache.get_or_insert_with("logic", || {
        call_count = call_count.saturating_add(1);
        generate_abi_signature(
            &[make_exported_function(
                "compute",
                vec![String::from("int32")],
                vec![String::from("int32")],
            )],
            &BTreeMap::new(),
        )
    });
    let second_signature = cache.get_or_insert_with("logic", || {
        call_count = call_count.saturating_add(1);
        generate_abi_signature(
            &[make_exported_function(
                "compute",
                vec![String::from("int64")],
                vec![String::from("int64")],
            )],
            &BTreeMap::new(),
        )
    });

    assert_eq!(
        call_count, 1,
        "cache hit should avoid recomputation for existing module"
    );
    assert_eq!(
        first_signature.abi_hash, second_signature.abi_hash,
        "cache should return the original stored signature"
    );
}

#[test]
fn mock_file_watcher_returns_queued_changes() {
    let mut watcher = MockFileWatcher::new(vec![
        FileChangeEvent::new("src/module_a.op"),
        FileChangeEvent::new("src/module_b.op"),
    ]);

    let started = watcher.start();
    assert!(started.is_ok(), "mock watcher start should succeed");

    let changes = watcher.poll_changes();
    assert_eq!(
        changes,
        vec![
            FileChangeEvent::new("src/module_a.op"),
            FileChangeEvent::new("src/module_b.op"),
        ],
        "mock watcher should return queued changes"
    );

    let second_poll = watcher.poll_changes();
    assert!(
        second_poll.is_empty(),
        "polling again should drain the queue"
    );
}

#[test]
fn abi_guard_accepts_compatible_signatures() {
    let current = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int32")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );
    let incoming = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int32")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );

    let result = AbiGuard::check(&current, &incoming);
    assert_eq!(
        result,
        AbiGuardResult::Accept,
        "ABI guard must accept compatible signatures"
    );
}

#[test]
fn abi_guard_rejects_incompatible_signatures_and_triggers_fallback_restart() {
    let current = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int32")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );
    let incoming = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int64")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );

    let result = AbiGuard::check(&current, &incoming);
    assert_eq!(
        result,
        AbiGuardResult::Reject,
        "ABI guard must reject incompatible signatures"
    );

    let fallback_result = FallbackRestartTrigger::trigger();
    assert_eq!(
        fallback_result,
        HotReloadError::RequiresFullRestart,
        "fallback trigger must request full restart"
    );
}

#[test]
fn state_preserver_round_trip_preserves_host_state() {
    let mut values = BTreeMap::new();
    values.insert(String::from("counter"), String::from("42"));
    values.insert(String::from("mode"), String::from("debug"));
    let state = MockHostState { values };

    let serialized = StatePreserver::save_state(&state);
    let restored = StatePreserver::restore_state::<MockHostState>(&serialized);
    assert!(
        restored.is_ok(),
        "restoring serialized state should succeed"
    );

    if let Ok(restored_state) = restored {
        assert_eq!(
            restored_state, state,
            "state must round-trip through preservation layer"
        );
    }
}

#[test]
fn error_recovery_handles_load_failure_without_stopping_host() {
    let abi = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int32")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );

    let mut modules = BTreeMap::new();
    modules.insert(
        String::from("logic_v0001.so"),
        LoadedModule {
            module_name: String::from("logic_v0001.so"),
            vtable: ModuleVTable {
                module_entry: noop_entry,
            },
            abi_signature: abi,
        },
    );

    let mut loader = MockModuleLoader::with_modules(modules);
    let mut host = HostProcess::new();
    let initial = hot_swap_module(&mut host, &mut loader, "logic_v0001.so");
    assert!(initial.is_ok(), "initial module load should succeed");

    let simulated_failure = HotReloadError::ModuleLoadFailed {
        module_name: String::from("logic_v0002.so"),
        reason: String::from("simulated compilation error"),
    };
    let recovery_result = ErrorRecovery::handle_load_failure(&mut host, &simulated_failure);
    assert!(
        recovery_result.is_ok(),
        "error recovery must keep host running after load failure"
    );
    assert!(
        host.active_module().is_some(),
        "host should still have the previous active module"
    );
}

#[test]
fn error_recovery_rolls_back_partial_swap_to_previous_module() {
    let abi = generate_abi_signature(
        &[make_exported_function(
            "compute",
            vec![String::from("int32")],
            vec![String::from("int32")],
        )],
        &BTreeMap::new(),
    );

    let mut modules = BTreeMap::new();
    modules.insert(
        String::from("logic_v0001.so"),
        LoadedModule {
            module_name: String::from("logic_v0001.so"),
            vtable: ModuleVTable {
                module_entry: noop_entry,
            },
            abi_signature: abi.clone(),
        },
    );
    modules.insert(
        String::from("logic_v0002.so"),
        LoadedModule {
            module_name: String::from("logic_v0002.so"),
            vtable: ModuleVTable {
                module_entry: noop_entry,
            },
            abi_signature: abi,
        },
    );

    let mut loader = RecoveringMockLoader::with_modules(modules).fail_load_for("logic_v0002.so");
    let mut host = HostProcess::new();

    let initial = hot_swap_module(&mut host, &mut loader, "logic_v0001.so");
    assert!(initial.is_ok(), "initial module load should succeed");

    let recovery_result = ErrorRecovery::rollback_partial_swap(
        &mut host,
        &mut loader,
        "logic_v0001.so",
        "logic_v0002.so",
    );
    assert!(
        recovery_result.is_ok(),
        "rollback should restore previous module when candidate fails"
    );
    assert_eq!(
        host.active_module()
            .map(|module| module.module_name.clone()),
        Some(String::from("logic_v0001.so")),
        "rollback must leave previous module active"
    );
}
