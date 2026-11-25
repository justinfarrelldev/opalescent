//! Checked project-to-native artifact emission, separate from production builds.
extern crate alloc;
mod cases;
mod module_values;
mod source;
use super::context::CodegenContext;
use super::error::CodegenError;
use crate::ast::{AstNode, Expr};
use crate::testing::native::checking::CheckedNativeProject;
use crate::testing::native::identity::CaseIdentity;
use crate::testing::native::lowering::{NativeCallbackIdentity, lower_checked_project};
use crate::testing::native::project::NativeTestProject;
use alloc::collections::BTreeMap;
use core::fmt::Write;
use inkwell::values::{FunctionValue, GlobalValue};

/// Independently selectable emitted descriptor in one reusable native artifact.
pub struct CompiledNativeCase<'context> {
    /// Stable structured case identity; not its source position or array index.
    pub identity: CaseIdentity,
    /// Deterministically encoded linker symbol for this descriptor.
    pub symbol: String,
    /// Target-aware immutable lifecycle descriptor.
    pub descriptor: GlobalValue<'context>,
}

/// One linked artifact contains all selected cases and shared source functions.
pub struct CompiledNativeProject<'context> {
    /// Every selected case has its own descriptor and runtime state plan.
    pub cases: Vec<CompiledNativeCase<'context>>,
    /// Managed source functions, compiled once independently of example count.
    pub source_callbacks: BTreeMap<NativeCallbackIdentity, FunctionValue<'context>>,
}

/// Emit the fully checked selected source graph and independent case descriptors.
///
/// No source callback or setup executes here. Native linking/worker invocation is
/// a separate step, and production compiler entry points are unchanged.
///
/// # Errors
/// Returns explicit static/codegen errors rather than an executable partial suite.
pub fn emit_checked_project<'context>(
    codegen: &CodegenContext<'context>,
    project: &NativeTestProject,
    checked: &CheckedNativeProject,
) -> Result<CompiledNativeProject<'context>, CodegenError> {
    let lowered =
        lower_checked_project(checked).map_err(|error| CodegenError::new(error.to_string()))?;
    let layouts = source::SourceLayouts::new(checked)?;
    let functions = source::functions(project, checked)?;
    for function in &functions {
        if super::native_generics::is_template(&function.signature) {
            continue;
        }
        super::native_declarations::declare_source_function(
            codegen,
            &symbol("source", &function.identity)?,
            &function.parameters,
            &function.signature,
        )?;
    }
    for function in &functions {
        if super::native_generics::is_template(&function.signature) {
            continue;
        }
        let mut env =
            layouts.environment(codegen, project, checked, &function.module, &functions)?;
        if function.is_value {
            module_values::emit(codegen, &mut env, function)?;
            continue;
        }
        super::native_declarations::emit_source_function(
            codegen,
            &mut env,
            &symbol("source", &function.identity)?,
            &function.parameters,
            &function.signature,
            &function.body,
        )?;
    }
    let mut source_callbacks = BTreeMap::new();
    for (identity, definition) in &lowered.callbacks {
        let Expr::Lambda {
            ref params,
            ref body,
            ..
        } = definition.callback.function
        else {
            return Err(CodegenError::new(
                "checked managed callback is not a function literal",
            ));
        };
        let loaded = project
            .modules
            .get(&identity.module_path)
            .ok_or_else(|| CodegenError::new("callback source module is not loaded"))?;
        let name = symbol(
            "callback",
            &(&loaded.identity, identity.start, identity.end),
        )?;
        let mut env =
            layouts.environment(codegen, project, checked, &identity.module_path, &functions)?;
        let body = source::lambda_body(body, definition.callback.function.span());
        let function = super::native_declarations::emit_source_function(
            codegen,
            &mut env,
            &name,
            params,
            &definition.signature,
            &body,
        )?;
        source_callbacks.insert(identity.clone(), function);
    }
    let cases = cases::emit_cases(
        codegen,
        project,
        checked,
        &lowered,
        &layouts,
        &functions,
        &source_callbacks,
    )?;
    emit_specializations(codegen, project, checked, &layouts, &functions)?;
    Ok(CompiledNativeProject {
        cases,
        source_callbacks,
    })
}

/// Drain declarations only after callers have finished using the shared LLVM builder.
fn emit_specializations(
    codegen: &CodegenContext<'_>,
    project: &NativeTestProject,
    checked: &CheckedNativeProject,
    layouts: &source::SourceLayouts,
    functions: &[source::SourceFunction],
) -> Result<(), CodegenError> {
    let templates = functions
        .iter()
        .filter(|value| super::native_generics::is_template(&value.signature))
        .map(|value| Ok((symbol("source", &value.identity)?, value)))
        .collect::<Result<BTreeMap<_, _>, CodegenError>>()?;
    loop {
        let request = codegen
            .native_specializations
            .borrow()
            .iter()
            .find(|&(name, _)| {
                codegen
                    .module
                    .get_function(name)
                    .is_some_and(|function| function.count_basic_blocks() == 0)
            })
            .map(|(name, value)| (name.clone(), value.clone()));
        let Some((name, value)) = request else {
            return Ok(());
        };
        let source = templates
            .get(&value.base)
            .ok_or_else(|| CodegenError::new("native specialization has no source template"))?;
        let mut env = layouts.environment(codegen, project, checked, &source.module, functions)?;
        env.native_type_arguments.extend(value.arguments);
        super::native_declarations::emit_source_function(
            codegen,
            &mut env,
            &name,
            &source.parameters,
            &value.signature,
            &source.body,
        )?;
    }
}

/// Hex-encoded structured identities cannot collide through alias punctuation.
pub(super) fn symbol(kind: &str, identity: &impl serde::Serialize) -> Result<String, CodegenError> {
    let encoded =
        serde_json::to_vec(identity).map_err(|error| CodegenError::new(error.to_string()))?;
    let mut name = format!("__opal_native_{kind}_");
    for byte in encoded {
        write!(name, "{byte:02x}").map_err(|error| CodegenError::new(error.to_string()))?;
    }
    Ok(name)
}
