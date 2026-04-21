# Structured Log Events

## Overview
This alternative models logging as typed event emission rather than plain string lines. Applications construct a `LogEvent` value and emit it to a `LogSink`, making logs machine-readable by default and suitable for indexing, routing, and analytics.

The design favors schema clarity and downstream tooling compatibility. Human-readable formatting can still be generated, but the canonical representation remains structured data.

## Assumes
- Sum types are available for defining `LogEvent` variants.
- A sink abstraction can buffer and persist encoded events.
- Error propagation remains explicit via `errors`, `guard`, and `propagate`.
- No deferred API is introduced in this phase.

## Syntax Design
```opal
import LogSink, LogEvent, SinkCreateError, EventWriteError, SinkFlushError from ./logging.types

##
Description: Opens a structured LogSink that persists encoded LogEvent values.
##
let new_log_sink_sync = f(log_path: string): LogSink errors SinkCreateError =>
    return new LogSink:
        destination_path: log_path

##
Description: Emits one typed LogEvent into the sink buffering pipeline.
##
let emit_event = f(mutable sink: LogSink, event: LogEvent): void errors EventWriteError =>
    return void

##
Description: Flushes buffered structured events from sink memory to storage.
##
let flush_sync = f(mutable sink: LogSink): void errors SinkFlushError =>
    return void
```

## Example Applications
```opal
import LogSink, LogEvent, SinkCreateError, EventWriteError, SinkFlushError from ./logging.types
import new_log_sink_sync, emit_event, flush_sync from ./structured_logger

##
Description: Emits structured events for API traffic and authentication failures.
##
let record_observability_sync = f(route: string, user_identifier: string): void errors SinkCreateError, EventWriteError, SinkFlushError =>
    # Initialize destination for machine-readable event storage.
    let mutable sink = propagate new_log_sink_sync('./var/events.log')

    # Emit HTTP timing event for dashboards and SLO alerts.
    propagate emit_event(sink, new LogEvent:
        HttpRequestCompleted:
            route: route
            status_code: 200
            elapsed_milliseconds: 42
    )

    # Emit auth failure event for security monitoring and triage.
    propagate emit_event(sink, new LogEvent:
        AuthenticationFailed:
            user_identifier: user_identifier
            reason: 'invalid password'
    )

    # Emit domain event to track inventory drift.
    propagate emit_event(sink, new LogEvent:
        InventoryAdjusted:
            sku: 'A-100'
            previous_quantity: 7
            new_quantity: 5
    )

    # Force durability so downstream tailers observe complete batches.
    propagate flush_sync(sink)
    return void
```

## Strengths
- **Machine-readable by default** enables robust indexing and analytics.
- **Strong schema discipline** through typed event variants.
- **Safer refactors** because event payload changes are compiler-checked.
- **Clear bridge to telemetry pipelines** that expect structured data.

## Weaknesses
- **Higher upfront design cost** for event schemas.
- **Potential verbosity** for simple ad-hoc debugging statements.
- **Versioning burden** when evolving event payload fields.

## Impact on Existing Syntax
No syntax changes are required. This alternative extends the standard library with sum-type-based event APIs and sink utilities.

## Interactions with Other Concerns
- **Error strategy**: sink create, emit, and flush all expose explicit typed failures.
- **Serialization concern**: event encoding format must align with chosen serialization primitives.
- **Network/HTTP concern**: HTTP-focused event variants compose naturally with request middleware.
- **Module organization**: encourages dedicated event-schema modules per domain.

## Implementation Difficulty
Medium to high. Runtime and stdlib work must define stable encoding, sink buffering, and schema-aware formatting while preserving predictable synchronous flush behavior.

## Must NOT Have
- No untyped map-only event payload API.
- No exceptions or implicit panic recovery.
- No deferred methods in this phase.
- No fallback to hidden global sink state.
