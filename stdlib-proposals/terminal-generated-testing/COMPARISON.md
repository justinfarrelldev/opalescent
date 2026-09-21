# Generated Terminal Testing Comparison

## Status and scope

This concern is a companion to `terminal-session-input`. It specifies how generated Opalescent test projects can run terminal-session programs deterministically with scripted input and asserted output, without exposing test-only terminal authority to production builds.

## Comparison matrix

| Axis | Harness-injected fake backend — recommended v1 | Public test-only scripting API | Environment transcript files |
|---|---:|---:|---:|
| **Ergonomics** | ★★★★☆ | ★★★★★ | ★★★☆☆ |
| **Error-model fit** | ★★★★★ | ★★★★☆ | ★★★☆☆ |
| **Opalescent-idiom fit** | ★★★★★ | ★★★★☆ | ★★★☆☆ |
| **Implementation effort** | Medium (2-3mo) | High (3-5mo) | Low (1-2mo) |
| **Extensibility** | ★★★★☆ | ★★★★★ | ★★☆☆☆ |
| **Async readiness** | ★★★★☆ | ★★★★☆ | ★★☆☆☆ |

## Analysis

### Harness-injected fake backend — recommended v1
- Keeps sealed test authority outside production source.
- Lets existing terminal fixtures become real compile/run tests.
- Requires integration-runner support but avoids public scripting leakage.

### Public test-only scripting API
- Most ergonomic for writing tests in Opalescent source.
- Requires strict test-only import availability and generated-code leakage checks.

### Environment transcript files
- Easy to prototype but too stringly and weak for typed terminal events.

## Selection

Select **harness-injected fake backend** first. Add `standard.testing.terminal` source-level scripting only after test-only availability is enforced in generated builds.

## Required red fixtures

- Activate the existing 14 terminal fixtures as compile/run tests.
- Add `terminal-simple-editor-edit-save` once terminal generated-code support lands.
- Compile-fail: production builds cannot import `standard.testing.terminal`.
