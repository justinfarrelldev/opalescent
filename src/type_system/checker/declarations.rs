//! Declaration type checking for the Opalescent type system

extern crate alloc;

use crate::ast::{
    AstNode, Decl, Expr, LetBinding, Parameter, Program, Stmt, Type, Visibility as AstVisibility,
};
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::{boxed::Box, format, vec::Vec};

impl TypeChecker {
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
        clippy::pattern_type_mismatch,
        reason = "Matching on &Decl requires referencing the variant while avoiding clones"
    )]
    fn register_declaration_signature(&mut self, decl: &Decl) -> Result<(), TypeError> {
        match decl {
            &Decl::Function {
                name: ref function_name,
                ref parameters,
                ref return_type,
                ref error_types,
                visibility: ref decl_visibility,
                is_entry,
                span,
                ..
            } => {
                let mut parameter_types = Vec::with_capacity(parameters.len());
                for param in parameters {
                    parameter_types.push(Self::ast_type_to_core_type(&param.param_type)?);
                }

                let return_core = return_type
                    .as_ref()
                    .map(Self::ast_type_to_core_type)
                    .transpose()?
                    .unwrap_or(CoreType::Unit);

                // Map error type names into nominal core types for function signature
                let mut core_errors: Vec<CoreType> = Vec::with_capacity(error_types.len());
                for name in error_types {
                    core_errors.push(CoreType::Generic {
                        name: name.clone(),
                        type_args: Vec::new(),
                    });
                }

                let function_type = CoreType::Function {
                    parameters: parameter_types,
                    return_type: Box::new(return_core),
                    error_types: core_errors,
                };

                let resolved_visibility = Self::convert_visibility(decl_visibility, is_entry);
                self.symbol_table.register(SymbolInfo {
                    name: function_name.clone(),
                    symbol_type: SymbolType::Function,
                    core_type: function_type,
                    visibility: resolved_visibility,
                    source_location: span,
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
                    });
                }
                Ok(())
            }
            Decl::Type { .. } | Decl::Import { .. } => Ok(()),
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
                ref parameters,
                ref return_type,
                ref body,
                ..
            } => self.type_check_function_declaration(
                parameters.as_slice(),
                return_type.as_ref(),
                body,
            ),
            Decl::Let {
                ref binding,
                ref initializer,
                ref visibility,
                ..
            } => self.type_check_let_declaration(binding, initializer, visibility),
            Decl::Type { ref type_def, .. } => Self::validate_adt_type(type_def),
            Decl::Import { .. } => {
                // Phase 4 will introduce full import validation; until then we simply acknowledge the declaration.
                Ok(())
            }
        }
    }

    /// Type check a function body within a dedicated parameter scope, enforcing return compatibility.
    fn type_check_function_declaration(
        &mut self,
        parameters: &[Parameter],
        return_type: Option<&Type>,
        body: &Stmt,
    ) -> Result<(), TypeError> {
        let mut parameter_types = Vec::with_capacity(parameters.len());
        for param in parameters {
            parameter_types.push(Self::ast_type_to_core_type(&param.param_type)?);
        }

        let return_core = return_type
            .map(Self::ast_type_to_core_type)
            .transpose()?
            .unwrap_or(CoreType::Unit);

        self.within_new_scope(|checker| -> Result<(), TypeError> {
            for (param, core_type) in parameters.iter().zip(parameter_types.iter()) {
                checker.symbol_table.register(SymbolInfo {
                    name: param.name.clone(),
                    symbol_type: SymbolType::Variable,
                    core_type: core_type.clone(),
                    visibility: Visibility::Private,
                    source_location: param.span(),
                });
            }

            checker.type_check_stmt_with_return(body, Some(&return_core))
        })
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
        });

        Ok(())
    }

    /// Type check an entire program, collecting all discovered errors.
    pub fn type_check_program(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        self.clear_constraints();

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
            Ok(())
        } else {
            Err(errors)
        }
    }
}
