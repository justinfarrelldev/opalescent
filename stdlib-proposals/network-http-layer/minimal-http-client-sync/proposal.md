# Minimal HTTP Client (Sync)

## Overview
This proposal introduces a minimal, functional-first synchronous HTTP client for Opalescent. It provides high-level `http_get_sync` and `http_post_sync` functions alongside a more flexible `http_request_sync` for complex needs. The design prioritizes simplicity and directness, mapping directly to common web operations without requiring complex object lifecycles or builder patterns.

## Assumes
- Standard Opalescent error handling (`guard`, `propagate`, `errors` clause).
- Native support for record types and fixed-width numeric types.
- A standard library module for byte array manipulation (`uint8[]`).

## Syntax Design
```op
import http_get_sync, http_post_sync from standard

let response = propagate http_get_sync('https://api.example.com', [])
```

## Example Applications
```op
let headers = [new HttpHeader: name: 'User-Agent', value: 'Opalescent']
guard http_get_sync('https://api.example.com/json', headers) into res else err =>
    # handle error
    return void
```

## Strengths
- **Low Boilerplate**: One-liner requests for common operations.
- **Purely Functional**: No hidden state; all configuration is passed in the request.
- **Predictable**: Synchronous execution makes control flow trivial to reason about.

## Weaknesses
- **Repetitive Configuration**: Cannot easily share configuration (like base URLs or headers) across multiple requests without manual passing.
- **Limited Flexibility**: Adding new configuration options (like per-request timeouts) might require changing many function signatures.

## Impact on Existing Syntax
None. This is a purely additive standard library proposal.

## Interactions with Other Concerns
- **Serialization**: Works closely with the JSON/binary serialization layer for processing response bodies.
- **Error Strategy**: Enforces the 9 standard network error types.

## Implementation Difficulty
Low. Maps easily to existing system-level HTTP libraries (like libcurl or native platform APIs).

## Must NOT Have
- Server-side capabilities.
- WebSocket or SSE support.
- Streaming response/request bodies.
