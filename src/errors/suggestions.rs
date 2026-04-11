extern crate alloc;

use crate::type_system::errors::TypeError;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::min;

/// Maximum edit distance threshold for typo suggestions.
pub const SUGGESTION_DISTANCE_THRESHOLD: usize = 2;

/// Ranked suggestion candidate for unresolved identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierSuggestion {
    /// Misspelled identifier from source.
    pub input: String,
    /// Best-matching identifier in visible scope.
    pub suggestion: String,
    /// Edit distance between `input` and `suggestion`.
    pub distance: usize,
}

/// Return the best identifier suggestion within threshold, if any.
#[must_use]
pub fn closest_identifier_suggestion(
    input: &str,
    known_identifiers: &[String],
) -> Option<IdentifierSuggestion> {
    let mut best: Option<IdentifierSuggestion> = None;

    for candidate in known_identifiers {
        if candidate == input {
            continue;
        }

        let distance = levenshtein_distance(input, candidate.as_str());
        if distance > SUGGESTION_DISTANCE_THRESHOLD {
            continue;
        }

        let next = IdentifierSuggestion {
            input: input.to_owned(),
            suggestion: candidate.clone(),
            distance,
        };

        best = match best {
            Some(current)
                if current.distance < next.distance
                    || (current.distance == next.distance
                        && current.suggestion.as_str() <= next.suggestion.as_str()) =>
            {
                Some(current)
            }
            _ => Some(next),
        };
    }

    best
}

/// Return type-annotation guidance for generic inference failures.
#[must_use]
pub fn did_you_mean_type_annotation(error: &TypeError) -> Option<String> {
    match error {
        &TypeError::CannotInferGenericType { ref param_name, .. } => Some(alloc::format!(
            "Consider adding a type annotation or explicit generic argument for '{param_name}'."
        )),
        _ => None,
    }
}

/// Compute Levenshtein edit distance between two strings.
#[must_use]
pub fn levenshtein_distance(left: &str, right: &str) -> usize {
    if left == right {
        return 0;
    }

    let right_chars: Vec<char> = right.chars().collect();
    let right_len = right_chars.len();

    if right_len == 0 {
        return left.chars().count();
    }

    let left_len = left.chars().count();
    if left_len == 0 {
        return right_len;
    }

    let mut previous: Vec<usize> = (0..=right_len).collect();
    let current_len = right_len.saturating_add(1);
    let mut current: Vec<usize> = vec![0; current_len];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index.saturating_add(1);
        for (right_index, right_char) in right_chars.iter().copied().enumerate() {
            let cost = usize::from(left_char != right_char);
            let insertion = current[right_index].saturating_add(1);
            let right_plus_one = right_index.saturating_add(1);
            let deletion = previous[right_plus_one].saturating_add(1);
            let substitution = previous[right_index].saturating_add(cost);
            current[right_plus_one] = min(min(insertion, deletion), substitution);
        }
        previous.clone_from(&current);
    }

    previous[right_len]
}
