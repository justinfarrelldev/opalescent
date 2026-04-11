use crate::codegen::optimization::{apply_optimization_passes, OptimizationLevel};
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;

fn build_constant_add_module(context: &Context) -> inkwell::module::Module<'_> {
    let module = context.create_module("opt_const_add");
    let function_type = context.i64_type().fn_type(&[], false);
    let function = module.add_function("const_add", function_type, None);
    let entry = context.append_basic_block(function, "entry");
    let builder = context.create_builder();
    builder.position_at_end(entry);
    let stack_slot = builder.build_alloca(context.i64_type(), "lhs").ok();
    if let Some(slot) = stack_slot {
        let _store = builder.build_store(slot, context.i64_type().const_int(2, false));
        let loaded = builder.build_load(slot, "lhs_load").ok();
        let sum = loaded.and_then(|value| {
            builder
                .build_int_add(
                    value.into_int_value(),
                    context.i64_type().const_int(3, false),
                    "sum",
                )
                .ok()
        });
        if let Some(value) = sum {
            let _ret = builder.build_return(Some(&value));
            return module;
        }
    }
    let return_value = context.i64_type().const_zero();
    let _ret = builder.build_return(Some(&return_value));
    module
}

fn build_dead_store_module(context: &Context) -> inkwell::module::Module<'_> {
    let module = context.create_module("opt_dead_store");
    let function_type = context.i64_type().fn_type(&[], false);
    let function = module.add_function("dead_store", function_type, None);
    let entry = context.append_basic_block(function, "entry");
    let builder = context.create_builder();
    builder.position_at_end(entry);
    let stack_slot = builder.build_alloca(context.i64_type(), "tmp").ok();
    if let Some(slot) = stack_slot {
        let _first_store = builder.build_store(slot, context.i64_type().const_int(11, false));
        let _second_store = builder.build_store(slot, context.i64_type().const_int(12, false));
    }
    let _ret = builder.build_return(Some(&context.i64_type().const_int(12, false)));
    module
}

fn build_inline_module(context: &Context) -> inkwell::module::Module<'_> {
    let module = context.create_module("opt_inline");
    let i64_type = context.i64_type();

    let inline_target_type = i64_type.fn_type(&[i64_type.into()], false);
    let inline_target = module.add_function("small_add", inline_target_type, None);
    let always_inline_kind = Attribute::get_named_enum_kind_id("alwaysinline");
    if always_inline_kind != 0 {
        let always_inline = context.create_enum_attribute(always_inline_kind, 0);
        inline_target.add_attribute(AttributeLoc::Function, always_inline);
    }
    let inline_target_entry = context.append_basic_block(inline_target, "entry");
    let builder = context.create_builder();
    builder.position_at_end(inline_target_entry);
    let param = inline_target
        .get_nth_param(0)
        .map(|value| value.into_int_value());
    let add_result = param.and_then(|value| {
        builder
            .build_int_add(value, i64_type.const_int(1, false), "inc")
            .ok()
    });
    let return_value = add_result.unwrap_or_else(|| i64_type.const_zero());
    let _callee_ret = builder.build_return(Some(&return_value));

    let call_wrapper_type = i64_type.fn_type(&[], false);
    let call_wrapper = module.add_function("caller", call_wrapper_type, None);
    let call_wrapper_entry = context.append_basic_block(call_wrapper, "entry");
    builder.position_at_end(call_wrapper_entry);
    let call_site = builder
        .build_call(
            inline_target,
            &[i64_type.const_int(41, false).into()],
            "call_small",
        )
        .ok();
    let called_value = call_site
        .and_then(|call| call.try_as_basic_value().basic())
        .map_or(i64_type.const_zero(), |value| value.into_int_value());
    let _caller_ret = builder.build_return(Some(&called_value));

    module
}

#[test]
fn test_optimization_level_o0_vs_o2_changes_ir() {
    let context = Context::create();
    let debug_module = build_constant_add_module(&context);
    let release_module = build_constant_add_module(&context);

    apply_optimization_passes(&debug_module, OptimizationLevel::Debug);
    apply_optimization_passes(&release_module, OptimizationLevel::Release);

    let debug_ir = debug_module.print_to_string().to_string();
    let release_ir = release_module.print_to_string().to_string();

    assert_ne!(
        debug_ir, release_ir,
        "O0 and O2 should not produce identical IR for foldable constants"
    );
}

#[test]
fn test_release_optimization_folds_constants() {
    let context = Context::create();
    let module = build_constant_add_module(&context);

    apply_optimization_passes(&module, OptimizationLevel::Release);

    let ir = module.print_to_string().to_string();
    assert!(
        !ir.contains("add i64"),
        "constant folding should remove integer add instruction in release mode: {ir}"
    );
    assert!(
        ir.contains("ret i64 5"),
        "constant folding should fold 2 + 3 to ret i64 5: {ir}"
    );
}

#[test]
fn test_release_optimization_removes_dead_store() {
    let context = Context::create();
    let module = build_dead_store_module(&context);

    apply_optimization_passes(&module, OptimizationLevel::Release);

    let ir = module.print_to_string().to_string();
    assert!(
        !ir.contains("store i64"),
        "dead code elimination should remove unused stores: {ir}"
    );
}

#[test]
fn test_release_optimization_inlines_small_functions() {
    let context = Context::create();
    let module = build_inline_module(&context);

    apply_optimization_passes(&module, OptimizationLevel::Release);

    let ir = module.print_to_string().to_string();
    assert!(
        !ir.contains("call i64 @small_add"),
        "always inliner should inline small_add into caller: {ir}"
    );
}
