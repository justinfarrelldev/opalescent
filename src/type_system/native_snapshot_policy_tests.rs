//! The frontend and backend share one explicit value-snapshot capability contract.
use super::native_double_signature::supports_argument_snapshot;
use super::types::CoreType;

/// Construct an ordinary nominal type identity without granting opaque layout access.
fn nominal(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: Vec::new(),
    }
}

#[test]
fn snapshot_policy_admits_managed_values_and_explicit_paths_not_unknown_layouts() {
    assert!(supports_argument_snapshot(&CoreType::Int64));
    assert!(supports_argument_snapshot(&CoreType::String));
    assert!(supports_argument_snapshot(&CoreType::Array(Box::new(
        CoreType::Array(Box::new(CoreType::String))
    ))));
    assert!(supports_argument_snapshot(&nominal("FilesystemPath")));
    assert!(!supports_argument_snapshot(&CoreType::Array(Box::new(
        nominal("FilesystemPath")
    ))));
    assert!(!supports_argument_snapshot(&nominal(
        "__opal_type::module::FilesystemPath"
    )));
    assert!(!supports_argument_snapshot(&nominal("UserRecord")));
}
