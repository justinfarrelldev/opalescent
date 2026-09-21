# Deterministic Fixture Runner

## Overview
This proposal defines deterministic generated-code test execution for terminal-session programs. The integration runner injects a fake terminal backend with scripted input events, captures trusted output, verifies close/restore behavior, and checks stdout/stderr/exit expectations after the session is closed.

It is a companion proposal to the selected terminal-session/input design, not a separate production API.

## Assumes
- The selected terminal session API has generated-code lowering for open/read/write/flush/close.
- Fake backend support exists in the runtime or harness.
- Test-only availability can distinguish integration fixtures from production builds.

## Syntax Design
No new language syntax is introduced for v1. Fixture metadata belongs to the Rust integration runner or test manifest, not production `.op` source.

```text
terminal fixture:
  script: key ArrowDown, text '!', key CtrlS, key CtrlQ
  expect_saved_file: fixtures/input.txt = 'alpha\nbeta!\n'
  expect_summary: EDITOR_SUMMARY opened=true saved=true dirty=false
```

## Example Applications
A generated editor fixture can use ordinary terminal APIs. The harness, not the source file, supplies deterministic events.

```opal
import terminal_session_open_sync, terminal_session_read_event_sync from standard
import terminal_session_close_sync from standard

entry main = f(args: string[]): void errors TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionRestoreError =>
    let options = terminal_session_options_default()
    let mutable session = propagate terminal_session_open_sync(options)
    let event = propagate terminal_session_read_event_sync(mutable ref session)
    propagate terminal_session_close_sync(mutable ref session)
    print('TERMINAL_FIXTURE_DONE')
    return void
```

## Strengths
- Production source cannot forge terminal events or sealed values.
- Makes existing terminal fixtures executable rather than static-only.
- Captures session output and restoration in one deterministic place.

## Weaknesses
- Requires integration harness work outside stdlib declarations.
- Less convenient than writing scripts directly in Opalescent test source.
- Needs clear fixture metadata format and diagnostics.

## Impact on Existing Syntax
No syntax changes. The compiler/test runner must reject production imports of test-only terminal modules and allow runner-authorized fake backend injection.

## Interactions with Other Concerns
- Extends `terminal-session-input/TESTING.md` rather than replacing it.
- Depends on terminal generated C ABI and ADT lowering.
- Enables editor fixtures that also use terminal rendering, string editing, array editing, and text layout proposals.

## Implementation Difficulty
Medium. The main work is runner/runtime plumbing and fixture activation.

## Must NOT Have
- No production import leakage from `standard.testing.terminal`.
- No raw terminal output bypass in tests.
- No static-only fixture claiming runtime coverage.
