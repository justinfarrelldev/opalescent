//! Compile-time availability checks for test-only declaration metadata.
//!
//! The checks in this module reject production-facing references before module
//! interfaces, generated exports, or artifact metadata can retain test-only
//! symbols. They model availability as checker metadata, never as runtime state.

extern crate alloc;

use super::TypeChecker;
use crate::ast::{DeclarationAnnotation, Type, TypeDef, TypeParameter};
use crate::token::Span;
use crate::type_system::{errors::TypeError, types::CoreType};
use alloc::format;

impl TypeChecker {
    /// Reject production declarations that opt into test-only availability.
    pub(super) fn validate_production_declaration_availability(
        &self,
        declaration_name: &str,
        annotations: &[DeclarationAnnotation],
        _span: Span,
    ) -> Result<(), TypeError> {
        if self.allow_test_only_imports {
            return Ok(());
        }

        for annotation in annotations {
            match *annotation {
                DeclarationAnnotation::Availability {
                    ref value,
                    span: annotation_span,
                } if value == "test_only" => {
                    return Err(test_only_metadata_error(
                        declaration_name,
                        "declares @availability(test_only)",
                        annotation_span,
                    ));
                }
                DeclarationAnnotation::ConstructorVisibility {
                    ref value,
                    span: annotation_span,
                } if value == "test_runner" => {
                    return Err(test_only_metadata_error(
                        declaration_name,
                        "declares @constructor_visibility(test_runner)",
                        annotation_span,
                    ));
                }
                DeclarationAnnotation::Availability { .. }
                | DeclarationAnnotation::ConstructorVisibility { .. }
                | DeclarationAnnotation::AbiTypeId { .. }
                | DeclarationAnnotation::AbiEvolution { .. } => {}
            }
        }

        Ok(())
    }

    /// Reject production generic bounds that syntactically name test-only types.
    pub(super) fn validate_production_type_parameter_constraints(
        &self,
        surface_kind: &str,
        owner_name: &str,
        declarations: Option<&[TypeParameter]>,
    ) -> Result<(), TypeError> {
        if self.allow_test_only_imports {
            return Ok(());
        }

        if let Some(type_parameters) = declarations {
            for declaration in type_parameters {
                for constraint in &declaration.constraints {
                    self.validate_ast_type_names(surface_kind, owner_name, constraint)?;
                }
            }
        }

        Ok(())
    }

    /// Reject production-visible core types that reach a test-only declaration.
    pub(super) fn validate_production_core_type_surface(
        &self,
        surface_kind: &str,
        owner_name: &str,
        core_type: &CoreType,
        span: Span,
    ) -> Result<(), TypeError> {
        if self.allow_test_only_imports {
            return Ok(());
        }

        if let Some((type_name, module_path)) = self
            .module_resolver
            .core_type_test_only_reference(core_type)
        {
            return Err(test_only_type_reference_error(
                surface_kind,
                owner_name,
                type_name.as_str(),
                module_path.as_str(),
                span,
            ));
        }

        Ok(())
    }

    /// Reject production AST type surfaces that syntactically name test-only types.
    pub(super) fn validate_production_ast_type_surface(
        &self,
        surface_kind: &str,
        owner_name: &str,
        ast_type: &Type,
    ) -> Result<(), TypeError> {
        if self.allow_test_only_imports {
            return Ok(());
        }
        self.validate_ast_type_names(surface_kind, owner_name, ast_type)
    }

    /// Reject production type declarations whose fields or aliases name test-only types.
    pub(super) fn validate_production_type_def_surface(
        &self,
        type_name: &str,
        type_def: &TypeDef,
    ) -> Result<(), TypeError> {
        if self.allow_test_only_imports {
            return Ok(());
        }

        match *type_def {
            TypeDef::Product { ref fields, .. } => {
                for field in fields {
                    self.validate_production_ast_type_surface(
                        "type field",
                        type_name,
                        &field.type_annotation,
                    )?;
                }
            }
            TypeDef::Sum { ref variants, .. } => {
                for variant in variants {
                    for field in &variant.fields {
                        self.validate_production_ast_type_surface(
                            "variant field",
                            type_name,
                            &field.type_annotation,
                        )?;
                    }
                }
            }
            TypeDef::Alias {
                ref target_type, ..
            } => {
                self.validate_production_ast_type_surface("type alias", type_name, target_type)?;
            }
            TypeDef::Opaque { .. } => {}
        }

        Ok(())
    }

    /// Recursively inspect an AST type tree for test-only nominal names.
    fn validate_ast_type_names(
        &self,
        surface_kind: &str,
        owner_name: &str,
        ast_type: &Type,
    ) -> Result<(), TypeError> {
        match *ast_type {
            Type::Basic { ref name, span } | Type::Generic { ref name, span, .. } => {
                self.validate_production_type_name_surface(surface_kind, owner_name, name, span)?;
            }
            Type::Array { .. } | Type::Function { .. } => {}
        }

        match *ast_type {
            Type::Basic { .. } => Ok(()),
            Type::Generic { ref type_args, .. } => {
                for type_arg in type_args {
                    self.validate_ast_type_names(surface_kind, owner_name, type_arg)?;
                }
                Ok(())
            }
            Type::Array {
                ref element_type, ..
            } => self.validate_ast_type_names(surface_kind, owner_name, element_type),
            Type::Function {
                ref parameters,
                ref return_types,
                ref errors,
                ..
            } => {
                for parameter in parameters {
                    self.validate_ast_type_names(surface_kind, owner_name, parameter)?;
                }
                for return_type in return_types {
                    self.validate_ast_type_names(surface_kind, owner_name, return_type)?;
                }
                if let Some(error_types) = errors.as_ref() {
                    for error_type in error_types {
                        self.validate_ast_type_names(surface_kind, owner_name, error_type)?;
                    }
                }
                Ok(())
            }
        }
    }

    /// Reject one production nominal type name when it resolves to test-only metadata.
    fn validate_production_type_name_surface(
        &self,
        surface_kind: &str,
        owner_name: &str,
        type_name: &str,
        span: Span,
    ) -> Result<(), TypeError> {
        if let Some(module_path) = self.module_resolver.test_only_type_source(type_name) {
            return Err(test_only_type_reference_error(
                surface_kind,
                owner_name,
                type_name,
                module_path,
                span,
            ));
        }
        Ok(())
    }
}

/// Build the diagnostic for production metadata that opts into test-only authority.
fn test_only_metadata_error(declaration_name: &str, reason: &str, span: Span) -> TypeError {
    TypeError::ConstraintSolvingFailed {
        reason: format!(
            "test-only availability violation: production declaration '{declaration_name}' {reason}"
        ),
        span: TypeError::span_from_span(span),
    }
}

/// Build the diagnostic for production surfaces that reach test-only types.
fn test_only_type_reference_error(
    surface_kind: &str,
    owner_name: &str,
    type_name: &str,
    module_path: &str,
    span: Span,
) -> TypeError {
    TypeError::ConstraintSolvingFailed {
        reason: format!(
            "test-only availability violation: production {surface_kind} '{owner_name}' references test-only type '{type_name}' from '{module_path}'"
        ),
        span: TypeError::span_from_span(span),
    }
}
