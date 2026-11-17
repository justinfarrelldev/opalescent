//! Resolve canonical checked signatures into deterministic case-local addresses.
use super::{
    CheckedNativeProject, CoreType, LoweredNativeCase, LoweredNativeProject,
    NativeCallbackDefinition, NativeCallbackIdentity, NativeCallbackRole, NativeInjection,
    NativeInvocationPlan, NativeLoweringError, NativeScopePlan, NativeWorldPlan, WorldIdentity,
};
use crate::ast::{AstNode, testing::ManagedCallback};
use crate::testing::native::cases::ResolvedTestBody;
extern crate alloc;
use alloc::collections::BTreeMap;
use std::path::Path;

/// The only mutable state is the compiler's source-definition deduplication map.
struct Planner<'checked> {
    /// Fully validated source project; no execution is performed during lowering.
    checked: &'checked CheckedNativeProject,
    /// Reusable source definitions and independent output case plans.
    result: LoweredNativeProject,
}

/// Exact case-local positions and permitted non-World reference arguments.
#[derive(Clone, Copy)]
struct InjectionContext<'case> {
    /// Canonical provider to unique initialization slot.
    slots: &'case BTreeMap<WorldIdentity, usize>,
    /// State-first cleanup owner, distinct from dependencies with the same type.
    cleanup: Option<(&'case WorldIdentity, usize)>,
    /// Example position follows effective body injections only.
    example_index: Option<usize>,
}

/// Build shared source definitions and explicit independent invocation graphs.
pub(super) fn lower(
    checked: &CheckedNativeProject,
) -> Result<LoweredNativeProject, NativeLoweringError> {
    let mut planner = Planner {
        checked,
        result: LoweredNativeProject::default(),
    };
    for case in &checked.suite.cases {
        let body = checked
            .suite
            .bodies
            .get(&case.body_identity)
            .ok_or_else(|| invalid("case has no checked shared source body"))?;
        let slots: BTreeMap<_, _> = body
            .initialization_order
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, identity)| (identity, index))
            .collect();
        if slots.len() != body.initialization_order.len() {
            return Err(invalid(
                "duplicate canonical World in case initialization order",
            ));
        }
        let context = InjectionContext {
            slots: &slots,
            cleanup: None,
            example_index: None,
        };
        let worlds = planner.worlds(body, context)?;
        let scopes = planner.scopes(body, context)?;
        let example_index = body
            .example_type
            .as_ref()
            .map(|_| body.injected_worlds.len());
        let invocation = planner.invocation(
            &body.module_path,
            &body.callback,
            NativeCallbackRole::Body,
            InjectionContext {
                example_index,
                ..context
            },
        )?;
        let signature = &planner.result.callbacks[&invocation.callback].signature;
        let example_type = example_index
            .map(|index| {
                function_parameters(signature)?
                    .get(index)
                    .cloned()
                    .ok_or_else(|| invalid("example schema has no checked source parameter"))
            })
            .transpose()?;
        if case.example.is_some() != example_type.is_some() {
            return Err(invalid("case example data and checked schema disagree"));
        }
        planner.result.cases.push(LoweredNativeCase {
            identity: case.identity.clone(),
            body_identity: case.body_identity.clone(),
            worlds,
            scopes,
            body: invocation,
            example: case.example.clone(),
            example_type,
        });
    }
    Ok(planner.result)
}

