//! A mocked application error must describe a real value, not missing payloads.
extern crate alloc;
use super::{TypeChecker, native_error};
use crate::ast::testing::SourceReference;
use crate::type_system::error_families::stdlib_error_families;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;

impl TypeChecker {
    /// Built-in leaf errors have registered framework construction; ordinary
    /// source errors must select a genuine payload-free nominal sum variant.
    /// A replacement can construct required payloads through ordinary source.
    pub(super) fn validate_native_mock_error_variant(
        &self,
        reference: &SourceReference,
        errors: &[CoreType],
    ) -> Result<(), TypeError> {
        let qualified = reference.path.join(".");
        let genuine = self
            .adt_variants
            .values()
            .any(|variants| variants.contains(&qualified));
        let empty_payload = self
            .adt_fields
            .get(&qualified)
            .is_some_and(alloc::collections::BTreeMap::is_empty);
        let structured = self
            .adt_variants
            .get(&qualified)
            .is_some_and(|variants| !variants.is_empty())
            || self
                .adt_fields
                .get(&qualified)
                .is_some_and(|fields| !fields.is_empty());
        let built_in_leaf = errors.iter().all(|error| matches!(*error, CoreType::Generic { ref name, .. } if stdlib_error_families().iter().any(|family| family.members.contains(&name.as_str()))));
        if (genuine && empty_payload) || (built_in_leaf && !genuine && !structured) {
            return Ok(());
        }
        Err(native_error(
            reference.span,
            "fails with requires a registered built-in leaf error or a payload-free nominal variant; select a variant or use a replacement to construct required error payloads",
        ))
    }
}
