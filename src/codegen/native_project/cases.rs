//! Typed invocation wrappers, fresh example constructors and case globals.
use super::{
    CheckedNativeProject, CodegenContext, CodegenError, CompiledNativeCase, NativeTestProject,
    source::{SourceFunction, SourceLayouts},
    symbol,
};
use crate::ast::{AstNode, Expr, NodeId};
use crate::codegen::native_callbacks::NativeCallbackArgument;
use crate::codegen::native_descriptors::{
    NativeCallbackDescriptor, NativeCaseDescriptorPlan, NativeHookScopeDescriptor,
    NativeWorldDescriptor,
};
use crate::testing::native::lowering::{
    LoweredNativeCase, LoweredNativeProject, NativeCallbackIdentity, NativeInjection,
    NativeInvocationPlan,
};
use crate::type_system::types::CoreType;
use inkwell::values::FunctionValue;
extern crate alloc;
use alloc::collections::BTreeMap;
use inkwell::types::BasicType;

/// Immutable compiler inputs, never exposed as a runtime fixture dictionary.
struct CaseEmitter<'source, 'context> {
    /// Shared test-only LLVM artifact.
    codegen: &'source CodegenContext<'context>,
    /// Original source graph, retained for spans and defining environments.
    project: &'source NativeTestProject,
    /// Authoritative compiler metadata.
    checked: &'source CheckedNativeProject,
    /// Checked invocation and reusable source body plans.
    lowered: &'source LoweredNativeProject,
    /// Source-order canonical layouts.
    layouts: &'source SourceLayouts,
    /// Ordinary concrete application/helper functions.
    functions: &'source [SourceFunction],
    /// Native source definitions shared across all cases.
    callbacks: &'source BTreeMap<NativeCallbackIdentity, FunctionValue<'context>>,
}

/// Emit all selected descriptors without compiling a separate artifact per case.
pub(super) fn emit_cases<'context>(
    codegen: &CodegenContext<'context>,
    project: &NativeTestProject,
    checked: &CheckedNativeProject,
    lowered: &LoweredNativeProject,
    layouts: &SourceLayouts,
    functions: &[SourceFunction],
    callbacks: &BTreeMap<NativeCallbackIdentity, FunctionValue<'context>>,
) -> Result<Vec<CompiledNativeCase<'context>>, CodegenError> {
    let emitter = CaseEmitter {
        codegen,
        project,
        checked,
        lowered,
        layouts,
        functions,
        callbacks,
    };
    lowered
        .cases
        .iter()
        .map(|case| emitter.case(case))
        .collect()
}

