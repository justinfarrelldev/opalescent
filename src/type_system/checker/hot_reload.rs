//! Hot-reload metadata helpers for function signature stability.

extern crate alloc;

use crate::type_system::checker::TypeChecker;
use crate::type_system::types::CoreType;
use alloc::{string::String, vec::Vec};

/// Metadata recorded for each function to support signature change detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionHotReloadMetadata {
    /// Function symbol name.
    pub name: String,
    /// Parameter type signature captured at the latest check.
    pub parameter_types: Vec<CoreType>,
    /// Return type signature captured at the latest check.
    pub return_types: Vec<CoreType>,
    /// Whether the latest signature matches the previously recorded signature.
    pub signature_stable: bool,
}

impl FunctionHotReloadMetadata {
    /// Construct metadata and compute stability against an optional previous snapshot.
    fn from_previous(
        name: String,
        parameter_types: Vec<CoreType>,
        return_types: Vec<CoreType>,
        previous: Option<&Self>,
    ) -> Self {
        let signature_stable = previous.is_none_or(|existing| {
            existing.parameter_types == parameter_types && existing.return_types == return_types
        });

        Self {
            name,
            parameter_types,
            return_types,
            signature_stable,
        }
    }
}

impl TypeChecker {
    /// Record or update function metadata used by the hot-reload ABI classifier.
    pub(super) fn record_function_hot_reload_metadata(
        &mut self,
        function_name: &str,
        parameter_types: &[CoreType],
        return_types: &[CoreType],
    ) {
        let previous = self.function_hot_reload_metadata.get(function_name);
        let metadata = FunctionHotReloadMetadata::from_previous(
            function_name.to_owned(),
            parameter_types.to_vec(),
            return_types.to_vec(),
            previous,
        );
        self.function_hot_reload_metadata
            .insert(function_name.to_owned(), metadata);
    }

    /// Retrieve recorded function hot-reload metadata by symbol name.
    pub fn function_hot_reload_metadata(
        &self,
        function_name: &str,
    ) -> Option<&FunctionHotReloadMetadata> {
        self.function_hot_reload_metadata.get(function_name)
    }
}
