//! LLVM code generation context management.

use inkwell::OptimizationLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};

/// Error type for code generation context creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    /// LLVM target not supported on this platform.
    UnsupportedTarget(String),
}

/// Shared LLVM handles used by code generation passes.
pub struct CodegenContext<'context> {
    /// Owning LLVM context for all IR allocations.
    pub context: &'context Context,
    /// Current compilation unit module.
    pub module: Module<'context>,
    /// IR builder used by lowering routines.
    pub builder: Builder<'context>,
    /// Optional target machine for target-aware code generation.
    pub target_machine: Option<TargetMachine>,
}

impl<'context> CodegenContext<'context> {
    /// Create a new codegen context with an explicit target triple.
    ///
    /// # Arguments
    ///
    /// * `context` - The LLVM context for IR allocations
    /// * `module_name` - Name of the compilation unit module
    /// * `target` - The target triple to compile for
    ///
    /// # Errors
    ///
    /// Returns `CodegenError::UnsupportedTarget` if LLVM does not support the target.
    pub fn for_triple(
        context: &'context Context,
        module_name: &str,
        target: &crate::build_system::targets::TargetTriple,
    ) -> Result<Self, CodegenError> {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let llvm_triple_str = target.to_llvm_string();
        let inkwell_triple = TargetTriple::create(&llvm_triple_str);
        module.set_triple(&inkwell_triple);

        let target_machine = Self::create_target_machine_for_triple(&inkwell_triple)
            .ok_or_else(|| CodegenError::UnsupportedTarget(llvm_triple_str))?;

        Ok(Self {
            context,
            module,
            builder,
            target_machine: Some(target_machine),
        })
    }

    /// Create a new codegen context with host target triple.
    #[must_use]
    pub fn new(context: &'context Context, module_name: &str) -> Self {
        let host_triple = crate::build_system::targets::TargetTriple {
            arch: if cfg!(target_arch = "aarch64") {
                crate::build_system::targets::Architecture::Aarch64
            } else {
                crate::build_system::targets::Architecture::X86_64
            },
            platform: if cfg!(target_os = "windows") {
                crate::build_system::targets::Platform::Windows
            } else if cfg!(target_os = "macos") {
                crate::build_system::targets::Platform::MacOs
            } else {
                crate::build_system::targets::Platform::Linux
            },
            env: if cfg!(target_env = "msvc") {
                Some(crate::build_system::targets::TripleEnv::Msvc)
            } else if cfg!(target_env = "musl") {
                Some(crate::build_system::targets::TripleEnv::Musl)
            } else {
                Some(crate::build_system::targets::TripleEnv::Gnu)
            },
        };
        Self::for_triple(context, module_name, &host_triple)
            .expect("host target should always be supported")
    }

    /// Return the active target triple configured on the module.
    #[must_use]
    pub fn target_triple(&self) -> TargetTriple {
        self.module.get_triple()
    }

    /// Try constructing a target machine for a given triple.
    fn create_target_machine_for_triple(triple: &TargetTriple) -> Option<TargetMachine> {
        Target::initialize_all(&InitializationConfig::default());

        let target = Target::from_triple(triple).ok()?;
        target.create_target_machine(
            triple,
            "generic",
            "",
            OptimizationLevel::None,
            RelocMode::Default,
            CodeModel::Default,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_system::targets::parse_target_triple;

    #[test]
    fn for_triple_windows_msvc() {
        let ctx = Context::create();
        let target = parse_target_triple("x86_64-pc-windows-msvc").expect("valid triple");
        let result = CodegenContext::for_triple(&ctx, "test_module", &target);
        assert!(result.is_ok(), "should construct context for Windows MSVC");
        let codegen_ctx = result.unwrap();
        let triple_str = codegen_ctx.target_triple().to_string();
        assert!(
            triple_str.contains("windows"),
            "LLVM triple should contain 'windows', got: {triple_str}"
        );
    }

    #[test]
    fn for_triple_linux_gnu() {
        let ctx = Context::create();
        let target = parse_target_triple("x86_64-unknown-linux-gnu").expect("valid triple");
        let result = CodegenContext::for_triple(&ctx, "test_module", &target);
        assert!(result.is_ok(), "should construct context for Linux GNU");
        let codegen_ctx = result.unwrap();
        let triple_str = codegen_ctx.target_triple().to_string();
        assert!(
            triple_str.contains("linux"),
            "LLVM triple should contain 'linux', got: {triple_str}"
        );
    }

    #[test]
    fn for_triple_error_type_exists() {
        let err = CodegenError::UnsupportedTarget("test-triple".to_string());
        match err {
            CodegenError::UnsupportedTarget(triple) => {
                assert_eq!(triple, "test-triple");
            }
        }
    }
}
