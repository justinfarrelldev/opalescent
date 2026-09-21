# Collections Editing Comparison

## Status and scope

This concern covers public array insertion and removal helpers for editor-style line buffers. Current docs expose `.push`, `.pop`, `.at`, indexed assignment, and higher-order helpers, but not stable public insert/remove-at signatures.

## Comparison matrix

| Axis | Method-style array editing — recommended long-term | Free-function array editing — recommended first implementation |
|---|---:|---:|
| **Ergonomics** | ★★★★★ | ★★★★☆ |
| **Error-model fit** | ★★★★★ | ★★★★★ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ |
| **Implementation effort** | Medium (2-3mo) | Low (1-2mo) |
| **Extensibility** | ★★★★☆ | ★★★★☆ |
| **Async readiness** | ★★★★★ | ★★★★★ |

## Analysis

### Method-style array editing — recommended long-term
- Best matches existing `.push`, `.pop`, and `.at` usage.
- Requires method registration and documentation consistency.

### Free-function array editing — recommended first implementation
- Simpler to expose in the module resolver and generated-code lowering.
- Clear generic signatures and explicit error behavior.
- Can later coexist with method aliases.

## Selection

Implement **free functions first** for a stable generated-code surface, then add method aliases once method-style collection APIs are consistently documented.

## Required red fixtures

- `array-insert-remove-lines`
- `array-insert-remove-errors`
- `editor-line-split-uses-array-insert`
