//! Checked source definitions and original module-specific lowering environments.
use super::{CheckedNativeProject, CodegenContext, CodegenError, NativeTestProject, symbol};
use crate::ast::testing::SourceReference;
use crate::ast::{
    AstNode, Decl, Expr, Field, ImportItem, LabeledValue, LambdaBody, NodeId, Parameter, Stmt,
    Type, TypeDef,
};
use crate::codegen::expressions::CodegenEnv;
use crate::testing::native::identity::DeclarationIdentity;
use crate::type_system::types::CoreType;
extern crate alloc;
use alloc::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// One concrete ordinary source function, before test ABI instrumentation.
pub(super) struct SourceFunction {
    /// Canonical filesystem module for the checked source environment.
    pub module: PathBuf,
    /// Defining-module canonical callable identity.
    pub identity: DeclarationIdentity,
    /// Source parameters retain ordinary borrowing modes.
    pub parameters: Vec<Parameter>,
    /// Ordinary source body, without source rewriting.
    pub body: Stmt,
    /// Authoritative checked application signature.
    pub signature: CoreType,
}

/// Global canonical field layouts with declaration-order, not map-order, fields.
#[derive(Default)]
pub(super) struct SourceLayouts {
    /// Nominal layout in original field order.
    fields: BTreeMap<String, Vec<(String, CoreType)>>,
    /// Explicit products are never inferred to be variants from path punctuation.
    products: BTreeSet<String>,
    /// Original named field positions.
    indices: BTreeMap<String, BTreeMap<String, u32>>,
    /// Genuine sum variants only.
    variants: BTreeMap<String, i64>,
}

/// Read checked signatures and ordinary source functions without executing them.
pub(super) fn functions(
    project: &NativeTestProject,
    checked: &CheckedNativeProject,
) -> Result<Vec<SourceFunction>, CodegenError> {
    let mut result = Vec::new();
    for (path, module) in &checked.modules {
        let loaded = project
            .modules
            .get(path)
            .ok_or_else(|| CodegenError::new("checked source module is not loaded"))?;
        for declaration in &module.program.declarations {
            let parts = match *declaration {
                Decl::Let {
                    ref binding,
                    initializer:
                        Expr::Lambda {
                            ref params,
                            ref body,
                            ..
                        },
                    ..
                } => Some((
                    binding.name.clone(),
                    params.clone(),
                    lambda_body(body, declaration.span()),
                )),
                Decl::Function {
                    ref name,
                    ref parameters,
                    ref body,
                    ..
                } => Some((name.clone(), parameters.clone(), body.clone())),
                Decl::Let { .. } => {
                    return Err(CodegenError::new(
                        "native module-level values require source initialization lowering",
                    ));
                }
                _ => None,
            };
            if let Some((name, parameters, body)) = parts {
                let signature = module
                    .checker
                    .symbol_table()
                    .lookup(&name)
                    .ok_or_else(|| {
                        CodegenError::new("source function has no checked symbol signature")
                    })?
                    .core_type
                    .clone();
                result.push(SourceFunction {
                    module: path.clone(),
                    identity: loaded.declaration_identity(&name),
                    parameters,
                    body,
                    signature,
                });
            }
        }
    }
    Ok(result)
}

impl SourceLayouts {
    /// Build ordered layouts using the checker's canonical resolved field types.
    pub fn new(checked: &CheckedNativeProject) -> Result<Self, CodegenError> {
        let mut result = Self::default();
        for module in checked.modules.values() {
            for declaration in &module.program.declarations {
                let Decl::Type {
                    ref name,
                    ref type_def,
                    span,
                    ..
                } = *declaration
                else {
                    continue;
                };
                let owner = module
                    .checker
                    .native_source_core_type(&Type::Basic {
                        name: name.clone(),
                        span,
                    })
                    .map_err(|error| CodegenError::new(error.to_string()))?;
                let CoreType::Generic { name: owner, .. } = owner else {
                    continue;
                };
                match *type_def {
                    TypeDef::Product { ref fields, .. } => {
                        result.add(&module.checker, &owner, fields)?;
                        result.products.insert(owner.clone());
                    }
                    TypeDef::Sum { ref variants, .. } => {
                        for (index, variant) in variants.iter().enumerate() {
                            let variant_name = format!("{owner}.{}", variant.name);
                            result.add(&module.checker, &variant_name, &variant.fields)?;
                            let index = i64::try_from(index)
                                .map_err(|error| CodegenError::new(error.to_string()))?;
                            result
                                .variants
                                .insert(variant_name, variant.explicit_id.unwrap_or(index));
                        }
                    }
                    TypeDef::Alias { .. } | TypeDef::Opaque { .. } => {}
                }
            }
        }
        for (identity, core) in &checked.world_types {
            let CoreType::Generic { ref name, .. } = *core else {
                return Err(CodegenError::new("World handle has no nominal type"));
            };
            let state = checked
                .world_states
                .get(identity)
                .ok_or_else(|| CodegenError::new("World has no concrete state"))?;
            result.products.insert(name.clone());
            result
                .fields
                .insert(name.clone(), vec![("state".to_owned(), state.clone())]);
            result
                .indices
                .insert(name.clone(), BTreeMap::from([("state".to_owned(), 0)]));
        }
        Ok(result)
    }

