//! Declaration type checking for the Opalescent type system

extern crate alloc;

use crate::ast::{
    AstNode, Decl, Expr, LetBinding, Parameter, Program, Stmt, Type, TypeDef, TypeParameter,
    Visibility as AstVisibility,
};
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::{CoreType, GenericTypeParameter};
use alloc::{collections::BTreeMap, format, vec::Vec};

impl TypeChecker {
    /// Validate that the program contains exactly one `entry` function.
    pub fn validate_entry_points(program: &Program) -> Result<(), TypeError> {
        let entry_declarations = program
            .declarations
            .iter()
            .filter_map(|decl| {
                if let Decl::Function { is_entry, span, .. } = *decl {
                    is_entry.then_some(span)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if entry_declarations.is_empty() {
            return Err(TypeError::MissingEntryPoint {
                span: TypeError::unknown_span(),
            });
        }

        if entry_declarations.len() > 1 {
            let duplicate_span = entry_declarations
                .first()
                .copied()
                .map_or_else(TypeError::unknown_span, TypeError::span_from_span);
            return Err(TypeError::DuplicateEntryPoint {
                count: entry_declarations.len(),
                span: duplicate_span,
            });
        }

        Ok(())
    }

    /// Convert an AST type into a core type while resolving generic identifiers
    /// against the provided function-level generic bindings.
    fn ast_type_to_core_type_with_generics(
        ast_type: &Type,
        generic_bindings: &[(alloc::string::String, CoreType)],
    ) -> Result<CoreType, TypeError> {
        match *ast_type {
            Type::Basic { ref name, .. } => {
                if let Some(core_type) = generic_bindings
                    .iter()
                    .find_map(|binding| (&binding.0 == name).then_some(&binding.1))
                {
                    return Ok(core_type.clone());
                }
                match Self::ast_type_to_core_type(ast_type) {
                    Ok(core_type) => Ok(core_type),
                    Err(TypeError::TypeNotFound { type_name, .. }) => Ok(CoreType::Generic {
                        name: type_name,
                        type_args: Vec::new(),
                    }),
                    Err(other) => Err(other),
                }
            }
            Type::Array {
                ref element_type, ..
            } => Ok(CoreType::Array(alloc::boxed::Box::new(
                Self::ast_type_to_core_type_with_generics(element_type, generic_bindings)?,
            ))),
            Type::Generic {
                ref name,
                ref type_args,
                ..
            } => {
                let mut resolved_args = Vec::new();
                for type_arg in type_args {
                    resolved_args.push(Self::ast_type_to_core_type_with_generics(
                        type_arg,
                        generic_bindings,
                    )?);
                }
                Ok(CoreType::Generic {
                    name: name.clone(),
                    type_args: resolved_args,
                })
            }
            Type::Function {
                ref parameters,
                ref return_types,
                ref errors,
                ..
            } => {
                let mut resolved_params = Vec::new();
                for parameter in parameters {
                    resolved_params.push(Self::ast_type_to_core_type_with_generics(
                        parameter,
                        generic_bindings,
                    )?);
                }
                let mut resolved_returns = Vec::new();
                for return_type in return_types {
                    resolved_returns.push(Self::ast_type_to_core_type_with_generics(
                        return_type,
                        generic_bindings,
                    )?);
                }
                let mut resolved_errors = Vec::new();
                if let Some(ref error_types) = *errors {
                    for error_type in error_types {
                        resolved_errors.push(Self::ast_type_to_core_type_with_generics(
                            error_type,
                            generic_bindings,
                        )?);
                    }
                }
                Ok(CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: resolved_params,
                    return_types: resolved_returns,
                    error_types: resolved_errors,
                })
            }
        }
    }

    /// Convert AST-level visibility into the internal representation, accounting for entry points.
    const fn convert_visibility(visibility: &AstVisibility, is_entry: bool) -> Visibility {
        if is_entry {
            Visibility::Entry
        } else {
            match *visibility {
                AstVisibility::Public => Visibility::Public,
                AstVisibility::Private => Visibility::Private,
            }
        }
    }

    /// Register a declaration's symbol signature prior to body checking so forward references succeed.
    #[expect(
        clippy::too_many_lines,
        reason = "Signature registration handles generics, return types, errors, and visibility in one pass"
    )]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Matching on &Decl requires referencing the variant while avoiding clones"
    )]
    fn register_declaration_signature(&mut self, decl: &Decl) -> Result<(), TypeError> {
        match decl {
            &Decl::Function {
                name: ref function_name,
                ref generic_constraints,
                ref parameters,
                ref return_types,
                ref error_types,
                visibility: ref decl_visibility,
                is_entry,
                span,
                ..
            } => {
                let mut generic_bindings: Vec<(alloc::string::String, CoreType)> = Vec::new();
                let mut parameter_types = Vec::with_capacity(parameters.len());
                let mut generic_core_params = Vec::new();
                if let Some(declarations) = generic_constraints.as_ref() {
                    for declaration in declarations {
                        let variable_core =
                            self.fresh_type_var(declaration.name.clone(), declaration.span)?;
                        let CoreType::Variable(type_var) = variable_core else {
                            return Err(TypeError::ConstraintSolvingFailed {
                                reason: "failed to allocate generic type variable".to_owned(),
                                span: TypeError::span_from_span(declaration.span),
                            });
                        };

                        let mut constraint_types = Vec::new();
                        for constraint in &declaration.constraints {
                            let resolved_constraint = match Self::ast_type_to_core_type(constraint)
                            {
                                Ok(core_type) => core_type,
                                Err(TypeError::TypeNotFound { type_name, .. }) => {
                                    CoreType::Generic {
                                        name: type_name,
                                        type_args: Vec::new(),
                                    }
                                }
                                Err(other) => return Err(other),
                            };
                            constraint_types.push(resolved_constraint);
                        }

                        generic_core_params.push(GenericTypeParameter {
                            name: declaration.name.clone(),
                            type_var: type_var.clone(),
                            constraints: constraint_types,
                        });
                        generic_bindings
                            .push((declaration.name.clone(), CoreType::Variable(type_var)));
                    }
                }

                for param in parameters {
                    parameter_types.push(Self::ast_type_to_core_type_with_generics(
                        &param.param_type,
                        generic_bindings.as_slice(),
                    )?);
                }

                let return_core_types = return_types
                    .as_deref()
                    .map(|ast_return_types| {
                        ast_return_types
                            .iter()
                            .map(|ast_return_type| {
                                Self::ast_type_to_core_type_with_generics(
                                    ast_return_type,
                                    generic_bindings.as_slice(),
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?
                    .unwrap_or_else(|| vec![CoreType::Unit]);

                let core_errors = self.resolve_error_types(error_types, span)?;

                let function_type = CoreType::Function {
                    generic_params: generic_core_params,
                    parameters: parameter_types,
                    return_types: return_core_types,
                    error_types: core_errors,
                };

                if let CoreType::Function {
                    parameters: declared_parameters,
                    return_types: declared_returns,
                    ..
                } = &function_type
                {
                    self.record_function_hot_reload_metadata(
                        function_name,
                        declared_parameters,
                        declared_returns,
                    );
                }

                let resolved_visibility = Self::convert_visibility(decl_visibility, is_entry);
                self.symbol_table.register(SymbolInfo {
                    name: function_name.clone(),
                    symbol_type: SymbolType::Function,
                    core_type: function_type,
                    visibility: resolved_visibility,
                    source_location: span,
                    is_let_binding: false,
                    is_mutable: false,
                    read_count: 0,
                });
                Ok(())
            }
            Decl::Let {
                binding,
                visibility,
                ..
            } => {
                if let Some(annotation) = binding.type_annotation.as_ref() {
                    let annotated_type = Self::ast_type_to_core_type(annotation)?;
                    let symbol_type = if binding.is_mutable {
                        SymbolType::Variable
                    } else {
                        SymbolType::Constant
                    };
                    let resolved_visibility = Self::convert_visibility(visibility, false);
                    self.symbol_table.register(SymbolInfo {
                        name: binding.name.clone(),
                        symbol_type,
                        core_type: annotated_type,
                        visibility: resolved_visibility,
                        source_location: binding.span,
                        is_let_binding: true,
                        is_mutable: binding.is_mutable,
                        read_count: 0,
                    });
                }
                Ok(())
            }
            Decl::Type {
                name,
                type_def,
                generic_constraints,
                ..
            } => {
                let mut generic_bindings: Vec<(alloc::string::String, CoreType)> = Vec::new();
                if let Some(declarations) = generic_constraints.as_ref() {
                    for declaration in declarations {
                        let variable_core =
                            self.fresh_type_var(declaration.name.clone(), declaration.span)?;
                        generic_bindings.push((declaration.name.clone(), variable_core));
                    }
                }

                let mut generic_core_params = Vec::new();
                if let Some(declarations) = generic_constraints.as_ref() {
                    for declaration in declarations {
                        let Some(variable_core) = generic_bindings.iter().find_map(|binding| {
                            (binding.0 == declaration.name).then_some(binding.1.clone())
                        }) else {
                            return Err(TypeError::ConstraintSolvingFailed {
                                reason: format!(
                                    "failed to allocate type variable for generic parameter '{}'",
                                    declaration.name
                                ),
                                span: TypeError::span_from_span(declaration.span),
                            });
                        };
                        let CoreType::Variable(type_var) = variable_core else {
                            return Err(TypeError::ConstraintSolvingFailed {
                                reason: "failed to allocate generic type variable".to_owned(),
                                span: TypeError::span_from_span(declaration.span),
                            });
                        };

                        let mut constraint_types = Vec::new();
                        for constraint in &declaration.constraints {
                            let resolved_constraint = Self::ast_type_to_core_type_with_generics(
                                constraint,
                                &generic_bindings,
                            )?;
                            constraint_types.push(resolved_constraint);
                        }

                        generic_core_params.push(GenericTypeParameter {
                            name: declaration.name.clone(),
                            type_var,
                            constraints: constraint_types,
                        });
                    }
                }

                let generic_type_args = generic_bindings
                    .iter()
                    .map(|binding| binding.1.clone())
                    .collect::<Vec<CoreType>>();
                let nominal_type = CoreType::Generic {
                    name: name.clone(),
                    type_args: generic_type_args,
                };
                self.environment_mut()
                    .register_type(name.clone(), nominal_type.clone());
                self.register_adt_generic_params(name.clone(), generic_core_params);
                self.symbol_table.register(SymbolInfo {
                    name: name.clone(),
                    symbol_type: SymbolType::Type,
                    core_type: nominal_type,
                    visibility: Visibility::Public,
                    source_location: decl.span(),
                    is_let_binding: false,
                    is_mutable: false,
                    read_count: 0,
                });

                if let TypeDef::Sum { variants, .. } = type_def {
                    let mut qualified_variants = Vec::new();
                    for variant in variants {
                        let qualified_name = format!("{name}.{}", variant.name);
                        qualified_variants.push(qualified_name.clone());
                        let mut variant_fields: BTreeMap<String, CoreType> = BTreeMap::new();
                        for field in &variant.fields {
                            let core_field_type = Self::ast_type_to_core_type_with_generics(
                                &field.type_annotation,
                                generic_bindings.as_slice(),
                            )?;
                            variant_fields.insert(field.name.clone(), core_field_type.clone());
                            self.symbol_table.register(SymbolInfo {
                                name: format!("{qualified_name}.{}", field.name),
                                symbol_type: SymbolType::Variable,
                                core_type: core_field_type,
                                visibility: Visibility::Public,
                                source_location: field.span,
                                is_let_binding: false,
                                is_mutable: false,
                                read_count: 0,
                            });
                        }
                        self.register_adt_fields(qualified_name.clone(), variant_fields);
                        self.symbol_table.register(SymbolInfo {
                            name: qualified_name,
                            symbol_type: SymbolType::Constant,
                            core_type: CoreType::Generic {
                                name: name.clone(),
                                type_args: generic_bindings
                                    .iter()
                                    .map(|binding| binding.1.clone())
                                    .collect::<Vec<CoreType>>(),
                            },
                            visibility: Visibility::Public,
                            source_location: variant.span,
                            is_let_binding: false,
                            is_mutable: false,
                            read_count: 0,
                        });
                    }
                    self.adt_variants.insert(name.clone(), qualified_variants);
                } else if let TypeDef::Product { fields, .. } = type_def {
                    let mut product_fields: BTreeMap<String, CoreType> = BTreeMap::new();
                    for field in fields {
                        let core_field_type = Self::ast_type_to_core_type_with_generics(
                            &field.type_annotation,
                            generic_bindings.as_slice(),
                        )?;
                        product_fields.insert(field.name.clone(), core_field_type.clone());
                        self.symbol_table.register(SymbolInfo {
                            name: format!("{name}.{}", field.name),
                            symbol_type: SymbolType::Variable,
                            core_type: core_field_type,
                            visibility: Visibility::Public,
                            source_location: field.span,
                            is_let_binding: false,
                            is_mutable: false,
                            read_count: 0,
                        });
                    }
                    self.register_adt_fields(name.clone(), product_fields);
                }
                Ok(())
            }
            Decl::Import { .. } => Ok(()),
        }
    }

    /// Type check a top-level declaration and update symbol/type environments accordingly.
    #[expect(
        clippy::ref_patterns,
        reason = "Pattern matching with ref to borrow fields from &Decl without cloning large AST nodes"
    )]
    fn type_check_declaration(&mut self, decl: &Decl) -> Result<(), TypeError> {
        match *decl {
            Decl::Function {
                ref generic_constraints,
                ref parameters,
                ref return_types,
                ref error_types,
                ref body,
                span,
                ..
            } => self.type_check_function_declaration(
                generic_constraints.as_deref(),
                parameters.as_slice(),
                return_types.as_deref(),
                error_types,
                body,
                span,
            ),
            Decl::Let {
                ref binding,
                ref initializer,
                ref visibility,
                ..
            } => self.type_check_let_declaration(binding, initializer, visibility),
            Decl::Type { .. } | Decl::Import { .. } => Ok(()),
        }
    }

    /// Type check a function body within a dedicated parameter scope, enforcing return compatibility.
    fn type_check_function_declaration(
        &mut self,
        generic_constraints: Option<&[TypeParameter]>,
        parameters: &[Parameter],
        return_types: Option<&[Type]>,
        error_types: &[String],
        body: &Stmt,
        span: crate::token::Span,
    ) -> Result<(), TypeError> {
        let mut generic_bindings: Vec<(alloc::string::String, CoreType)> = Vec::new();
        if let Some(declarations) = generic_constraints {
            for declaration in declarations {
                let variable_core =
                    self.fresh_type_var(declaration.name.clone(), declaration.span)?;
                generic_bindings.push((declaration.name.clone(), variable_core));
            }
        }

        let mut parameter_types = Vec::with_capacity(parameters.len());
        for param in parameters {
            parameter_types.push(Self::ast_type_to_core_type_with_generics(
                &param.param_type,
                generic_bindings.as_slice(),
            )?);
        }

        let return_core_types = return_types
            .map(|ast_return_types| {
                ast_return_types
                    .iter()
                    .map(|ast_return_type| {
                        Self::ast_type_to_core_type_with_generics(
                            ast_return_type,
                            generic_bindings.as_slice(),
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_else(|| vec![CoreType::Unit]);

        let core_errors = self.resolve_error_types(error_types, span)?;

        self.symbol_table.enter_function(core_errors, span);
        self.begin_return_context();

        let result = self.within_new_scope(|checker| -> Result<(), TypeError> {
            for (param, core_type) in parameters.iter().zip(parameter_types.iter()) {
                checker.symbol_table.register(SymbolInfo {
                    name: param.name.clone(),
                    symbol_type: SymbolType::Variable,
                    core_type: core_type.clone(),
                    visibility: Visibility::Private,
                    source_location: param.span(),
                    is_let_binding: false,
                    is_mutable: false,
                    read_count: 0,
                });
            }

            checker.type_check_stmt_with_return(body, Some(return_core_types.as_slice()))
        });

        self.end_return_context();
        self.symbol_table.exit_function();

        result
    }

    /// Type check a module-level let declaration and ensure the registered symbol honors visibility.
    fn type_check_let_declaration(
        &mut self,
        binding: &LetBinding,
        initializer: &Expr,
        visibility: &AstVisibility,
    ) -> Result<(), TypeError> {
        self.type_check_let_statement(binding, Some(initializer))?;

        let inferred_type = self
            .symbol_table()
            .lookup(&binding.name)
            .map(|info| info.core_type.clone())
            .ok_or_else(|| TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "Binding '{}' failed to register during top-level let processing",
                    binding.name
                ),
                span: TypeError::span_from_span(binding.span),
            })?;

        let symbol_type = if binding.is_mutable {
            SymbolType::Variable
        } else {
            SymbolType::Constant
        };
        let visibility = Self::convert_visibility(visibility, false);
        self.symbol_table.register(SymbolInfo {
            name: binding.name.clone(),
            symbol_type,
            core_type: inferred_type,
            visibility,
            source_location: binding.span,
            is_let_binding: true,
            is_mutable: binding.is_mutable,
            read_count: 0,
        });

        Ok(())
    }

    /// Type check an entire program, collecting all discovered errors.
    pub fn type_check_program(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        self.clear_constraints();
        self.clear_warnings();
        self.clear_expression_metadata();

        let mut errors: Vec<TypeError> = Vec::new();
        let mut skipped_decls: Vec<usize> = Vec::new();

        for decl in &program.declarations {
            if let Err(error) = self.register_declaration_signature(decl) {
                skipped_decls.push(decl.node_id().0);
                errors.push(error);
            }
        }

        for decl in &program.declarations {
            if skipped_decls.contains(&decl.node_id().0) {
                continue;
            }

            if let Err(error) = self.type_check_declaration(decl) {
                errors.push(error);
            }
        }

        if errors.is_empty() {
            if let Err(error) = self.solve_constraints() {
                errors.push(error);
            }
        } else {
            self.clear_constraints();
        }

        if errors.is_empty() {
            if let Err(entry_error) = Self::validate_entry_points(program) {
                Err(vec![entry_error])
            } else {
                let unused_bindings: Vec<(alloc::string::String, crate::token::Span)> = self
                    .symbol_table()
                    .unused_let_bindings()
                    .iter()
                    .map(|binding| (binding.name.clone(), binding.source_location))
                    .collect();
                for (name, source_location) in unused_bindings {
                    self.push_warning(crate::type_system::errors::Warning::UnusedVariable {
                        name,
                        span: TypeError::span_from_span(source_location),
                        suppression_annotation: None,
                    });
                }
                Ok(())
            }
        } else {
            Err(errors)
        }
    }
}
