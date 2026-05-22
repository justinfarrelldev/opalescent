# Game of Life Project Post-Mortem

This note is a developer-experience post-mortem for creating
`test-projects/game-of-life-full`, an 80x40 Conway's Game of Life program in
Opalescent with a flat `int8[]` board, multiple modules, a `.types.op` file,
terminal rendering, and a 15fps infinite loop.

## Overall Experience

The project was absolutely possible in current Opalescent, but it was not a
smooth path the whole way. The language and runtime had enough pieces to build a
real terminal animation: arrays, mutable locals, `while` loops, local imports,
`.types.op` imports, fallible terminal writes, and `FrameClock` pacing all worked.
The hardest parts were not the Game of Life rules themselves. The hard parts
were around module-boundary type behavior, terminal semantics, and knowing which
standard APIs were safe for an infinite application.

The final shape feels like a useful instructional project. It exercises several
real language surfaces without requiring compiler changes for the app itself. It
also exposed a genuine runtime behavior issue in terminal clearing.

## What Worked Well

The flat-array model was straightforward and fit the current language well. The
core formula is simple:

```text
index = y * width + x
```

Using `int8[]` for the board was a good match. It avoided nested-array questions,
kept memory small, and made the rules module easy to reason about. Building the
next generation into a fresh array also matched Game of Life semantics cleanly:
read the current generation, produce the next generation, then replace the board.

Mutable locals and `while` loops were fine. The nested loops in `patterns.op`,
`rules.op`, and `render.op` all compiled without drama once the syntax was kept
close to existing fixtures.

The `FrameClock` API worked well. Creating it in `main` with:

```opalescent
let clock = propagate new FrameClock:
    frames_per_second: config.frames_per_second
```

made the pacing code very small. It also fit Opalescent's error style: the
constructor can fail, and `main` owns that propagated error.

The terminal writer APIs were usable for streaming output. Writing each cell with
`writer_write_sync` is intentionally simple and allocation-light. It is not the
most efficient terminal renderer imaginable, but for 3200 cells per frame at
15fps it is a safe and understandable implementation.

## What Was Weird Or Required Workarounds

The `.types.op` experiment partially worked, but not in the first shape I tried.
The original plan was to expose config helpers such as:

```opalescent
public let default_config = f(): LifeConfig => ...
public let board_cell_count = f(config: LifeConfig): int64 => ...
```

with `LifeConfig` imported from `./life.types` inside `config.op`. That did not
work. The compiler reported `Type 'LifeConfig' not found` while checking the
public top-level lambda signatures.

The underlying behavior appears to be that top-level `let ... = f(...)`
signatures in value modules are still mapped through the built-in type mapper
before imported nominal types are available. Existing direct type imports in
`main.op` work, and imported nominal error types in other contexts can work, but
this specific public-lambda-signature path did not.

The workaround was to keep `LifeConfig` real and useful in `main.op`, while
making shared module APIs accept primitive values:

```opalescent
public let board_cell_count = f(width: int64, height: int64): int64 => ...
```

That is valid, but it weakens the instructional ideal. A learner might expect a
configuration type to move cleanly across modules. In this project, the types
file demonstrates the feature, but the API design has to bend around a current
compiler limitation.

Another weird point was indentation sensitivity after editing. One failed build
came from statements being accidentally nested under a `new LifeConfig:` block.
The compiler reported a cluster of parse errors rather than one very direct
"this statement is still inside the object initializer" message. The fix was
simple, but the diagnosis required reading the file carefully.

## Terminal Clearing Surprise

The first renderer cleared once and then moved the cursor home each frame. That
was a reasonable animation strategy, but it was not what the user asked for after
trying the program. The next version called `terminal_clear_screen_on_sync` each
frame. That still did not fully clear in VS Code's integrated terminal because
the runtime helper emitted:

```text
ESC[2J ESC[H
```

That clears the visible screen and moves the cursor home, but it does not clear
scrollback in many terminals. The result is that old generations can still appear
when scrolling or selecting terminal output, making it look like the app is not
really clearing.

The runtime fix was to make `terminal_clear_screen_on_sync` emit:

```text
ESC[2J ESC[3J ESC[H
```

