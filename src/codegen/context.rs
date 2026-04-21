//! LLVM code generation context management.

use inkwell::OptimizationLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};

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
    /// Create a new codegen context with host target triple.
    #[must_use]
    pub fn new(context: &'context Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let triple = TargetMachine::get_default_triple();
        module.set_triple(&triple);

        let target_machine = Self::create_target_machine_for_triple(&triple);

        Self {
            context,
            module,
            builder,
            target_machine,
        }
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
