# Terminal Text Layout Comparison

## Status and scope

This concern covers terminal display-cell measurement and clipping. It exists because terminal cursor positions are cells, while current strings are indexed by Unicode scalar values. A simple editor may intentionally start with ASCII/single-cell text, but general terminal editor correctness requires Unicode grapheme and cell-width helpers.

## Comparison matrix

| Axis | ASCII/single-cell baseline — permitted v1 limit | Unicode scalar width | Grapheme cell width — future target |
|---|---:|---:|---:|
| **Ergonomics** | ★★★☆☆ | ★★★☆☆ | ★★★★☆ |
| **Error-model fit** | ★★★★★ | ★★★★☆ | ★★★★☆ |
| **Opalescent-idiom fit** | ★★★★☆ | ★★★★☆ | ★★★★★ |
| **Implementation effort** | Low (1mo) | Medium (2-3mo) | High (4-6mo) |
| **Extensibility** | ★★☆☆☆ | ★★★☆☆ | ★★★★★ |
| **Async readiness** | ★★★★★ | ★★★★★ | ★★★★★ |

## Analysis

### ASCII/single-cell baseline — permitted v1 limit
- Smallest path for a first editor fixture.
- Must reject or escape non-ASCII input and document the limitation.

### Unicode scalar width
- Better than ASCII for some characters but still wrong for combining marks and many emoji sequences.
- Risks giving users false confidence.

### Grapheme cell width — future target
- Correct target for terminal UI layout.
- Requires Unicode segmentation and wcwidth/East Asian Width policy.

## Selection

Permit an **ASCII/single-cell baseline** for the first editor only if tests enforce that non-ASCII input is rejected or escaped. Draft and later implement **grapheme cell width** before claiming Unicode-correct terminal editing.

## Required red fixtures

- `terminal-layout-ascii-clip`
- `terminal-layout-wide-char-known-limitation` or `terminal-layout-wide-char-width`
- `terminal-editor-ascii-mode-rejects-or-escapes-wide-input`
