//! Function modifiers for declarations

/// Function modifiers (pure, untested)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionModifier {
    /// Pure function (no side effects)
    Pure,
    /// Untested function
    Untested,
}
