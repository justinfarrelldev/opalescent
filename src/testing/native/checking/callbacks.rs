//! Ordinary typed callbacks with framework-specific nominal injection contracts.
extern crate alloc;
use super::{
    LoadedModule, LoadedModuleAst, NativeCheckError, ResolvedSuite, ResolvedWorldSet, TypeChecker,
    WorldIdentity, invalid, type_error,
};
use crate::ast::testing::{HookDeclaration, ManagedCallback, TestItem};
use crate::ast::{AstNode, BorrowKind, Expr, Parameter};
use crate::token::Span;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;

/// Check each shared source callback once, independently of its example count.
pub(super) fn check_module(
    module: &LoadedModule,
    checker: &mut TypeChecker,
    worlds: &ResolvedWorldSet,
    states: &BTreeMap<WorldIdentity, CoreType>,
    types: &BTreeMap<WorldIdentity, CoreType>,
    suite: &ResolvedSuite,
    signatures: &mut BTreeMap<(usize, usize), CoreType>,
) -> Result<(), NativeCheckError> {
    for world in worlds
        .declarations
        .values()
        .filter(|world| world.module_path == module.path)
    {
        let mut dependencies = Vec::new();
        for dependency in &world.declaration.dependencies {
            let identity = &worlds.bindings[&module.path][&dependency.path.join(".")];
            dependencies.push(types[identity].clone());
        }
        let signature = check_setup(checker, &world.declaration.setup, &dependencies)
            .map_err(|error| type_error(&module.path, error))?;
        record(signatures, &world.declaration.setup, signature);
        if let Some(ref cleanup) = world.declaration.cleanup {
            let cleanup_signature =
                check_cleanup(checker, cleanup, &states[&world.identity], &dependencies)
                    .map_err(|error| type_error(&module.path, error))?;
            record(signatures, cleanup, cleanup_signature);
        }
    }
    if let LoadedModuleAst::Test(ref ast) = module.ast {
        check_hooks(checker, &ast.items, types, &module.path, signatures)?;
    }
    for body in suite
        .bodies
        .values()
        .filter(|body| body.module_path == module.path)
    {
        let expected: Vec<_> = body
            .injected_worlds
            .iter()
            .map(|world| types[world].clone())
            .collect();
        let example_type = body
            .example_type
            .as_ref()
            .map(|source_type| checker.native_source_core_type(source_type))
            .transpose()
            .map_err(|error| type_error(&module.path, error))?;
        let signature = checker
            .type_check_native_case_callback(&body.callback, &expected, example_type.as_ref())
            .map_err(|error| type_error(&module.path, error))?;
        record(signatures, &body.callback, signature);
    }
    for case in &suite.cases {
        let body = &suite.bodies[&case.body_identity];
        if body.module_path != module.path {
            continue;
        }
        if let (Some(source_type), Some(example)) =
            (body.example_type.as_ref(), case.example.as_ref())
        {
            let record_type = checker
                .native_source_core_type(source_type)
                .map_err(|error| type_error(&module.path, error))?;
            checker
                .type_check_native_example(&record_type, example)
                .map_err(|error| type_error(&module.path, error))?;
        }
    }
    Ok(())
}

/// Validate ordered setup dependencies, with an optional final runner context.
fn check_setup(
    checker: &mut TypeChecker,
    callback: &ManagedCallback,
    dependencies: &[CoreType],
) -> Result<CoreType, crate::type_system::errors::TypeError> {
    let (parameters, _) = function_parts(callback)?;
    let context = context_count(checker, parameters)?;
    if parameters.len() != dependencies.len().saturating_add(context) {
        return Err(invalid(
            callback.function.span(),
            "setup must receive its declared dependency Worlds in exact order",
        ));
    }
    let mut required = Vec::new();
    for (parameter, expected) in parameters.iter().zip(dependencies) {
        if parameter.borrow_kind == BorrowKind::Owned
            || checker.native_source_core_type(&parameter.param_type)? != *expected
        {
            return Err(invalid(
                parameter.span,
                "setup dependency parameter requires the exact nominal World in declared order, passed by reference",
            ));
        }
        required.push(parameter.name.clone());
    }
    checker.type_check_native_callback(callback, &required)
}

