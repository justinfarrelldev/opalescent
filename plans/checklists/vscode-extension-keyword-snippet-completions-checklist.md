# VS Code Keyword and Snippet Completions Checklist

- [x] Confirm language docs/context are already reviewed for valid syntax and no match snippets.
- [x] Red: add failing tests for keyword completions and common structure snippets.
- [x] Red: add failing tests for primitive type and standard-library completion documentation.
- [x] Green: add static keyword, snippet, primitive type, and standard-library completion metadata.
- [x] Green: make the VS Code provider insert snippets with tab stops.
- [x] Refactor/review completion sorting, labels, and docs.
- [x] Run VS Code extension lint and tests.
- [x] Commit atomically without staging unrelated extension formatting changes.
