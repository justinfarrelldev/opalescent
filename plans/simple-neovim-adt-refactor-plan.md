# Simple Neovim ADT Layout Manifest Refactor Plan

Branch: `simple-neovim-clone`

## Goal

Refactor `test-projects/terminal-simple-editor` to exercise module-interface ADT layout manifests in a realistic multi-module application while preserving a successful generated build/run.

## Scope

- Replace integer-constant editor modes, commands, statuses, and termination reasons with public nominal ADTs declared in a `.types.op` module.
- Introduce product ADTs for cursor/state/transition values so editor modules pass structured state across module boundaries instead of long multi-return tuples.
- Keep the user-visible editor behavior unchanged for the deterministic fake-terminal save-and-quit flow.
- Add integration coverage that both checks the ADT-oriented source shape and builds/runs the editor with the fake terminal backend.

## Red Phase

1. Add an integration test for `terminal-simple-editor` that requires:
   - a public shared `editor.types.op` state model,
   - removal of the legacy `constants.op` integer-state model,
   - successful project compile/run under fake terminal events.
2. Run the new targeted test and confirm it fails before the refactor.

## Green Phase

1. Add `src/editor.types.op` with public ADTs:
   - `EditorMode`, `EditorCommand`, `EditorStatus`, `EditorTermination`,
   - `CursorPosition`, `EditorState`, and `EditorTransition`.
2. Add state construction/update helpers that construct imported products and variants from a non-types module.
3. Refactor labels/render/input/main to consume and refine imported ADTs, including payload variants.
4. Remove `constants.op` once no longer used.
5. Run the targeted integration test until it passes.

## Refactor/Review Phase

1. Simplify repeated state-transition code where maintainable without hiding manifest stress coverage.
2. Build the Opalescent project directly and run the fake-terminal scenario.
3. Run focused and full repository validation.
4. Review the resulting Opalescent code for maintainability, manifest coverage, and unchanged editor behavior.
