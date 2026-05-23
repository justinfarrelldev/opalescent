## 2026-05-23T00:43:19-04:00
- Pre-fix mutable reassignment from a fresh RC-returning user function leaks one array per iteration in the focused regression (`live=128` after 128 loop iterations).
- Alias-return ownership provenance remains unresolved by design; the ignored `alias_return_assignment_known_limitation` test documents this as a separate future problem instead of claiming it is fixed here.
