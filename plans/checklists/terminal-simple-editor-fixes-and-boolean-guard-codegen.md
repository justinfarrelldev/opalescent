# Terminal simple editor fixes and boolean guard codegen

- [x] Red: add/reproduce boolean expression-guard fallback codegen failure for fallible boolean calls.
- [x] Green: fix boolean expression-guard codegen so fallback booleans use the same lowered representation as success booleans.
- [x] Red: reproduce terminal-simple-editor missing explicit file crash.
- [x] Red: reproduce terminal-simple-editor save failure abort.
- [x] Red: reproduce terminal-simple-editor empty command invalid path payload.
- [x] Red: reproduce terminal-simple-editor prior app-side input-limit stale running termination.
- [x] Red: reproduce terminal-simple-editor long-line cursor beyond rendered width.
- [x] Red: require production editor source to contain no test-harness hooks.
- [x] Green: handle missing explicit files as new empty buffers.
- [x] Green: handle save failures with mutable result plus continue.
- [x] Green: store command text explicitly and parse on Enter.
- [x] Green: remove app-side input limit and test summaries.
- [x] Green: clamp cursor screen column to rendered width.
- [x] Green: handle bell failures locally where current language allows.
- [x] Refactor/review changed Opalescent and compiler code.
- [x] Verify targeted compiler/codegen and terminal-simple-editor scenarios.
