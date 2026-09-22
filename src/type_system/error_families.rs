//! Declaration-only families for standard-library error leaf types.

use super::environment::TypeEnvironment;
use super::types::CoreType;

/// A named collection of stdlib error leaves available in an `errors` declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StdlibErrorFamily {
    /// Declaration-only family name.
    pub name: &'static str,
    /// Ordered leaf names covered by the family.
    pub members: &'static [&'static str],
    /// Whether a complete leaf declaration may be shortened to this family.
    pub warning_eligible: bool,
    /// Lower ranks are preferred when later warning selection has overlapping families.
    pub specificity_rank: usize,
}

mod data;
use data::STDLIB_ERROR_FAMILIES;

/// Return stdlib error families in taxonomy order.
pub const fn stdlib_error_families() -> &'static [StdlibErrorFamily] {
    STDLIB_ERROR_FAMILIES
}

/// Look up a declaration-only stdlib error family by name.
pub fn stdlib_error_family(name: &str) -> Option<&'static StdlibErrorFamily> {
    STDLIB_ERROR_FAMILIES
        .iter()
        .find(|family| family.name == name)
}

/// Return whether an emitted leaf error is covered by a declared leaf or family.
pub fn error_type_is_covered_by_declared_type(emitted_leaf: &str, declared_type: &str) -> bool {
    emitted_leaf == declared_type
        || stdlib_error_family(declared_type)
            .is_some_and(|family| family.members.contains(&emitted_leaf))
}

/// Construct the payload-free nominal type used by every stdlib error leaf and family.
pub fn stdlib_error_core_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: Vec::new(),
    }
}

/// Register declaration-only stdlib family names without changing precise leaf signatures.
pub fn register_stdlib_error_family_types(environment: &mut TypeEnvironment) {
    for family in STDLIB_ERROR_FAMILIES {
        environment.register_type(family.name.to_owned(), stdlib_error_core_type(family.name));
        for member in family.members {
            environment.register_type((*member).to_owned(), stdlib_error_core_type(member));
        }
    }
}
