//! Source-level testing declarations, kept separate from production grammar.
//!
//! References here are unresolved syntax. Resolution produces canonical
//! declaration identities before descriptors or instrumentation are generated.
use super::{ConstructorField, Decl, Documentation, Expr, ImportItem, Type};
use crate::token::Span;

/// An operation available only inside a compiled test artifact.
#[derive(Debug, Clone, PartialEq)]
pub enum TestingStatement {
    /// Typed assertion with captured operands and original location.
    Expect(Expectation),
    /// Typed exact-match rule or explicit fallback configuration.
    Stub {
        /// Nominal double handle expression.
        target: Expr,
        /// Exact rule arguments; absent for fallback configuration.
        arguments: Option<Vec<Expr>>,
        /// Deterministic configured behavior.
        behavior: StubBehavior,
    },
    /// Verify every non-fallback configured rule was used.
    Verify(Expr),
    /// Explicitly deactivate this handle for the current attempt.
    Restore(Expr),
}

/// Typed double rule/fallback behavior, validated against the resolved edge.
#[derive(Debug, Clone, PartialEq)]
pub enum StubBehavior {
    /// Unexpected calls fail through the internal test-abort channel.
    FailTest,
    /// Return the configured ordinary value.
    Return(Expr),
    /// Produce a genuine allowed nominal application error.
    ApplicationError(SourceReference),
    /// Invoke a signature-compatible ordinary replacement function.
    Replacement(Expr),
    /// Forward to the original operation, bypassing its dispatcher.
    SpyOriginal,
}

impl TestingStatement {
    /// Operand expressions in source evaluation order, for existing AST visitors.
    #[must_use]
    pub fn expressions(&self) -> Vec<&Expr> {
        match *self {
            Self::Verify(ref target) | Self::Restore(ref target) => vec![target],
            Self::Stub {
                ref target,
                ref arguments,
                ref behavior,
            } => {
                let mut expressions = vec![target];
                if let Some(ref values) = *arguments {
                    expressions.extend(values);
                }
                match *behavior {
                    StubBehavior::Return(ref value) | StubBehavior::Replacement(ref value) => {
                        expressions.push(value);
                    }
                    StubBehavior::FailTest
                    | StubBehavior::ApplicationError(_)
                    | StubBehavior::SpyOriginal => {}
                }
                expressions
            }
            Self::Expect(ref expectation) => {
                let mut expressions = vec![&expectation.actual];
                match expectation.matcher {
                    Matcher::Equal { ref expected, .. }
                    | Matcher::Compare { ref expected, .. }
                    | Matcher::Sequence { ref expected, .. } => expressions.push(expected),
                    Matcher::CalledWith { ref arguments } => expressions.extend(arguments),
                    Matcher::CallCount {
                        count: ExpectedCallCount::Exactly(ref count),
                    } => expressions.push(count),
                    Matcher::CallCount { .. } | Matcher::Empty { .. } | Matcher::Variant { .. } => {
                    }
                }
                if let Some(ref deadline) = expectation.eventually_seconds {
                    expressions.push(deadline);
                }
                expressions
            }
        }
    }
}

/// Structured assertion, never split from source text using matcher phrases.
#[derive(Debug, Clone, PartialEq)]
pub struct Expectation {
    /// Actual expression (an observation for `eventually`).
    pub actual: Expr,
    /// Fixed matcher vocabulary and typed expected operand.
    pub matcher: Matcher,
    /// Optional polling deadline in seconds, evaluated only once.
    pub eventually_seconds: Option<Expr>,
    /// Original source extent.
    pub span: Span,
}

