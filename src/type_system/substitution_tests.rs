//! Quantified callable signatures remain callable after ordinary constraint solving.
use super::substitution::Substitution;
use super::types::{CoreType, GenericTypeParameter, TypeVar};

#[test]
fn substitutions_preserve_bound_callable_variables_and_generic_metadata() {
    let variable = TypeVar {
        id: 7,
        name: "T".to_owned(),
    };
    let signature = CoreType::Function {
        generic_params: vec![GenericTypeParameter {
            name: "T".to_owned(),
            type_var: variable.clone(),
            constraints: Vec::new(),
        }],
        parameters: vec![CoreType::Variable(variable.clone())],
        return_types: vec![CoreType::Variable(variable)],
        error_types: Vec::new(),
    };
    assert_eq!(Substitution::empty().apply(&signature), signature);
    // Imported signatures may use the same numeric variable ID as local free
    // variables. Solving the latter must not instantiate a quantified import.
    assert_eq!(
        Substitution::single(7, CoreType::Int64).apply(&signature),
        signature
    );
}
