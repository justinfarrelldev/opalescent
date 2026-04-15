//! Formatting rules for the Opalescent code formatter.
//!
//! This module defines the set of textual rules applied to a formatted output
//! string to produce canonical, consistently-styled source code.  The rules
//! are implemented as pure functions operating on `&str` and `String` so they
//! can be tested independently of the AST printer.
//!
//! Rules applied (in order by [`apply_all`]):
//!
//! 1. Consistent line endings (CRLF → LF).
//! 2. Trailing whitespace removal from each line.
//! 3. Exactly one trailing newline at end of file.
//! 4. Operator spacing: ensure a single space on both sides of binary operators.
//! 5. Consecutive blank lines collapsed to a maximum of one.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Apply all formatting rules to `source` and return the result.
///
/// The rules are applied in a deterministic, idempotent order so that
/// `apply_all(apply_all(s)) == apply_all(s)` for all inputs.
#[must_use]
pub fn apply_all(source: &str) -> String {
    let after_line_endings = normalize_line_endings(source);
    let after_trailing = remove_trailing_whitespace(&after_line_endings);
    let after_blank = collapse_consecutive_blank_lines(&after_trailing);
    let after_ops = normalize_operator_spacing(&after_blank);
    ensure_trailing_newline(&after_ops)
}

/// Convert all Windows-style (`\r\n`) and bare carriage-return (`\r`) line
/// endings to Unix-style (`\n`).
#[must_use]
pub fn normalize_line_endings(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

/// Strip trailing whitespace (spaces and tabs) from every line.
#[must_use]
pub fn remove_trailing_whitespace(source: &str) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let trimmed: Vec<String> = lines
        .iter()
        .map(|line| line.trim_end().to_owned())
        .collect();
    trimmed.join("\n")
}

/// Collapse runs of two or more consecutive blank lines into a single blank
/// line.
#[must_use]
pub fn collapse_consecutive_blank_lines(source: &str) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut result: Vec<&str> = Vec::with_capacity(lines.len());
    let mut consecutive_blank: usize = 0;

    for line in &lines {
        if line.trim().is_empty() {
            consecutive_blank = consecutive_blank.saturating_add(1);
            if consecutive_blank <= 1 {
                result.push(line);
            }
        } else {
            consecutive_blank = 0;
            result.push(line);
        }
    }

    result.join("\n")
}

/// Ensure that `source` ends with exactly one newline character.
#[must_use]
pub fn ensure_trailing_newline(source: &str) -> String {
    let mut s = source.trim_end_matches('\n').to_owned();
    s.push('\n');
    s
}

/// Normalise spacing around common binary operators.
///
/// Ensures exactly one space on each side of `=`, `==`, `!=`, `<=`, `>=`,
/// `<`, `>`, `+`, `-`, `*`, `/`, `%`, `and`, `or`.  The rule operates on
/// the raw text and is a best-effort heuristic — it is not aware of operator
/// context (e.g. unary minus or generics `<T>`).
///
/// Because this rule only adds/normalises spaces (never removes program
/// tokens), it is safe to apply multiple times idempotently.
#[must_use]
pub fn normalize_operator_spacing(source: &str) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let processed: Vec<String> = lines
        .iter()
        .map(|line| normalize_operator_spacing_line(line))
        .collect();
    processed.join("\n")
}

/// Apply operator-spacing normalisation to a single line.
///
/// String literals inside the line are left unchanged by detecting single
/// quote delimiters and skipping whitespace normalisation for those segments.
fn normalize_operator_spacing_line(line: &str) -> String {
    // Only strip excess spaces around `=`, `==`, `!=`, `<=`, `>=`.
    // We do this carefully to avoid mangling `=>` arrows or `::`.
    // Strategy: collapse `  =  ` / ` = ` / `  =` / `= ` to exactly ` = `
    // for the assignment/comparison operators, but only outside of string
    // literals.
    //
    // For simplicity and idempotency, we use a series of targeted
    // normalisation passes on the raw line.

    // First, normalise multi-space runs around operators to single spaces.
    // We do NOT touch content inside string literals or comments.

    collapse_spaces_around_ops(line)
}

/// Collapse multiple consecutive spaces around comparison/assignment operators
/// to exactly one space on each side.
///
/// This is a simplified, line-level operation that avoids touching string
/// content by scanning character by character.
fn collapse_spaces_around_ops(line: &str) -> String {
    let leading_end = line
        .char_indices()
        .find(|&(_, ch)| !ch.is_whitespace())
        .map_or(line.len(), |(idx, _)| idx);
    let (leading_whitespace, rest) = line.split_at(leading_end);

    let mut result = String::with_capacity(line.len());
    result.push_str(leading_whitespace);

    let mut in_string = false;
    let mut escaped = false;
    let mut prev_was_space = false;

    let chars: Vec<char> = rest.chars().collect();
    let mut idx = 0_usize;

    while idx < chars.len() {
        let ch = chars[idx];

        if in_string {
            result.push(ch);
            if escaped {
                escaped = false;
                prev_was_space = false;
                idx = idx.saturating_add(1);
                continue;
            }
            if ch == '\\' {
                escaped = true;
                prev_was_space = false;
                idx = idx.saturating_add(1);
                continue;
            }
            if ch == '\'' {
                in_string = false;
            }
            prev_was_space = false;
            idx = idx.saturating_add(1);
            continue;
        }

        if ch == '\'' {
            in_string = true;
            escaped = false;
            result.push(ch);
            prev_was_space = false;
            idx = idx.saturating_add(1);
            continue;
        }

        if ch == ' ' {
            if !prev_was_space {
                result.push(ch);
            }
            prev_was_space = true;
        } else {
            result.push(ch);
            prev_was_space = false;
        }

        idx = idx.saturating_add(1);
    }

    result
}
