//! Test-module signatures precede double dispatch contracts and callback bodies.
use super::TypeChecker;
use crate::ast::{Decl, Expr, Program};
use crate::type_system::ModuleInterface;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::Visibility;

impl TypeChecker {
    /// Publish a provisional, typed interface without executing or checking
    /// helper bodies before their double signatures/state fields are available.
    /// Constants are type-checked exactly once; their source functions do not run.
    pub(crate) fn native_prepare_test_values(
        &mut self,
        program: &Program,
    ) -> Result<(Program, ModuleInterface), Vec<TypeError>> {
        let mut errors = Vec::new();
        let mut functions = Vec::new();
        let mut constants = Vec::new();
        for declaration in &program.declarations {
            if let Some(error) = Self::validate_function_doc_comment(declaration) {
                errors.push(error);
                continue;
            }
            if matches!(
                *declaration,
                Decl::Function { .. }
                    | Decl::Let {
                        initializer: Expr::Lambda { .. },
                        ..
                    }
            ) {
                if let Err(error) = self.register_declaration_signature(declaration) {
                    errors.push(error);
                }
                functions.push(declaration.clone());
            } else {
                constants.push(declaration);
            }
        }
        if !errors.is_empty() {
            self.clear_constraints();
            return Err(errors);
        }
        for declaration in constants {
            if let Err(error) = self.type_check_declaration(declaration) {
                errors.push(error);
            }
        }
        if errors.is_empty() {
            if let Err(error) = self.solve_constraints() {
                errors.push(error);
            }
        }
        if !errors.is_empty() {
            self.clear_constraints();
            return Err(errors);
        }
        self.sync_current_module_adt_fields();
        let mut interface = self
            .module_interface(&self.current_module_path)
            .unwrap_or_else(|| ModuleInterface::new(self.current_module_path.clone()));
        for declaration in &functions {
            let name = match *declaration {
                Decl::Function { ref name, .. } => name,
                Decl::Let { ref binding, .. } => &binding.name,
                _ => continue,
            };
            if let Some(symbol) = self.symbol_table().lookup(name).cloned() {
                match symbol.visibility {
                    Visibility::Public | Visibility::Entry => {
                        interface.exports.insert(name.clone(), symbol);
                    }
                    Visibility::Private => {
                        interface.private_symbols.insert(name.clone(), symbol);
                    }
                }
            }
            if let Some(labels) = self.function_return_labels(name) {
                interface.register_function_return_labels(name.clone(), labels.to_vec());
            }
            if let Some(modes) = self.function_borrow_kinds_for_symbol(name) {
                interface.register_function_borrow_kinds(name.clone(), modes.to_vec());
            }
        }
        Ok((
            Program {
                declarations: functions,
                span: program.span,
                id: program.id,
            },
            interface,
        ))
    }

    /// Check previously registered test helper bodies after all referenced
    /// application boundaries and provider state layouts are known.
    pub(crate) fn native_check_test_value_bodies(
        &mut self,
        program: &Program,
    ) -> Result<(), Vec<TypeError>> {
        let mut errors = Vec::new();
        for declaration in &program.declarations {
            if let Err(error) = self.type_check_declaration(declaration) {
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
        self.sync_current_module_adt_fields();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