impl<'context> CaseEmitter<'_, 'context> {
    /// One descriptor has fresh value construction and no shared mutable state.
    fn case(&self, case: &LoweredNativeCase) -> Result<CompiledNativeCase<'context>, CodegenError> {
        let name = symbol("case", &case.identity)?;
        let mut worlds = Vec::new();
        for (index, world) in case.worlds.iter().enumerate() {
            let setup = self.invocation(
                &format!("{name}_world_{index}_setup"),
                &world.setup,
                Some(&world.state_type),
            )?;
            let cleanup = world
                .cleanup
                .as_ref()
                .map(|plan| self.invocation(&format!("{name}_world_{index}_cleanup"), plan, None))
                .transpose()?;
            let drop_value = crate::codegen::native_value_callbacks::emit_value_drop_wrapper(
                self.codegen,
                &format!("{name}_world_{index}_drop"),
                &world.state_type,
            )?;
            worlds.push(NativeWorldDescriptor {
                identity: world.identity.clone(),
                setup,
                cleanup,
                drop_value: Some(drop_value),
            });
        }
        let mut scopes = Vec::new();
        for (index, scope) in case.scopes.iter().enumerate() {
            let mut before = Vec::new();
            let mut after = Vec::new();
            for (hook, plan) in scope.before.iter().enumerate() {
                before.push(self.invocation(
                    &format!("{name}_scope_{index}_before_{hook}"),
                    plan,
                    None,
                )?);
            }
            for (hook, plan) in scope.after.iter().enumerate() {
                after.push(self.invocation(
                    &format!("{name}_scope_{index}_after_{hook}"),
                    plan,
                    None,
                )?);
            }
            scopes.push(NativeHookScopeDescriptor { before, after });
        }
        let body = self.invocation(&format!("{name}_body"), &case.body, None)?;
        let (construct_example, drop_example) = self.example(&name, case)?;
        let plan = NativeCaseDescriptorPlan {
            worlds,
            scopes,
            body,
            construct_example,
            drop_example,
        };
        let descriptor =
            crate::codegen::native_descriptors::emit_case_descriptor(self.codegen, &name, &plan)?;
        Ok(CompiledNativeCase {
            identity: case.identity.clone(),
            symbol: name,
            descriptor,
        })
    }

    /// Source signature metadata selects ordinary errors, never aggregate guessing.
    fn invocation(
        &self,
        name: &str,
        plan: &NativeInvocationPlan,
        value: Option<&CoreType>,
    ) -> Result<NativeCallbackDescriptor<'context>, CodegenError> {
        let source = *self
            .callbacks
            .get(&plan.callback)
            .ok_or_else(|| CodegenError::new("case invocation has no compiled source function"))?;
        let definition =
            self.lowered.callbacks.get(&plan.callback).ok_or_else(|| {
                CodegenError::new("case invocation has no checked source definition")
            })?;
        let CoreType::Function {
            ref error_types, ..
        } = definition.signature
        else {
            return Err(CodegenError::new(
                "case invocation signature is not callable",
            ));
        };
        let arguments: Vec<_> = plan
            .arguments
            .iter()
            .map(|argument| match *argument {
                NativeInjection::WorldReference { index, .. } => {
                    NativeCallbackArgument::WorldReference(index)
                }
                NativeInjection::StateReference { index, .. } => {
                    NativeCallbackArgument::StateReference(index)
                }
                NativeInjection::ExampleReference => NativeCallbackArgument::ExampleReference,
                NativeInjection::ContextReference => NativeCallbackArgument::ContextReference,
            })
            .collect();
        let function = if let Some(core) = value {
            crate::codegen::native_value_callbacks::emit_value_callback_wrapper(
                self.codegen,
                name,
                source,
                core,
                !error_types.is_empty(),
                &arguments,
            )?
        } else {
            crate::codegen::native_callbacks::emit_void_callback_wrapper_with_arguments(
                self.codegen,
                name,
                source,
                !error_types.is_empty(),
                &arguments,
            )?
        };
        let module = self
            .project
            .modules
            .get(&plan.callback.module_path)
            .ok_or_else(|| CodegenError::new("callback source module is missing"))?;
        Ok(NativeCallbackDescriptor {
            function,
            file: module.identity.clone(),
            span: definition.callback.function.span(),
        })
    }

    /// Synthesize only the typed constant-data constructor, not rewritten source.
    fn example(
        &self,
        name: &str,
        case: &LoweredNativeCase,
    ) -> Result<
        (
            Option<NativeCallbackDescriptor<'context>>,
            Option<FunctionValue<'context>>,
        ),
        CodegenError,
    > {
        let (Some(example), Some(core)) = (case.example.as_ref(), case.example_type.as_ref())
        else {
            return Ok((None, None));
        };
        let CoreType::Generic {
            name: ref owner, ..
        } = *core
        else {
            return Err(CodegenError::new(
                "checked example schema is not a nominal record",
            ));
        };
        let mut env = self.layouts.environment(
            self.codegen,
            self.project,
            self.checked,
            &case.body.callback.module_path,
            self.functions,
        )?;
        let llvm_type = crate::codegen::types::core_type_to_llvm(self.codegen.context, core);
        let source = self.codegen.declare_native_test_function(
            &format!("{name}_example_source"),
            llvm_type.fn_type(&[], false),
            None,
        )?;
        self.codegen
            .builder
            .position_at_end(self.codegen.context.append_basic_block(source, "entry"));
        let constructor = Expr::Constructor {
            callee: Box::new(Expr::Identifier {
                name: owner.clone(),
                span: example.span,
                id: NodeId(0),
            }),
            fields: example.fields.clone(),
            span: example.span,
            id: NodeId(0),
        };
        let value = crate::codegen::expressions::codegen_expression(
            self.codegen,
            &mut env,
            &constructor,
            Some(core),
        )?;
        self.codegen.emit_application_return(Some(value))?;
        let function = crate::codegen::native_value_callbacks::emit_value_callback_wrapper(
            self.codegen,
            &format!("{name}_example"),
            source,
            core,
            false,
            &[],
        )?;
        let drop = crate::codegen::native_value_callbacks::emit_value_drop_wrapper(
            self.codegen,
            &format!("{name}_example_drop"),
            core,
        )?;
        let file = self
            .project
            .modules
            .get(&case.body.callback.module_path)
            .ok_or_else(|| CodegenError::new("example source module is missing"))?
            .identity
            .clone();
        Ok((
            Some(NativeCallbackDescriptor {
                function,
                file,
                span: example.span,
            }),
            Some(drop),
        ))
    }
}
