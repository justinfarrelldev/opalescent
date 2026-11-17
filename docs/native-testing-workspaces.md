# Explicit temporary workspaces

Native test modules can import `TemporaryDirectory`, `temporary_directory_create`,
`temporary_directory_file`, and `temporary_directory_remove` from `standard.testing`.
Production modules cannot import this test-only interface.

- `temporary_directory_create()` creates a unique private directory. Creation and
  allocation failures are ordinary application errors, usable with `propagate`.
- `temporary_directory_file(ref directory, name)` returns an owned UTF-8 filename
  usable with `path_from` and the existing filesystem APIs. `name` must be a
  nonempty child basename, not `.`/`..`, and contain no directory separators.
  Violating that nonfallible helper's precondition terminates the worker as an
  infrastructure failure rather than returning a fabricated filename.
- `temporary_directory_remove(mutable ref directory)` explicitly removes the
  directory and its contents through the ordinary recursive filesystem operation.
  Removal errors are ordinary application errors. Successful removal consumes the
  directory handle; it must not be used again.

Declare removal in World cleanup to run it after successful setup, including body
assertion aborts. Omitting cleanup leaves the external directory alone. These
helpers install no registry, exit handler, resource-discovery or leak-protection
mechanism. Each attempt/process creates its own unique directory; no implicit
World/file/scenario serialization is introduced.
