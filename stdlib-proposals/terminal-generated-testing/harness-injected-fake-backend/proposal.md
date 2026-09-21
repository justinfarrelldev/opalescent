# Harness-Injected Fake Backend

## Overview
Generated terminal tests should use ordinary production terminal-session source while the integration harness injects a deterministic fake backend out-of-band. The source imports no test-only scripting API, but `terminal_session_open_sync` binds to a fake session when the harness sets the authorized fake-backend environment.

This is the selected v1 for generated terminal fixture execution.

## Assumes
- Generated C/runtime lowering exists for terminal session open, read, trusted write, flush, close, and selected rendering helpers.
- The runtime can detect harness authorization (`OPAL_TERMINAL_FAKE_BACKEND=1`) and scripted events (`OPAL_TERMINAL_FAKE_EVENTS`).
- Production builds continue rejecting `standard.testing.terminal` imports.

## Syntax Design
No Opalescent syntax is added. Test metadata lives in the Rust integration harness or project runner rather than `.op` source.

```text
OPAL_TERMINAL_FAKE_BACKEND=1
OPAL_TERMINAL_FAKE_EVENTS=text:hello|key:Enter
```

## Example Applications
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
- Keeps sealed test authority and fake backend construction out of production source.
- Makes generated terminal fixtures executable with deterministic scripted input.
- Reuses the same terminal-session API that production code calls.

## Weaknesses
- Less ergonomic than source-level test scripting.
- Requires runner/runtime coordination and environment hygiene.
- v1 only covers the generated-runtime subset currently lowered.

## Impact on Existing Syntax
No syntax changes. Production generated terminal opening remains gated unless the harness explicitly injects the fake backend.

## Interactions with Other Concerns
- Complements `terminal-session-input/typed-event-session` by exercising session read/write lifecycles.
- Supports `terminal-session-rendering/high-level-session-rendering` fixtures for clear/move/draw/bell output.
- Keeps `standard.testing.terminal` as a future source-level testing surface once generated test-only imports are fully enforced.

## Implementation Difficulty
Medium. The core work is deterministic event parsing, trusted output capture, and lifecycle/error assertions in generated binaries.

## Must NOT Have
- No production import path for fake backend construction.
- No raw string terminal writes that bypass trusted output APIs.
- No test that claims terminal coverage while only compiling static source.
