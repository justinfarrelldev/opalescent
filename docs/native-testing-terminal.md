# Production-binary terminal tests

Cases with `kind: end_to_end` cause the coordinator to build the normal application
once with the normal project compiler, in that run's private `application` output
directory. It is separate from cached instrumented workers. The testing library's
`application_executable(): ApplicationExecutable` exposes only that executable.
Worker processes receive the descriptor explicitly; ambient descriptor overrides
are cleared when no application was requested.

`standard.testing` provides the synchronous POSIX PTY driver:

- `terminal_process_launch(executable, argument, directory, columns, rows)` launches
  an application with one filename argument, working directory and PTY dimensions.
- `terminal_process_send_text(mutable ref process, text)` sends text once.
- `terminal_process_send_key(mutable ref process, name)` sends a named key:
  `Enter`, `Escape`, `Backspace`, `Tab`, arrows, `Home`, `End` or `Delete`.
- `terminal_process_row(ref process, zero_based_row)` pumps output and returns a
  normalized UTF-8 row, without replaying input. Common VT cursor movement, screen
  and line clearing, UTF-8, display-cell width and styling sequences are handled.
- `terminal_process_cursor_line`/`terminal_process_cursor_column` observe the cursor.
- `terminal_process_exited` and `terminal_process_exit_code` observe/reap the child.
- `terminal_process_transcript` returns captured terminal text/control sequences.
- `terminal_process_stop(mutable ref process)` explicitly terminates (if needed),
  waits, closes transport and consumes the handle. Declare it in World cleanup.

Launch/send/stop failures use ordinary application errors. The descriptor is
available to E2E cases; terminal dimensions must be between 1 and 512. POSIX PTYs
are currently implemented; unsupported platforms return a launch error, not a
simulated terminal. No automatic orphan-process recovery or resource registry is
installed. A hard worker crash/timeout is not a promise of external cleanup.

The production child clears `OPAL_TERMINAL_FAKE_BACKEND`, `OPAL_TERMINAL_FAKE_EVENTS`
and `OPAL_TERMINAL_FAKE_PAUSE_EVENTS`. The editor fixture waits for normal/insert
mode, sends each action once, enters `wq`, waits for exit and checks exact file
contents. It does not require a final post-quit frame. Its after-hook captures the
transcript even when a body assertion aborts.
