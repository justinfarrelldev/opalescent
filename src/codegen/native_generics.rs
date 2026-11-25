//! Demand-driven specialization preserves the native source ABI and defining module.
extern crate alloc;
use self::alloc::{boxed::Box, collections::BTreeMap};
use super::{context::CodegenContext, error::CodegenError, expressions::CodegenEnv};
use crate::{
    ast::{Expr, Type},
    type_system::{substitution::Substitution, types::CoreType},
};

/// Concrete signature and original definition for a deferred body emission.
#[derive(Clone)]
pub(super) struct Specialization {
    /// Canonical generic source definition symbol.
    pub base: String,
    /// Original type parameter names and their concrete values, in declaration order.
    pub arguments: Vec<(String, CoreType)>,
    /// Fully instantiated native function signature.
    pub signature: CoreType,
}

/// A template has no executable ABI until a call supplies concrete arguments.
pub(super) fn is_template(signature: &CoreType) -> bool {
    matches!(*signature,CoreType::Function {ref generic_params,..} if !generic_params.is_empty())
}

/// Resolve an explicit type through the caller's concrete type bindings and nominal aliases.
pub(super) fn annotation(env: &CodegenEnv<'_>, source: &Type) -> Result<CoreType, CodegenError> {
    fn resolve(env: &CodegenEnv<'_>, mapped: CoreType) -> CoreType {
        match mapped {
            CoreType::Generic { name, type_args } => {
                if type_args.is_empty() {
                    if let Some(concrete) = env.native_type_arguments.get(&name) {
                        return concrete.clone();
                    }
                }
                CoreType::Generic {
                    name: env.adt_layout_aliases.get(&name).cloned().unwrap_or(name),
                    type_args: type_args.into_iter().map(|ty| resolve(env, ty)).collect(),
                }
            }
            CoreType::Array(element) => CoreType::Array(Box::new(resolve(env, *element))),
            CoreType::Function {
                generic_params,
                parameters,
                return_types,
                error_types,
            } => CoreType::Function {
                generic_params,
                parameters: parameters.into_iter().map(|ty| resolve(env, ty)).collect(),
                return_types: return_types
                    .into_iter()
                    .map(|ty| resolve(env, ty))
                    .collect(),
                error_types: error_types.into_iter().map(|ty| resolve(env, ty)).collect(),
            },
            other => other,
        }
    }
    let core = crate::type_system::type_mapping::ast_type_to_core_type(source).map_err(
        |crate::type_system::type_mapping::AstTypeMappingError::TypeNotFound {
             type_name, ..
         }| CodegenError::new(format!("could not lower native generic type '{type_name}'")),
    )?;
    Ok(resolve(env, core))
}

/// Find a native template by its checked import mapping and declare a concrete call alias.
pub(super) fn call_alias<'ctx>(
    ctx: &CodegenContext<'ctx>,
    env: &mut CodegenEnv<'ctx>,
    local: &str,
    explicit: Option<&[Type]>,
    args: &[Expr],
    expected: Option<&CoreType>,
) -> Result<Option<String>, CodegenError> {
    if env.variables.contains_key(local) {
        return Ok(None);
    }
    let Some(base) = env.imported_functions.get(local).cloned() else {
        return Ok(None);
    };
    let Some(parameters) = env.native_generic_parameters.get(&base) else {
        return Ok(None);
    };
    let Some(&CoreType::Function {
        ref generic_params,
        parameters: ref types,
        ref return_types,
        ref error_types,
    }) = env.imported_signatures.get(local)
    else {
        return Err(CodegenError::new(
            "native generic call is missing its checked signature",
        ));
    };
    let mut inferred = BTreeMap::new();
    if explicit.is_none() {
        for (pattern, expr) in types.iter().zip(args) {
            let source = if let Expr::BorrowArgument { ref target, .. } = *expr {
                target.as_ref()
            } else {
                expr
            };
            if let Some(actual) = super::native_assertions::operand_core_type(env, source) {
                infer(pattern, &actual, &mut inferred);
            }
        }
        if let (Some(result), Some(ty)) = (return_types.first(), expected) {
            infer(result, ty, &mut inferred);
        }
    }
    let concrete = if let Some(explicit) = explicit {
        explicit
            .iter()
            .map(|ty| annotation(env, ty))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        generic_params.iter().map(|parameter|inferred.get(&parameter.type_var.id).cloned().ok_or_else(||CodegenError::new(format!("could not infer native specialization for '{}'; supply a concrete type argument",parameter.name)))).collect::<Result<Vec<_>,_>>()?
    };
    if concrete.len() != generic_params.len() {
        return Err(CodegenError::new(
            "native specialization type argument count mismatch",
        ));
    }
    let mut subst = Substitution::empty();
    for (parameter, ty) in generic_params.iter().zip(&concrete) {
        subst = subst.compose(&Substitution::single(parameter.type_var.id, ty.clone()));
    }
    let arguments: Vec<_> = generic_params
        .iter()
        .zip(concrete)
        .map(|(param, ty)| (param.name.clone(), ty))
        .collect();
    let signature = CoreType::Function {
        generic_params: Vec::new(),
        parameters: types.iter().map(|ty| subst.apply(ty)).collect(),
        return_types: return_types.iter().map(|ty| subst.apply(ty)).collect(),
        error_types: error_types.iter().map(|ty| subst.apply(ty)).collect(),
    };
    let mut existing = ctx.native_specializations.borrow_mut();
    let symbol = if let Some((symbol, _)) = existing
        .iter()
        .find(|&(_, value)| value.base == base && value.arguments == arguments)
    {
        symbol.clone()
    } else {
        let symbol = format!("{base}.specialization.{}", existing.len());
        super::native_declarations::declare_source_function(ctx, &symbol, parameters, &signature)?;
        existing.insert(
            symbol.clone(),
            Specialization {
                base,
                arguments,
                signature: signature.clone(),
            },
        );
        symbol
    };
    let alias = env.next_name("__native_generic_call");
    env.imported_signatures.insert(alias.clone(), signature);
    env.imported_functions.insert(alias.clone(), symbol);
    Ok(Some(alias))
}

/// Recover a quantified argument from ordinary typed call operands, including nested types.
fn infer(pattern: &CoreType, actual: &CoreType, types: &mut BTreeMap<usize, CoreType>) {
    match (pattern, actual) {
        (&CoreType::Variable(ref var), actual) => {
            types.entry(var.id).or_insert_with(|| actual.clone());
        }
        (&CoreType::Array(ref left), &CoreType::Array(ref right)) => infer(left, right, types),
        (
            &CoreType::Generic {
                name: ref left,
                type_args: ref left_args,
            },
            &CoreType::Generic {
                name: ref right,
                type_args: ref right_args,
            },
        ) if left == right => {
            for (formal, concrete) in left_args.iter().zip(right_args) {
                infer(formal, concrete, types);
            }
        }
        _ => {}
    }
}
