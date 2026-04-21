# Global Logger Module

## Overview
This alternative adds a module-level logger with straightforward calls: `log_debug`, `log_info`, `log_warn`, and `log_error`. Application code imports logging once, configures destination and level at startup, then logs from anywhere without passing a handle.

The design targets command-line tools and service entrypoints where low ceremony is preferred. Buffered writes keep hot paths cheap, while `flush_sync` provides an explicit sync durability boundary for process shutdown, panic reporting, and batch checkpoints.

## Assumes
- Standard library can keep module-local mutable state for logger configuration and in-memory buffers.
- Error values remain explicit through `errors` clauses with `guard` and `propagate`.
- No deferred surface is added in this proposal.
- Log message formatting is string-based, with optional key-value fields.

## Syntax Design
```opal
import LogLevel, LogField, LoggerInitError, LoggerNotInitializedError, LogFlushError from ./logging.types

##
Description: Configures the global logger destination and minimum severity.
##
let configure_global_logger_sync = f(log_path: string, minimum_level: LogLevel): void errors LoggerInitError =>
    return void

##
Description: Adds a debug-level message to the global logger buffer.
##
let log_debug = f(message: string, fields: LogField[]): void errors LoggerNotInitializedError =>
    return void

##
Description: Adds an info-level message to the global logger buffer.
##
let log_info = f(message: string, fields: LogField[]): void errors LoggerNotInitializedError =>
    return void

##
Description: Adds a warning-level message to the global logger buffer.
##
let log_warn = f(message: string, fields: LogField[]): void errors LoggerNotInitializedError =>
    return void

##
Description: Adds an error-level message to the global logger buffer.
##
let log_error = f(message: string, fields: LogField[]): void errors LoggerNotInitializedError =>
    return void

##
Description: Flushes all buffered global logger output to stable storage.
##
let flush_sync = f(): void errors LoggerNotInitializedError, LogFlushError =>
    return void
```

## Example Applications
```opal
import LogLevel, LogField, LoggerInitError, LoggerNotInitializedError, LogFlushError from ./logging.types
import configure_global_logger_sync, log_debug, log_info, log_warn, log_error, flush_sync from ./global_logger

##
Description: Starts a service with global logger setup and request logging.
##
let run_service_sync = f(config_path: string): void errors LoggerInitError, LoggerNotInitializedError, LogFlushError =>
    # Configure destination and minimum level during boot.
    propagate configure_global_logger_sync('./var/service.log', LogLevel.Info)

    # Record successful boot metadata.
    propagate log_info('service started', [new LogField:
        key: 'config_path'
        value: config_path
    ])

    # Record lower-severity diagnostics for local troubleshooting.
    propagate log_debug('warming cache', [new LogField:
        key: 'phase'
        value: 'startup'
    ])

    # Record unusual but recoverable state transitions.
    propagate log_warn('cache miss ratio elevated', [new LogField:
        key: 'region'
        value: 'eu-west'
    ])

    # Record hard failures with machine-filterable context.
    propagate log_error('database unavailable', [new LogField:
        key: 'dependency'
        value: 'orders-db'
    ])

    # Force durability before exiting this lifecycle stage.
    propagate flush_sync()
    return void
```

## Strengths
- **Lowest ceremony** for broad application logging adoption.
- **Clear sync durability boundary** through explicit `flush_sync`.
- **Simple migration path** from print-style diagnostics.
- **Centralized policy** for minimum level and destination.

## Weaknesses
- **Implicit dependency** on global module state can obscure data flow.
- **Reduced test isolation** unless tests reset global logger state.
- **Context passing is manual** because no explicit logger handle carries defaults.

## Impact on Existing Syntax
No grammar changes are required. This is a standard library surface extension using existing function signatures, records, arrays, and error handling constructs.

## Interactions with Other Concerns
- **Error strategy**: fits module-level error enums well by exposing `LoggerInitError`, `LoggerNotInitializedError`, and `LogFlushError`.
- **Module organization**: encourages one bootstrap point that configures global logging once.
- **File I/O surface**: composes naturally with sync file append and fsync-backed flush primitives.
- **LSP and docs**: straightforward API signatures improve discoverability.

## Implementation Difficulty
Low to medium. Core work is runtime-safe module state plus buffered writer plumbing. Main complexity is guaranteeing deterministic flush behavior on process termination boundaries.

## Must NOT Have
- No exceptions or implicit unwinding behavior.
- No deferred methods in this alternative.
- No hidden periodic background flush thread.
- No abbreviated names like `log_i` or `dbg`.