impl Planner<'_> {
    /// Dependencies use one actual initialized position, including shared diamonds.
    fn worlds(
        &mut self,
        body: &ResolvedTestBody,
        context: InjectionContext<'_>,
    ) -> Result<Vec<NativeWorldPlan>, NativeLoweringError> {
        let checked = self.checked;
        let mut result = Vec::new();
        for identity in &body.initialization_order {
            let world =
                checked.worlds.declarations.get(identity).ok_or_else(|| {
                    invalid("initialization references an unknown canonical World")
                })?;
            let state_type = checked
                .world_states
                .get(identity)
                .ok_or_else(|| invalid("World has no checked concrete state type"))?
                .clone();
            let setup = self.invocation(
                &world.module_path,
                &world.declaration.setup,
                NativeCallbackRole::WorldSetup,
                context,
            )?;
            let index = *context
                .slots
                .get(identity)
                .ok_or_else(|| invalid("World has no case-local initialized slot"))?;
            let cleanup = world
                .declaration
                .cleanup
                .as_ref()
                .map(|callback| {
                    self.invocation(
                        &world.module_path,
                        callback,
                        NativeCallbackRole::WorldCleanup,
                        InjectionContext {
                            cleanup: Some((identity, index)),
                            ..context
                        },
                    )
                })
                .transpose()?;
            result.push(NativeWorldPlan {
                identity: identity.clone(),
                state_type,
                setup,
                cleanup,
            });
        }
        Ok(result)
    }

    /// Preserve source order and scope entry independently of callback outcomes.
    fn scopes(
        &mut self,
        body: &ResolvedTestBody,
        context: InjectionContext<'_>,
    ) -> Result<Vec<NativeScopePlan>, NativeLoweringError> {
        let mut result = Vec::new();
        for scope in &body.hook_scopes {
            let mut plan = NativeScopePlan::default();
            for hook in &scope.hooks {
                let role = if hook.before {
                    NativeCallbackRole::BeforeHook
                } else {
                    NativeCallbackRole::AfterHook
                };
                let invocation =
                    self.invocation(&body.module_path, &hook.callback, role, context)?;
                if hook.before {
                    plan.before.push(invocation);
                } else {
                    plan.after.push(invocation);
                }
            }
            result.push(plan);
        }
        Ok(result)
    }

    /// Register source once; independently resolve every case's argument positions.
    fn invocation(
        &mut self,
        module: &Path,
        callback: &ManagedCallback,
        role: NativeCallbackRole,
        context: InjectionContext<'_>,
    ) -> Result<NativeInvocationPlan, NativeLoweringError> {
        let span = callback.function.span();
        let signature = self
            .checked
            .modules
            .get(module)
            .and_then(|module| module.callbacks.get(&(span.start.offset, span.end.offset)))
            .ok_or_else(|| invalid("managed callback lacks an authoritative checked signature"))?;
        let parameters = function_parameters(signature)?;
        let mut arguments = Vec::new();
        for (index, parameter) in parameters.iter().enumerate() {
            arguments.push(self.argument(parameter, index, context)?);
        }
        let identity = NativeCallbackIdentity {
            module_path: module.to_path_buf(),
            start: span.start.offset,
            end: span.end.offset,
        };
        if let Some(definition) = self.result.callbacks.get(&identity) {
            if definition.signature != *signature
                || definition.role != role
                || definition.callback != *callback
            {
                return Err(invalid(
                    "source callback identity refers to conflicting checked definitions",
                ));
            }
        } else {
            self.result.callbacks.insert(
                identity.clone(),
                NativeCallbackDefinition {
                    callback: callback.clone(),
                    signature: signature.clone(),
                    role,
                },
            );
        }
        Ok(NativeInvocationPlan {
            callback: identity,
            arguments,
        })
    }

    /// State-first cleanup is explicit; equal concrete state types cannot alias Worlds.
    fn argument(
        &self,
        parameter: &CoreType,
        index: usize,
        context: InjectionContext<'_>,
    ) -> Result<NativeInjection, NativeLoweringError> {
        if index == 0 {
            if let Some((identity, slot)) = context.cleanup {
                if self.checked.world_states.get(identity) != Some(parameter) {
                    return Err(invalid(
                        "cleanup state parameter differs from its provider's checked state",
                    ));
                }
                return Ok(NativeInjection::StateReference {
                    identity: identity.clone(),
                    index: slot,
                });
            }
        }
        if let Some((identity, _)) = self
            .checked
            .world_types
            .iter()
            .find(|&(_, core)| core == parameter)
        {
            let slot = context.slots.get(identity).ok_or_else(|| {
                invalid("callback requests a World absent from its case lifecycle")
            })?;
            return Ok(NativeInjection::WorldReference {
                identity: identity.clone(),
                index: *slot,
            });
        }
        if matches!(*parameter, CoreType::Generic { ref name, .. } if name == "__opal_testing::TestContext")
        {
            return Ok(NativeInjection::ContextReference);
        }
        if context.example_index == Some(index) {
            return Ok(NativeInjection::ExampleReference);
        }
        Err(invalid(
            "checked managed parameter has no explicit typed injection position",
        ))
    }
}

/// No source expression is re-inferred or evaluated during planning.
fn function_parameters(signature: &CoreType) -> Result<&[CoreType], NativeLoweringError> {
    if let CoreType::Function { ref parameters, .. } = *signature {
        Ok(parameters)
    } else {
        Err(invalid("checked callback signature is not a function"))
    }
}

/// Static compilation failure is distinct from all attempt outcomes.
fn invalid(message: &str) -> NativeLoweringError {
    NativeLoweringError(message.to_owned())
}
