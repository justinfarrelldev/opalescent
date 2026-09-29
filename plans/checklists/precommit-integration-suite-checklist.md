# Pre-commit integration suite checklist

- [x] Confirm full integration red failures are from stdlib error-family tests.
- [x] Add a `cargo make test-all` task that runs the normal suite and integration E2E suite.
- [x] Update `.git/hooks/pre-commit` test invocation to use `cargo make test-all` only.
- [x] Update stale stdlib error-family warning fixtures to canonical plural families.
- [x] Remediate saferm's environment optional lookup error family warning.
- [x] Run focused failing integration tests.
- [x] Run full `cargo make test-all`.
- [x] Run pre-commit.
