# Request Builder (Sync)

## Overview
This proposal adopts a builder pattern for constructing synchronous HTTP requests in Opalescent. It introduces an `HttpRequestBuilder` value type with chainable modifier functions (`with_header`, `with_body`) and a terminal `send_sync()` execution step. This approach allows for readable, declarative request definitions while maintaining synchronous execution.

## Assumes
- Opalescent's record type and value-based immutability.
- Functional chaining via function calls (as Opalescent lacks method-style chaining syntax).

## Syntax Design
```op
let builder = new_request_builder(HttpMethod.Get, 'https://api.example.com')
let builder_with_header = with_header(builder, 'Accept', 'application/json')
let response = propagate send_sync(builder_with_header)
```

## Example Applications
```op
let post_builder = with_body(with_header(new_request_builder(HttpMethod.Post, 'https://api.example.com'), 'Content-Type', 'text/plain'), [1, 2, 3])
guard send_sync(post_builder) into response else err =>
    return void
```

## Strengths
- **Declarative Construction**: Clearer intent when setting multiple headers or complex request parameters.
- **Composable**: Builders can be partially constructed and passed between functions.
- **Extensible**: New configuration fields can be added to the builder without breaking existing `send_sync()` calls.

## Weaknesses
- **More Boilerplate**: Requires more lines of code for simple requests compared to the minimal client.
- **Nested Calls**: Without method chaining, deeply modified builders can lead to nested or multiple temporary variables.

## Impact on Existing Syntax
None. Purely additive to the standard library.

## Interactions with Other Concerns
- **Error Strategy**: All `send_sync()` calls must handle the 9 standard network error types.
- **Serialization**: Builders often work with serializers to set the `body` field.

## Implementation Difficulty
Medium. Requires implementing the builder value type and ensuring efficient field copying or mutation.

## Must NOT Have
- Server-side features.
- WebSocket support.
- Async execution.
