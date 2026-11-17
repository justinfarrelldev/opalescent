# Native testing value contracts

Assertions compare typed values, not diagnostic strings or pointer identity.
Supported equality includes numeric/boolean values, UTF-8 strings, recursively
supported arrays, acyclic supported products and payloadless nominal variants.
Nominal variant assertions may identify a payload-bearing variant without reading
its payload.

## Double argument snapshots

Exact rules and `to have been called with` support:

- Numeric and boolean primitives.
- UTF-8 strings.
- Arrays, including nested arrays, whose elements recursively belong to the
  preceding managed-value set.
- Standalone builtin `FilesystemPath` values.

Strings and arrays use existing reference-counting/copy-on-write semantics.
Configured operands are captured before later expressions run. History copies
logical argument values during the call; it does not keep the addresses of
borrowed variables. History remains valid after those caller scopes exit.

A filesystem path currently has an immutable UTF-8 byte-string representation.
Path equality compares that value, after any construction-time expansion. It
performs no filesystem operations, normalization, file-identity lookup or
platform case folding. A snapshot owns a separate managed copy of the bytes.
An unrelated user type with the same short name does not gain these semantics.

Other nominal records, resource/opaque handles, function values and arrays of
caller-owned paths do not support argument-history assertions or exact rules in
v1. Such uses are compiler diagnostics, including during case listing. Count-only
original spies and statically resolved replacement functions remain available.

## Stored results

Stored rules support ordinary primitive, string and recursively supported managed
array results. Every repeated return receives its normal value ownership.
Caller-owned/affine results require a replacement function or original spy rather
than duplicating a stored owner. Deliberately configured nominal application
errors use the existing application-error ABI, not the test-abort channel.

Generated release callbacks dispose only framework-owned value memory using the
existing value rules. They do not register, discover, close or recover user
resources, infer World cleanup, or introduce a language ownership protocol.
