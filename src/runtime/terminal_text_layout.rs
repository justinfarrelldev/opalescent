#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::missing_const_for_fn,
    clippy::missing_docs_in_private_items,
    clippy::string_slice,
    clippy::too_many_lines,
    reason = "Unicode range arithmetic and boundary slicing are centralized in this layout helper"
)]

extern crate alloc;

use crate::runtime::errors::{RuntimeError, RuntimeResult};
use crate::runtime::memory::{OpalString, RuntimeAllocator};

/// Measure the terminal display-cell width of UTF-8 text.
#[must_use]
pub fn terminal_text_cell_width(value: &OpalString) -> i64 {
    text_cell_width(value.as_str())
}

/// Clip UTF-8 text at a grapheme boundary that fits within `max_cells`.
///
/// # Errors
///
/// Returns `TerminalTextLayoutError` for negative limits and allocator errors
/// when the clipped text cannot be allocated.
pub fn terminal_text_clip_to_cells<Allocator>(
    allocator: &Allocator,
    value: &OpalString,
    max_cells: i64,
) -> RuntimeResult<(OpalString, i64)>
where
    Allocator: RuntimeAllocator,
{
    if max_cells < 0 {
        return Err(RuntimeError::user_error(1_201, "TerminalTextLayoutError"));
    }

    let source = value.as_str();
    let mut used_cells = 0_i64;
    let mut end_byte = 0_usize;
    let mut cursor = 0_usize;
    while cursor < source.len() {
        let next = next_grapheme_end(source, cursor);
        let width = grapheme_width(&source[cursor..next]);
        if used_cells.saturating_add(width) > max_cells {
            break;
        }
        used_cells = used_cells.saturating_add(width);
        end_byte = next;
        cursor = next;
    }

    allocator
        .allocate_string(&source[..end_byte])
        .map(|text| (text, used_cells))
}

#[must_use]
fn text_cell_width(text: &str) -> i64 {
    let mut width = 0_i64;
    let mut cursor = 0_usize;
    while cursor < text.len() {
        let next = next_grapheme_end(text, cursor);
        width = width.saturating_add(grapheme_width(&text[cursor..next]));
        cursor = next;
    }
    width
}

#[must_use]
fn next_grapheme_end(text: &str, start: usize) -> usize {
    let mut end = next_scalar_end(text, start);
    let mut join_next = false;
    while end < text.len() {
        let Some(next_char) = text[end..].chars().next() else {
            break;
        };
        if is_grapheme_extend(next_char) {
            end = next_scalar_end(text, end);
            continue;
        }
        if next_char == '\u{200D}' {
            end = next_scalar_end(text, end);
            join_next = true;
            continue;
        }
        if join_next {
            end = next_scalar_end(text, end);
            join_next = false;
            continue;
        }
        break;
    }
    end
}

#[must_use]
fn next_scalar_end(text: &str, start: usize) -> usize {
    let mut iterator = text[start..].char_indices();
    let _current = iterator.next();
    iterator
        .next()
        .map_or(text.len(), |(offset, _)| start.saturating_add(offset))
}

#[must_use]
fn grapheme_width(cluster: &str) -> i64 {
    if cluster.chars().any(|value| value == '\u{200D}') && cluster.chars().any(is_wide_emoji_scalar)
    {
        return 2;
    }
    cluster.chars().map(scalar_width).sum()
}

#[must_use]
fn scalar_width(value: char) -> i64 {
    if value == '\u{200D}' || is_grapheme_extend(value) || value.is_control() {
        0
    } else if is_wide_scalar(value) {
        2
    } else {
        1
    }
}

