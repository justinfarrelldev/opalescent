//! Resolved declaration identities and stable structured case identities.
use serde::{Deserialize, Serialize};

/// Canonical declaration identity; aliases resolve to this same tuple.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DeclarationIdentity {
    /// Canonical project-relative module path, or a canonical standard module.
    pub module: String,
    /// Declaration name in the defining module (never its local import alias).
    pub name: String,
}

/// Nominal World identity, distinct from the provider's concrete state type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorldIdentity(pub DeclarationIdentity);

/// Nominal test-double declaration and generated handle identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DoubleIdentity(pub DeclarationIdentity);

/// Resolved callable declaration associated with a case, not measured coverage.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FunctionIdentity(pub DeclarationIdentity);

/// One source import edge to be instrumented before optimization.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResolvedImportEdge {
    /// Canonical importing module.
    pub importing_module: String,
    /// Canonical imported callable, independent of an edge's local alias.
    pub imported_function: FunctionIdentity,
}

/// Complete case identity; neither declaration position nor source line is used.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CaseIdentity {
    /// Project-relative module containing the test.
    pub module: String,
    /// Effective scenario names from outermost to innermost.
    pub scenarios: Vec<String>,
    /// Static test title.
    pub test: String,
    /// Static example title, absent for an unparameterized test.
    pub example: Option<String>,
}