    /// Use source field order, while preserving checker-resolved nominal identities.
    fn add(
        &mut self,
        checker: &crate::type_system::checker::TypeChecker,
        owner: &str,
        fields: &[Field],
    ) -> Result<(), CodegenError> {
        let mut layout = Vec::new();
        let mut indices = BTreeMap::new();
        for (index, field) in fields.iter().enumerate() {
            let core = checker.adt_field_type(owner, &field.name).ok_or_else(|| {
                CodegenError::new(format!(
                    "checked field metadata is missing for {owner}.{}",
                    field.name
                ))
            })?;
            layout.push((field.name.clone(), core.clone()));
            indices.insert(
                field.name.clone(),
                u32::try_from(index).map_err(|error| CodegenError::new(error.to_string()))?,
            );
        }
        self.fields.insert(owner.to_owned(), layout);
        self.indices.insert(owner.to_owned(), indices);
        Ok(())
    }

    /// No implicit imports: only actual checker symbols install source aliases.
    pub fn environment<'context>(
        &self,
        codegen: &CodegenContext<'context>,
        project: &NativeTestProject,
        checked: &CheckedNativeProject,
        module: &Path,
        functions: &[SourceFunction],
    ) -> Result<CodegenEnv<'context>, CodegenError> {
        let loaded = project
            .modules
            .get(module)
            .ok_or_else(|| CodegenError::new("native source environment module is not loaded"))?;
        let checked_module = checked
            .modules
            .get(module)
            .ok_or_else(|| CodegenError::new("native source environment module is not checked"))?;
        let mut env = CodegenEnv::new(true);
        env.current_source_path.clone_from(&loaded.identity);
        env.current_source_text.clone_from(&loaded.source);
        env.adt_field_layouts = self.fields.clone();
        env.adt_product_layouts.clone_from(&self.products);
        env.adt_field_indices = self.indices.clone();
        env.adt_variant_discriminants = self.variants.clone();
        for entry in checked_module
            .checker
            .symbol_table()
            .current_scope_symbols()
        {
            match entry.core_type {
                CoreType::Function { .. } => {
                    env.imported_signatures
                        .insert(entry.name.clone(), entry.core_type.clone());
                }
                CoreType::Generic { ref name, .. } => {
                    env.adt_layout_aliases
                        .insert(entry.name.clone(), name.clone());
                    let prefix = format!("{name}.");
                    for variant in self.variants.keys().filter(|key| key.starts_with(&prefix)) {
                        env.adt_layout_aliases.insert(
                            format!(
                                "{}.{}",
                                entry.name,
                                variant.strip_prefix(&prefix).unwrap_or(variant)
                            ),
                            variant.clone(),
                        );
                    }
                }
                _ => {}
            }
        }
        for function in functions
            .iter()
            .filter(|function| function.module == module)
        {
            env.imported_functions.insert(
                function.identity.name.clone(),
                symbol("source", &function.identity)?,
            );
        }
        for declaration in &checked_module.program.declarations {
            if let Decl::Import {
                ref source,
                ref items,
                ..
            } = *declaration
            {
                if source.starts_with('.') {
                    for item in items {
                        if let ImportItem::Named {
                            ref name,
                            ref alias,
                            span,
                        } = *item
                        {
                            let local = alias.as_ref().unwrap_or(name);
                            if !env.imported_signatures.contains_key(local) {
                                continue;
                            }
                            let reference = SourceReference {
                                path: vec![local.clone()],
                                span,
                            };
                            let identity = crate::testing::native::subjects::resolve_subject(
                                project, module, &reference,
                            )
                            .map_err(|error| CodegenError::new(error.to_string()))?;
                            env.imported_functions
                                .insert(local.clone(), symbol("source", &identity.0)?);
                        }
                    }
                } else {
                    crate::codegen::functions::codegen_import_declaration(
                        codegen,
                        &mut env,
                        declaration,
                    )?;
                }
            }
        }
        crate::codegen::native_context::install_layout(&mut env);
        Ok(env)
    }
}

/// Preserve original expression nodes and spans; synthesize only a return container.
pub(super) fn lambda_body(body: &LambdaBody, span: crate::token::Span) -> Stmt {
    match *body {
        LambdaBody::Block(ref statements) => Stmt::Block {
            statements: statements.clone(),
            span,
            id: NodeId(0),
        },
        LambdaBody::Expression(ref expression) => Stmt::Return {
            values: vec![LabeledValue {
                label: String::new(),
                value: *expression.clone(),
                span: expression.span(),
                id: NodeId(0),
            }],
            span,
            id: NodeId(0),
        },
    }
}
