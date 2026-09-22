# Simple Neovim ADT layout manifest refactor checklist

- [x] Create the refactor plan.
- [x] Add red integration coverage for the ADT-oriented editor architecture.
- [x] Confirm the new targeted test fails before implementation.
- [x] Add public shared editor ADTs in `editor.types.op`.
- [x] Add structured state/transition helpers using imported ADT manifests.
- [x] Refactor labels, rendering, input handling, and main loop to use nominal ADTs/products.
- [x] Remove the legacy integer-constant state model.
- [x] Confirm targeted editor integration coverage passes.
- [x] Build/run `terminal-simple-editor` with fake terminal events.
- [x] Run focused module-interface/editor regression tests.
- [x] Run full validation (`cargo test -q`, targeted integration, lint/build/line-count checks).
- [x] Perform final code review and update this checklist.