/// Cleanup receives raw state first and an ordered subset of declared Worlds.
/// Neither the state nor cleanup callback creates a resource-release obligation.
fn check_cleanup(
    checker: &mut TypeChecker,
    callback: &ManagedCallback,
    state: &CoreType,
    dependencies: &[CoreType],
) -> Result<CoreType, crate::type_system::errors::TypeError> {
    let (parameters, returns) = function_parts(callback)?;
    if returns.len() != 1 || checker.native_source_core_type(&returns[0])? != CoreType::Unit {
        return Err(invalid(
            callback.function.span(),
            "World cleanup must return void",
        ));
    }
    let Some(state_parameter) = parameters.first() else {
        return Err(invalid(
            callback.function.span(),
            "World cleanup requires its initialized state as the first reference parameter",
        ));
    };
    if state_parameter.borrow_kind == BorrowKind::Owned
        || checker.native_source_core_type(&state_parameter.param_type)? != *state
    {
        return Err(invalid(
            state_parameter.span,
            "cleanup first parameter must reference the World state type",
        ));
    }
    let context = context_count(checker, parameters)?;
    let mut remaining = dependencies;
    for parameter in parameters
        .iter()
        .skip(1)
        .take(parameters.len().saturating_sub(1).saturating_sub(context))
    {
        let actual = checker.native_source_core_type(&parameter.param_type)?;
        let Some(index) = remaining
            .iter()
            .position(|dependency| *dependency == actual)
        else {
            return Err(invalid(
                parameter.span,
                "cleanup dependencies must be an ordered subset of declared dependency Worlds",
            ));
        };
        if parameter.borrow_kind == BorrowKind::Owned {
            return Err(invalid(
                parameter.span,
                "cleanup dependency Worlds must be passed by reference",
            ));
        }
        remaining = remaining.get(index.saturating_add(1)..).unwrap_or_default();
    }
    checker.type_check_native_callback(callback, &[])
}

/// Check hooks even when they serve no selected body; their parameter World
/// identities remain explicit and never become hidden body injections.
fn check_hooks(
    checker: &mut TypeChecker,
    items: &[TestItem],
    types: &BTreeMap<WorldIdentity, CoreType>,
    path: &std::path::Path,
    signatures: &mut BTreeMap<(usize, usize), CoreType>,
) -> Result<(), NativeCheckError> {
    for item in items {
        match *item {
            TestItem::Hook(ref hook) => {
                let signature =
                    check_hook(checker, hook, types).map_err(|error| type_error(path, error))?;
                record(signatures, &hook.callback, signature);
            }
            TestItem::Scenario(ref scenario) => {
                check_hooks(checker, &scenario.items, types, path, signatures)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Resolve only nominal World parameters plus an optional final test context.
fn check_hook(
    checker: &mut TypeChecker,
    hook: &HookDeclaration,
    types: &BTreeMap<WorldIdentity, CoreType>,
) -> Result<CoreType, crate::type_system::errors::TypeError> {
    let (parameters, _) = function_parts(&hook.callback)?;
    let context = context_count(checker, parameters)?;
    let mut expected = Vec::new();
    for parameter in parameters
        .iter()
        .take(parameters.len().saturating_sub(context))
    {
        let actual = checker.native_source_core_type(&parameter.param_type)?;
        if !types.values().any(|world| *world == actual) {
            return Err(invalid(
                parameter.span,
                "hook parameter must be a nominal World reference",
            ));
        }
        expected.push(actual);
    }
    checker.type_check_native_case_callback(&hook.callback, &expected, None)
}

/// Source signature parts, rejecting uninstantiated generic callbacks.
fn function_parts(
    callback: &ManagedCallback,
) -> Result<(&[Parameter], &[crate::ast::Type]), crate::type_system::errors::TypeError> {
    let Expr::Lambda {
        ref params,
        ref return_types,
        ref generic_params,
        ..
    } = callback.function
    else {
        return Err(invalid(
            callback.function.span(),
            "framework callback must be an ordinary typed function literal",
        ));
    };
    if generic_params
        .as_ref()
        .is_some_and(|parameters| !parameters.is_empty())
    {
        return Err(invalid(
            callback.function.span(),
            "framework callbacks require concrete signatures, not uninstantiated generic functions",
        ));
    }
    Ok((params, return_types))
}

/// A final context adds no provider requirement and must be a mutable reference.
fn context_count(
    checker: &TypeChecker,
    parameters: &[Parameter],
) -> Result<usize, crate::type_system::errors::TypeError> {
    let Some(parameter) = parameters.last() else {
        return Ok(0);
    };
    if matches!(checker.native_source_core_type(&parameter.param_type)?, CoreType::Generic { ref name, .. } if name == "__opal_testing.TestContext")
    {
        if parameter.borrow_kind != BorrowKind::MutableRef {
            return Err(invalid(
                parameter.span,
                "test context must be passed as a final mutable ref",
            ));
        }
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Retain the inferred ordinary application signature for native wrapper lowering.
fn record(
    signatures: &mut BTreeMap<(usize, usize), CoreType>,
    callback: &ManagedCallback,
    signature: CoreType,
) {
    let Span { start, end } = callback.function.span();
    signatures.insert((start.offset, end.offset), signature);
}
