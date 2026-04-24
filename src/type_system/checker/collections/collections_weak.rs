extern crate alloc;

use crate::type_system::checker::TypeChecker;
use crate::type_system::types::CoreType;
use alloc::vec;

impl TypeChecker {
    pub(super) fn resolve_weak_member_call(
        receiver_type: &CoreType,
        member_name: &str,
    ) -> Option<CoreType> {
        let CoreType::Generic {
            ref name,
            ref type_args,
        } = *receiver_type
        else {
            return None;
        };
        if name != "Weak" {
            return None;
        }
        let inner = type_args.first().cloned().unwrap_or(CoreType::Unit);
        match member_name {
            "upgrade" => Some(CoreType::Function {
                generic_params: vec![],
                parameters: vec![],
                return_types: vec![CoreType::Generic {
                    name: alloc::string::String::from("Option"),
                    type_args: vec![inner],
                }],
                error_types: vec![],
            }),
            _ => None,
        }
    }
}