This clears the visible screen, clears scrollback, and returns the cursor home.
The existing exact-byte integration test was updated to match. This was the one
place where the app revealed a runtime-level behavior issue rather than an app
bug.

## Standard Library And Runtime Edges

The terminal API is useful, but the distinction between "clear visible screen"
and "clear visible screen plus scrollback" matters for terminal apps. The name
`terminal_clear_screen_on_sync` sounds like the stronger user-visible behavior,
so updating the runtime was better than hand-writing escape sequences in the
Game of Life project.

The ANSI support check is conservative: redirected stdout is not treated as ANSI.
That is good. It lets the app write transcript-style frames when output is
captured instead of filling files with escape sequences.

I also had to be careful around string construction. The earlier research showed
that repeatedly constructing per-frame strings or builders would be a poor fit
for an infinite program. Streaming directly through `StdoutWriter` was the safer
choice.

## What Was Too Wordy

The documentation pressure is real. Public functions require doc comments, and
for a teaching project that is mostly good, but several small helpers become
noisy. Functions like `alive_cell`, `dead_cell`, `board_width`, and simple seed
predicates need comments that are longer than the code they explain. That makes
some modules feel heavier than the logic warrants.

The seed-pattern predicates are also verbose. Opalescent currently makes the
explicit coordinate checks easy to read, but a table-driven representation would
be more compact if the language had a more ergonomic way to express small
coordinate lists and iterate them without adding more machinery than the example
needs.

The README also had to explain a compiler limitation around imported nominal
types. That explanation is useful, but it makes the project feel less clean than
the conceptual design. Ideally, `LifeConfig` would appear naturally in shared
module signatures and the README would not need a caveat.

## What Felt About Right

The board module is a good size. `board_index`, `in_bounds`, `cell_at`, and
`is_alive_at` form a clear instructional layer without too much ceremony.

The rules module is also about right. Neighbor counting and next-cell state are
plain enough that a reader can map them directly to Conway's rules. The fresh
`next_board` allocation is explicit and teaches the important rule that a
generation should not be mutated while it is being read.

The final `main.op` is clean. It has the configuration value, the clock, the
mutable board, the generation counter, and the infinite loop. That is exactly the
right amount of orchestration for this kind of app.

## Things I Had To Fight

The biggest fight was the imported type in public function signatures. The first
version of `config.op` looked like the design I wanted, but it did not compile.
Switching between normal and explicit `import type` did not fix it. The working
solution was an API reshape, not a syntax tweak.

The second fight was terminal behavior. The Opalescent program was calling a
reasonable API, but the runtime's clear sequence was not strong enough for the
observed terminal. The correct fix belonged in the runtime helper and its exact
byte test.

The third fight was process discipline around previews. I initially used an
external preview file under `/tmp`. That was technically convenient, but it was
not aligned with the requested repository-local workflow. Future preview commands
for this project should either print directly to the terminal under `timeout` or
write to an ignored path inside the repository.

## Suggested Follow-Ups

The compiler should eventually allow imported nominal `.types.op` types in
module-level public lambda signatures. That would let this project use
`LifeConfig` consistently across `config.op`, `board.op`, `rules.op`, and
`render.op` without primitive parameter threading.

The terminal standard library tests should continue to assert exact bytes for
clear and cursor operations. That caught the expected behavior precisely once the
runtime was updated.

It may be worth adding a tiny terminal-app guide to the repository documentation:
use `FrameClock`, stream output, avoid per-frame builders in infinite loops, use
ANSI support checks, and always run infinite examples with external timeouts.

A future language feature for compact data literals or tuple-like coordinate
pairs would make pattern seeding much nicer. The current code is clear, but it is
longer than the concept.

## Final Assessment

The project is a good stress test for Opalescent as a real application language.
The basics held: module loading, arrays, loops, fallible operations, native
runtime calls, and project builds all came together. The sharp edges were
specific and actionable rather than fundamental blockers.

The most important lesson is that Opalescent can already build this kind of
terminal program, but instructional examples should be honest about current
module/type limitations and terminal-runtime behavior. The final project is valid
and useful, and the process exposed a couple of improvements that would make the
next project feel noticeably smoother.
