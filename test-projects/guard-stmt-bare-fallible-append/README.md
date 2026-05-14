# guard-stmt-bare-fallible-append

Compile-fail fixture proving that fallible calls inside a named guard error clause still require explicit handling.

The helper parses with `string_to_int32`, then attempts a bare `append_text_sync` inside `else err =>` before forwarding the original parse error. The compiler should reject the bare append call with `UnhandledCallError` instead of silently discarding its filesystem errors.
