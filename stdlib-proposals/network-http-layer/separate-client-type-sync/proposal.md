# Separate Client Type (Sync)

## Overview
This proposal separates the HTTP client configuration from individual requests. It introduces an `HttpClient` value type to hold shared settings such as base URLs, default headers, and common timeouts. Requests are executed by passing both a client instance and a request instance to `http_client_execute_sync()`. This approach excels in scenarios with multiple requests to the same service.

## Assumes
- Opalescent's record type and value-based immutability.
- Standard library modules for headers and request types.

## Syntax Design
```op
let client = new_http_client('https://api.example.com', [new HttpHeader: name: 'User-Agent', value: 'Opalescent'], 30000)
let request = new HttpRequest:
    method: HttpMethod.Get
    url: '/data'
    headers: []
    body: []
let response = propagate http_client_execute_sync(client, request)
```

## Example Applications
```op
let client = new_http_client('https://api.github.com', [new HttpHeader: name: 'Accept', value: 'application/vnd.github.v3+json'], 30000)
let get_user = new HttpRequest:
    method: HttpMethod.Get
    url: '/users/octocat'
    headers: []
    body: []
guard http_client_execute_sync(client, get_user) into response else err =>
    return void
```

## Strengths
- **Shared Configuration**: Reduces duplication when communicating with a single API or service.
- **Resource Management**: Provides a natural place for connection pool settings or authentication headers.
- **Clean Execution**: `http_client_execute_sync()` remains simple while being highly configurable.

## Weaknesses
- **Initial Boilerplate**: Even simple one-off requests require initializing a client instance.
- **Complexity**: Introduces an extra concept (the client) for developers to manage.

## Impact on Existing Syntax
None. Purely additive.

## Interactions with Other Concerns
- **Error Strategy**: All executions must handle the 9 standard network error types.
- **Serialization**: Clients often work with serializers to process response bodies.

## Implementation Difficulty
Medium. Requires merging client-level and request-level settings before execution.

## Must NOT Have
- Server-side features.
- WebSocket or SSE support.
- Async execution.
