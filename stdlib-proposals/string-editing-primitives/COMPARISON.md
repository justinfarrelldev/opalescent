# String Editing Primitives Comparison

## Status and scope

This concern covers pure string editing helpers needed by text editors and other line-oriented tools. Existing public string helpers expose Unicode-scalar indexing through `.length`, `.at`, `string_extract_range`, `string_take_prefix`, and `string_take_suffix`; this proposal set builds on that model.

## Comparison matrix

| Axis | Scalar range functions — recommended v1 | Grapheme-aware functions | Line-buffer object |
|---|---:|---:|---:|
| **Ergonomics** | ★★★★☆ | ★★★★☆ | ★★★★★ |
| **Error-model fit** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Implementation effort** | Low (1-2mo) | High (4-6mo) | Medium (3-4mo) |
| **Extensibility** | ★★★★☆ | ★★★★★ | ★★★★☆ |
| **Async readiness** | ★★★★★ | ★★★★★ | ★★★★☆ |

## Analysis

### Scalar range functions — recommended v1
- Aligns with current string scalar semantics.
- Gives editors immediate insert/delete/replace operations without introducing grapheme policy.
- Must document that scalar indexes are not terminal cells or grapheme clusters.

### Grapheme-aware functions
- Required for general Unicode editor correctness.
- More expensive to specify and implement because Unicode segmentation and versioning become public compatibility contracts.

### Line-buffer object
- Most ergonomic for an editor, but introduces a new stateful text abstraction before the primitive operations are settled.

## Selection

Select **scalar range functions** for v1, and keep grapheme-aware functions as the required follow-up before advertising full Unicode editing correctness.

## Required red fixtures

- `string-edit-insert-delete-ascii`
- `string-edit-split-join-line`
- `string-edit-range-errors`
- `string-edit-allocation-error-contract`