/// Initial supported assertion vocabulary.
#[derive(Debug, Clone, PartialEq)]
pub enum Matcher {
    /// Supported typed equality or inequality, never stringified equality.
    Equal {
        /// Expected value expression.
        expected: Expr,
        /// Negated assertion.
        negated: bool,
    },
    /// Numeric comparison under ordinary compatibility rules.
    Compare {
        /// Expected bound.
        expected: Expr,
        /// Supported comparison relation.
        relation: Comparison,
    },
    /// String/collection operation with a typed expected operand.
    Sequence {
        /// Expected fragment, prefix, suffix or exact ordered elements.
        expected: Expr,
        /// Supported sequence relation.
        relation: SequenceRelation,
        /// Negated assertion where permitted by the fixed vocabulary.
        negated: bool,
    },
    /// Empty string or collection.
    Empty {
        /// Negated assertion.
        negated: bool,
    },
    /// Recorded invocation count, independent of exact-rule verification.
    CallCount {
        /// Once, never, or an explicit typed count.
        count: ExpectedCallCount,
    },
    /// Supported owned argument snapshots from actual calls.
    CalledWith {
        /// Expected arguments, evaluated in source order exactly once.
        arguments: Vec<Expr>,
    },
    /// A genuine variant of a named nominal type.
    Variant {
        /// Variant identifier resolved against the named type.
        variant: String,
        /// Unresolved nominal type reference.
        nominal_type: SourceReference,
    },
}

/// Fixed call-count assertion vocabulary.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedCallCount {
    /// Exactly one invocation.
    Once,
    /// No invocation.
    Never,
    /// A typed numeric expression evaluated once.
    Exactly(Expr),
}

/// Supported ordered numeric relations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    /// Strictly greater than.
    Greater,
    /// Strictly less than.
    Less,
    /// Greater than or equal.
    AtLeast,
    /// Less than or equal.
    AtMost,
}

/// Supported string and collection relations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceRelation {
    /// Containment of a fragment or supported element.
    Contain,
    /// String prefix.
    StartWith,
    /// String suffix.
    EndWith,
    /// Length, order, multiplicity and element equality.
    ExactlyInOrder,
}

/// An explicitly selected test or test-support module.
#[derive(Debug, Clone, PartialEq)]
pub struct TestModule {
    /// Declarations in original source order.
    pub items: Vec<TestItem>,
    /// Original source extent.
    pub span: Span,
}

/// A declaration within a test module or scenario.
#[derive(Debug, Clone, PartialEq)]
pub enum TestItem {
    /// Ordinary language declaration (including application imports).
    Application(Box<Decl>),
    /// Explicit test-only provider/handle imports.
    SupportImport(TestSupportImport),
    /// Typed interception of a single resolved import edge.
    Double(DoubleDeclaration),
    /// Independent behavior group; not an execution scope for shared state.
    Scenario(ScenarioDeclaration),
    /// Reusable per-attempt environment provider.
    World(WorldDeclaration),
    /// One body, expanded into independent cases by its named examples.
    Test(TestDeclaration),
    /// Per-attempt before/after callback.
    Hook(HookDeclaration),
    /// Scoped, replace-not-append metadata.
    Metadata(TestMetadata),
}

/// Source declaration of an import-edge interception boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoubleDeclaration {
    /// Nominal declaration and generated handle name.
    pub name: String,
    /// Imported callable name (resolved to the edge, not a runtime address).
    pub function: SourceReference,
    /// Imported module specifier.
    pub from: String,
    /// Importing application module specifier.
    pub as_used_by: String,
    /// Retained documentation block.
    pub documentation: Option<Documentation>,
    /// Original source location.
    pub span: Span,
}

/// Test-only declaration import, introducing provider and nominal handle together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSupportImport {
    /// Declaration category selected explicitly in source.
    pub kind: TestSupportKind,
    /// Named imports with aliases and original spans.
    pub items: Vec<ImportItem>,
    /// Module specifier resolved in explicit test-support mode.
    pub source: String,
    /// Original source extent.
    pub span: Span,
}

/// Imported test-only declaration category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSupportKind {
    /// Environment provider and nominal World handle.
    World,
    /// Interception declaration and nominal double handle.
    Double,
}

/// Named values sharing one concrete record schema and compiled body.
#[derive(Debug, Clone, PartialEq)]
pub struct ExampleGroup {
    /// Ordinary type reference; must resolve to a supported record schema.
    pub record_type: Type,
    /// Independently identified cases, in source declaration order.
    pub examples: Vec<ExampleDeclaration>,
    /// Original source extent.
    pub span: Span,
}

