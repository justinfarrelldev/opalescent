extern crate alloc;

use crate::hot_reload::abi::{
    generate_abi_signature, signatures_compatible, ExportedFunction, FunctionSignature,
    ModuleVTable, PodLayout,
};
use crate::hot_reload::loader::{
    hot_swap_module, HostProcess, HotReloadError, LoadedModule, ModuleLoader,
};
use crate::hot_reload::version::{versioned_module_name, ModuleVersion};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

extern "C" fn noop_entry() {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MockModuleLoader {
    modules: BTreeMap<String, LoadedModule>,
    unload_calls: Vec<String>,
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
    assert_eq!(
        versioned_name,
        String::from("logic_v0001.so"),
        "versioned module file name must include suffix and extension"
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
        matches!(error, Some(HotReloadError::IncompatibleAbi { .. })),
        "swap failure should report incompatible ABI"
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
