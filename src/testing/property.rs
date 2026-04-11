//! Property-based testing primitives.

extern crate alloc;

use alloc::vec::Vec;

/// A property test definition with generation and shrinking hooks.
#[derive(Debug, Clone)]
pub struct PropertyTest<ValueType>
where
    ValueType: Clone,
{
    /// Property name for reporting.
    pub name: &'static str,
    /// Value generator using iteration index as deterministic seed.
    pub generate: fn(usize) -> ValueType,
    /// Shrink strategy producing candidate simpler values.
    pub shrink: fn(&ValueType) -> Vec<ValueType>,
    /// Property predicate that must hold for each generated value.
    pub property_fn: fn(&ValueType) -> bool,
}

/// Failure details for a property check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyFailure<ValueType>
where
    ValueType: Clone,
{
    /// Iteration index where property first failed.
    pub iteration: usize,
    /// Counter-example discovered by the checker.
    pub counter_example: ValueType,
}

/// Result of running a property test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyCheckResult<ValueType>
where
    ValueType: Clone,
{
    /// Property held across all iterations.
    Passed,
    /// Property failed with shrink information.
    Failed(PropertyFailure<ValueType>),
}

/// Run `iterations` checks for `test_case`.
#[must_use]
pub fn check_property<ValueType>(
    test_case: &PropertyTest<ValueType>,
    iterations: usize,
) -> PropertyCheckResult<ValueType>
where
    ValueType: Clone,
{
    let mut iteration = 0_usize;
    while iteration < iterations {
        let initial_value = (test_case.generate)(iteration);
        if !(test_case.property_fn)(&initial_value) {
            let reduced = shrink_counter_example(test_case, initial_value);
            return PropertyCheckResult::Failed(PropertyFailure {
                iteration,
                counter_example: reduced,
            });
        }
        iteration = iteration.saturating_add(1);
    }

    PropertyCheckResult::Passed
}

/// Repeatedly shrink a failing counter-example to a smaller failing case.
fn shrink_counter_example<ValueType>(
    test_case: &PropertyTest<ValueType>,
    initial: ValueType,
) -> ValueType
where
    ValueType: Clone,
{
    let mut current = initial;
    loop {
        let candidates = (test_case.shrink)(&current);
        let mut next_failure: Option<ValueType> = None;
        for candidate in candidates {
            if !(test_case.property_fn)(&candidate) {
                next_failure = Some(candidate);
                break;
            }
        }

        match next_failure {
            Some(value) => {
                current = value;
            }
            None => {
                return current;
            }
        }
    }
}