/// One independently scheduled named example.
#[derive(Debug, Clone, PartialEq)]
pub struct ExampleDeclaration {
    /// Static title, used in structured case identity (never its position).
    pub title: String,
    /// Typed constant field initializers, validated before execution.
    pub fields: Vec<ConstructorField>,
    /// Retained paragraph documentation.
    pub documentation: Option<Documentation>,
    /// Original source extent.
    pub span: Span,
}

/// A named grouping with inherited metadata and hooks.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioDeclaration {
    /// Explicit title, or absent for a subject-derived title.
    pub title: Option<String>,
    /// Nested declarations in source order.
    pub items: Vec<TestItem>,
    /// Retained documentation block.
    pub documentation: Option<Documentation>,
    /// Original source location.
    pub span: Span,
}

/// A reusable World provider, not an alias for its state type.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldDeclaration {
    /// Nominal provider and generated handle name.
    pub name: String,
    /// Ordered dependency references, resolved before checking callbacks.
    pub dependencies: Vec<SourceReference>,
    /// Exactly one setup callback.
    pub setup: ManagedCallback,
    /// Optional, explicit cleanup; absence is legal.
    pub cleanup: Option<ManagedCallback>,
    /// Retained documentation block.
    pub documentation: Option<Documentation>,
    /// Original source location.
    pub span: Span,
}

/// An ordinary typed function literal managed by the framework.
#[derive(Debug, Clone, PartialEq)]
pub struct ManagedCallback {
    /// Function literal; not a string or interpreted prose.
    pub function: Expr,
    /// Whether nominal application errors must be inferred for this callback.
    pub infer_errors: bool,
}

/// One independently described behavior with exactly one compiled callback.
#[derive(Debug, Clone, PartialEq)]
pub struct TestDeclaration {
    /// Static human-readable title.
    pub title: String,
    /// Test-level overrides in source order.
    pub metadata: Vec<TestMetadata>,
    /// Optional named, statically typed case data.
    pub examples: Option<ExampleGroup>,
    /// Ordinary typed test body.
    pub callback: ManagedCallback,
    /// Retained documentation block.
    pub documentation: Option<Documentation>,
    /// Original source location.
    pub span: Span,
}

/// Per-attempt lifecycle hook.
#[derive(Debug, Clone, PartialEq)]
pub struct HookDeclaration {
    /// True for before-hooks; false for after-hooks.
    pub before: bool,
    /// Ordinary typed callback.
    pub callback: ManagedCallback,
    /// Retained documentation block.
    pub documentation: Option<Documentation>,
    /// Original source location.
    pub span: Span,
}

/// A qualified, unresolved reference with an original source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceReference {
    /// Qualified path segments, not an application call expression.
    pub path: Vec<String>,
    /// Original source extent.
    pub span: Span,
}

/// Declared intent, not measured coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestKind {
    /// Individual application function behavior (the default).
    Unit,
    /// Composed modules or real external operations.
    Integration,
    /// Normal production-binary behavior.
    EndToEnd,
}

/// Scoped test metadata. Empty World lists explicitly mean `none`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestMetadata {
    /// Scheduler-only exclusion within this runner invocation.
    Exclusive {
        /// Complete constant key set acquired before setup.
        keys: Vec<String>,
        /// Original source location.
        span: Span,
    },
    /// Function association, without a claim that it was executed.
    Subjects {
        /// Singular or plural clause; retained to diagnose mixed declarations.
        plural: bool,
        /// Genuine function references, resolved in a later compiler pass.
        references: Vec<SourceReference>,
        /// Original source location.
        span: Span,
    },
    /// Ordered initial body injections, replacing inherited injections.
    Worlds {
        /// Provider references in exact parameter order.
        references: Vec<SourceReference>,
        /// Original source location.
        span: Span,
    },
    /// Activation-only requirements that add no body parameters.
    ActivateWorlds {
        /// Provider references.
        references: Vec<SourceReference>,
        /// Original source location.
        span: Span,
    },
    /// Category override.
    Kind {
        /// Explicit test category.
        kind: TestKind,
        /// Original source location.
        span: Span,
    },
}