#[must_use]
const fn is_grapheme_extend(value: char) -> bool {
    let scalar = value as u32;
    matches!(
        scalar,
        0x0300..=0x036F
            | 0x0483..=0x0489
            | 0x0591..=0x05BD
            | 0x05BF
            | 0x05C1..=0x05C2
            | 0x05C4..=0x05C5
            | 0x05C7
            | 0x0610..=0x061A
            | 0x064B..=0x065F
            | 0x0670
            | 0x06D6..=0x06DC
            | 0x06DF..=0x06E4
            | 0x06E7..=0x06E8
            | 0x06EA..=0x06ED
            | 0x0711
            | 0x0730..=0x074A
            | 0x07A6..=0x07B0
            | 0x07EB..=0x07F3
            | 0x0816..=0x0819
            | 0x081B..=0x0823
            | 0x0825..=0x0827
            | 0x0829..=0x082D
            | 0x0859..=0x085B
            | 0x08D3..=0x08E1
            | 0x08E3..=0x0903
            | 0x093A
            | 0x093C
            | 0x0941..=0x0948
            | 0x094D
            | 0x0951..=0x0957
            | 0x0962..=0x0963
            | 0x0981
            | 0x09BC
            | 0x09C1..=0x09C4
            | 0x09CD
            | 0x09E2..=0x09E3
            | 0x0A01..=0x0A02
            | 0x0A3C
            | 0x0A41..=0x0A42
            | 0x0A47..=0x0A48
            | 0x0A4B..=0x0A4D
            | 0x0A51
            | 0x0A70..=0x0A71
            | 0x0A75
            | 0x0A81..=0x0A82
            | 0x0ABC
            | 0x0AC1..=0x0AC5
            | 0x0AC7..=0x0AC8
            | 0x0ACD
            | 0x0AE2..=0x0AE3
            | 0x0B01
            | 0x0B3C
            | 0x0B3F
            | 0x0B41..=0x0B44
            | 0x0B4D
            | 0x0B56
            | 0x0B62..=0x0B63
            | 0x0B82
            | 0x0BC0
            | 0x0BCD
            | 0x0C00
            | 0x0C04
            | 0x0C3E..=0x0C40
            | 0x0C46..=0x0C48
            | 0x0C4A..=0x0C4D
            | 0x0C55..=0x0C56
            | 0x0C62..=0x0C63
            | 0x0C81
            | 0x0CBC
            | 0x0CBF
            | 0x0CC6
            | 0x0CCC..=0x0CCD
            | 0x0CE2..=0x0CE3
            | 0x0D00..=0x0D01
            | 0x0D3B..=0x0D3C
            | 0x0D41..=0x0D44
            | 0x0D4D
            | 0x0D62..=0x0D63
            | 0x0DCA
            | 0x0DD2..=0x0DD4
            | 0x0DD6
            | 0x0E31
            | 0x0E34..=0x0E3A
            | 0x0E47..=0x0E4E
            | 0x0EB1
            | 0x0EB4..=0x0EBC
            | 0x0EC8..=0x0ECD
            | 0x0F18..=0x0F19
            | 0x0F35
            | 0x0F37
            | 0x0F39
            | 0x0F71..=0x0F7E
            | 0x0F80..=0x0F84
            | 0x0F86..=0x0F87
            | 0x0F8D..=0x0F97
            | 0x0F99..=0x0FBC
            | 0x0FC6
            | 0xFE00..=0xFE0F
            | 0x1AB0..=0x1AFF
            | 0x1DC0..=0x1DFF
            | 0x20D0..=0x20FF
            | 0xE0100..=0xE01EF
    ) || is_emoji_modifier(value)
}

#[must_use]
const fn is_wide_scalar(value: char) -> bool {
    let scalar = value as u32;
    matches!(
        scalar,
        0x1100..=0x115F
            | 0x231A..=0x231B
            | 0x2329..=0x232A
            | 0x23E9..=0x23EC
            | 0x23F0
            | 0x23F3
            | 0x25FD..=0x25FE
            | 0x2614..=0x2615
            | 0x2648..=0x2653
            | 0x267F
            | 0x2693
            | 0x26A1
            | 0x26AA..=0x26AB
            | 0x26BD..=0x26BE
            | 0x26C4..=0x26C5
            | 0x26CE
            | 0x26D4
            | 0x26EA
            | 0x26F2..=0x26F3
            | 0x26F5
            | 0x26FA
            | 0x26FD
            | 0x2705
            | 0x270A..=0x270B
            | 0x2728
            | 0x274C
            | 0x274E
            | 0x2753..=0x2755
            | 0x2757
            | 0x2795..=0x2797
            | 0x27B0
            | 0x27BF
            | 0x2B1B..=0x2B1C
            | 0x2B50
            | 0x2B55
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F000..=0x1FAFF
            | 0x20000..=0x3FFFD
    )
}

#[must_use]
const fn is_wide_emoji_scalar(value: char) -> bool {
    let scalar = value as u32;
    matches!(scalar, 0x2600..=0x27BF | 0x1F000..=0x1FAFF)
}

#[must_use]
const fn is_emoji_modifier(value: char) -> bool {
    let scalar = value as u32;
    matches!(scalar, 0x1F3FB..=0x1F3FF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::memory::DefaultRuntimeAllocator;

    #[test]
    fn terminal_text_layout_measures_and_clips_grapheme_cells() {
        let allocator = DefaultRuntimeAllocator;
        let text = allocator
            .allocate_string("a界e\u{301}🙂👩\u{200D}💻")
            .expect("test string allocation should succeed");
        assert_eq!(terminal_text_cell_width(&text), 8);

        let clipped = terminal_text_clip_to_cells(&allocator, &text, 4)
            .expect("clip should succeed for positive limit");
        assert_eq!(clipped.0.as_str(), "a界e\u{301}");
        assert_eq!(clipped.1, 4);
    }
}
