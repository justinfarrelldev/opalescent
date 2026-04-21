# Logger Handle

## Overview
This alternative introduces an explicit `Logger` handle that is created once and passed to functions that need logging. The handle stores destination, minimum level, and buffering policy, so dependencies remain visible in every signature.

The design emphasizes explicit data flow and strong testability. Services can pass a production logger in runtime paths and a test logger in unit tests without relying on global mutable state.

## Assumes
- Record types can represent a mutable logger handle.
- Standard library can expose constructor and sink-backed write functions.
- Errors are typed and propagated explicitly with `guard` or `propagate`.
- No deferred API is introduced.

## Syntax Design
```opal
import Logger, LogLevel, LogField, LoggerCreateError, LogFlushError from ./logging.types

##
Description: Creates a Logger handle bound to destination and level policy.
##
let new_logger_sync = f(log_path: string, minimum_level: LogLevel): Logger errors LoggerCreateError =>
    return new Logger:
        destination_path: log_path
        minimum_level: minimum_level

##
Description: Buffers a debug message using the provided Logger handle.
##
let log_debug = f(mutable logger: Logger, message: string, fields: LogField[]): void =>
    return void

##
Description: Buffers an info message using the provided Logger handle.
##
let log_info = f(mutable logger: Logger, message: string, fields: LogField[]): void =>
    return void

##
Description: Buffers a warning message using the provided Logger handle.
##
let log_warn = f(mutable logger: Logger, message: string, fields: LogField[]): void =>
    return void

##
Description: Buffers an error message using the provided Logger handle.
##
let log_error = f(mutable logger: Logger, message: string, fields: LogField[]): void =>
    return void

##
Description: Flushes buffered records owned by a Logger handle to storage.
##
let flush_sync = f(mutable logger: Logger): void errors LogFlushError =>
    return void
```

## Example Applications
```opal
import Logger, LogLevel, LogField, LoggerCreateError, LogFlushError from ./logging.types
import new_logger_sync, log_debug, log_info, log_warn, log_error, flush_sync from ./logger

##
Description: Processes one order while explicitly threading Logger dependencies.
##
let process_order_sync = f(order_id: string): void errors LoggerCreateError, LogFlushError =>
    # Create a logger once at operation entry.
    let mutable logger = propagate new_logger_sync('./var/orders.log', LogLevel.Debug)

    # Emit a traceable debug event before validation logic.
    log_debug(logger, 'starting order validation', [new LogField:
        key: 'order_id'
        value: order_id
    ])

    # Emit high-level progress for operators.
    log_info(logger, 'order accepted', [new LogField:
        key: 'order_id'
        value: order_id
    ])

    # Emit recoverable risk indicators.
    log_warn(logger, 'inventory below threshold', [new LogField:
        key: 'sku'
        value: 'A-100'
    ])

    # Emit terminal error context when business rules fail.
    log_error(logger, 'payment authorization failed', [new LogField:
        key: 'processor'
        value: 'card-network'
    ])

    # Persist all buffered entries before function exit.
    propagate flush_sync(logger)
    return void
```

## Strengths
- **Explicit dependencies** improve readability and architecture boundaries.
- **Excellent testability** by substituting alternate logger handles.
- **No global mutable state** needed in normal usage.
- **Composable context** because caller-owned handle can carry environment metadata.

## Weaknesses
- **More boilerplate** from passing `Logger` through call chains.
- **Signature expansion** in deeply layered systems.
- **Lifecycle burden** on callers to flush at appropriate boundaries.

## Impact on Existing Syntax
No parser or language syntax changes are needed. This alternative only adds standard library types and functions.

## Interactions with Other Concerns
- **Error strategy**: constructor and flush align with explicit error enums and propagation.
- **Dependency injection style**: naturally aligns with explicit capability passing.
- **Concurrency model**: each worker can own an independent `Logger` without shared mutable global state.
- **Module organization**: encourages boundary-oriented helper functions that accept `Logger`.

## Implementation Difficulty
Medium. The compiler impact is negligible, but library/runtime work must implement efficient handle mutation and predictable flush semantics.

## Must NOT Have
- No global default logger hidden behind this API.
- No deferred methods in this alternative.
- No exception-based control flow.
- No implicit flushing on every log call.
