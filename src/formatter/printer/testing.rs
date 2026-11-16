//! Canonical formatting for structured test-only statements.
use super::Formatter;
use crate::ast::Expr;
use crate::ast::testing::{
    Comparison, ExpectedCallCount, Matcher, SequenceRelation, StubBehavior, TestingStatement,
};

/// Render an exact rule or explicit deterministic fallback mode.
fn print_stub(
    formatter: &Formatter,
    target: &Expr,
    arguments: Option<&[Expr]>,
    behavior: &StubBehavior,
    depth: usize,
) -> String {
    let indent = formatter.indent(depth);
    let target_text = formatter.print_expr(target, depth);
    let arguments_text = arguments.map_or_else(String::new, |values| {
        format!(
            "({})",
            values
                .iter()
                .map(|value| formatter.print_expr(value, depth))
                .collect::<Vec<_>>()
                .join(", ")
        )
    });
    match *behavior {
        StubBehavior::FailTest => format!(
            "{indent}stub {target_text}:\n{}otherwise fail the test",
            formatter.indent(depth.saturating_add(1))
        ),
        StubBehavior::Return(ref value) => format!(
            "{indent}stub {target_text}{arguments_text} returns {}",
            formatter.print_expr(value, depth)
        ),
        StubBehavior::ApplicationError(ref error) => format!(
            "{indent}stub {target_text}{arguments_text} fails with {}",
            error.path.join(".")
        ),
        StubBehavior::Replacement(ref replacement) => format!(
            "{indent}stub {target_text} using {}",
            formatter.print_expr(replacement, depth)
        ),
        StubBehavior::SpyOriginal => format!("{indent}spy {target_text} using original"),
    }
}

/// Render the finite assertion grammar without stringifying AST operands.
pub(super) fn print_testing_statement(
    formatter: &Formatter,
    statement: &TestingStatement,
    depth: usize,
) -> String {
    match *statement {
        TestingStatement::Verify(ref target) => format!(
            "{}verify {}",
            formatter.indent(depth),
            formatter.print_expr(target, depth)
        ),
        TestingStatement::Restore(ref target) => format!(
            "{}restore {}",
            formatter.indent(depth),
            formatter.print_expr(target, depth)
        ),
        TestingStatement::Stub {
            ref target,
            ref arguments,
            ref behavior,
        } => print_stub(formatter, target, arguments.as_deref(), behavior, depth),
        TestingStatement::Expect(ref expectation) => {
            let actual = formatter.print_expr(&expectation.actual, depth);
            let matcher = match expectation.matcher {
                Matcher::Equal {
                    ref expected,
                    negated,
                } => format!(
                    "{}to be {}",
                    if negated { "not " } else { "" },
                    formatter.print_expr(expected, depth)
                ),
                Matcher::Compare {
                    ref expected,
                    relation,
                } => {
                    let phrase = match relation {
                        Comparison::Greater => "greater than",
                        Comparison::Less => "less than",
                        Comparison::AtLeast => "at least",
                        Comparison::AtMost => "at most",
                    };
                    format!("to be {phrase} {}", formatter.print_expr(expected, depth))
                }
                Matcher::Sequence {
                    ref expected,
                    relation,
                    negated,
                } => {
                    let phrase = match relation {
                        SequenceRelation::Contain | SequenceRelation::ExactlyInOrder => "contain",
                        SequenceRelation::StartWith => "start with",
                        SequenceRelation::EndWith => "end with",
                    };
                    let exact = if relation == SequenceRelation::ExactlyInOrder {
                        "exactly "
                    } else {
                        ""
                    };
                    let suffix = if relation == SequenceRelation::ExactlyInOrder {
                        " in order"
                    } else {
                        ""
                    };
                    format!(
                        "{}to {phrase} {exact}{}{suffix}",
                        if negated { "not " } else { "" },
                        formatter.print_expr(expected, depth)
                    )
                }
                Matcher::CallCount { ref count } => match *count {
                    ExpectedCallCount::Once => "to have been called once".to_owned(),
                    ExpectedCallCount::Never => "to have never been called".to_owned(),
                    ExpectedCallCount::Exactly(ref count_expression) => format!(
                        "to have been called exactly {} times",
                        formatter.print_expr(count_expression, depth)
                    ),
                },
                Matcher::CalledWith { ref arguments } => format!(
                    "to have been called with ({})",
                    arguments
                        .iter()
                        .map(|argument| formatter.print_expr(argument, depth))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                Matcher::Empty { negated } => {
                    format!("{}to be empty", if negated { "not " } else { "" })
                }
                Matcher::Variant {
                    ref variant,
                    ref nominal_type,
                } => format!(
                    "to be the {variant} variant of {}",
                    nominal_type.path.join(".")
                ),
            };
            let indent = formatter.indent(depth);
            expectation.eventually_seconds.as_ref().map_or_else(
                || format!("{indent}expect {actual} {matcher}"),
                |deadline| format!("{indent}expect eventually {actual}\n{indent}{}{}\n{indent}within {} seconds", formatter.indent(1), matcher, formatter.print_expr(deadline, depth)),
            )
        }
    }
}
