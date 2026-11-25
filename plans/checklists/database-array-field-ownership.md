# DB-001: returned record array field ownership

- [x] Obtain approval for the compiler fix and separate regression tests.
- [x] Reproduce the lost array ownership in dedicated production and native regression tests (red).
- [x] Fix array ownership at the nominal product constructor boundary (green).
- [x] Refactor field lowering, verify fresh arrays do not get an extra retain, and test a later initializer failure.
- [x] Record the fix and the separate outstanding database-state failure.
- [x] Keep the compiler fix and its regression tests separate from the database executor changes.
