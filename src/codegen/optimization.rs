use inkwell::OptimizationLevel as LlvmOptimizationLevel;
use inkwell::module::Module;
use inkwell::passes::{PassManager, PassManagerBuilder};

#[doc = "Optimization level selection for LLVM module pass pipelines."]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimizationLevel {
    #[doc = "Debug-friendly mode with no optimization passes applied."]
    Debug,
    #[doc = "Release mode using an O2-class optimization pipeline."]
    Release,
}

#[doc = "Apply LLVM optimization passes to a module at the selected optimization level."]
pub fn apply_optimization_passes(module: &Module<'_>, level: OptimizationLevel) {
    let pass_manager = PassManager::create(());

    if matches!(level, OptimizationLevel::Release) {
        let pass_manager_builder = PassManagerBuilder::create();
        pass_manager_builder.set_optimization_level(llvm_optimization_level(level));
        pass_manager_builder.set_inliner_with_threshold(225);
        pass_manager_builder.populate_module_pass_manager(&pass_manager);

        pass_manager.add_instruction_simplify_pass();
        pass_manager.add_sccp_pass();
        pass_manager.add_cfg_simplification_pass();
        pass_manager.add_dead_store_elimination_pass();
        pass_manager.add_aggressive_dce_pass();
        pass_manager.add_global_dce_pass();
        pass_manager.add_strip_dead_prototypes_pass();
        pass_manager.add_dead_arg_elimination_pass();
        pass_manager.add_always_inliner_pass();
    }

    let _modified = pass_manager.run_on(module);
}

#[doc = "Map Opalescent optimization levels to LLVM pass-builder optimization levels."]
const fn llvm_optimization_level(level: OptimizationLevel) -> LlvmOptimizationLevel {
    match level {
        OptimizationLevel::Debug => LlvmOptimizationLevel::None,
        OptimizationLevel::Release => LlvmOptimizationLevel::Aggressive,
    }
}
