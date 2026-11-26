# COMPILER-002: array field reads in `let` bindings

- [x] Red: the new E2E fixture reported `stored=1` instead of `stored=0` after the alias left scope.
- [x] Green: retain borrowed array fields when initializing owned local bindings; preserve transfer of fresh arrays.
- [x] Recheck the database: the isolated native regression passed, but checking the whole project exposed dangling arrays in nominal sum payloads.
- [x] Red: the variant fixture reported `payload=1` instead of `payload=0` when its constructor's source local went out of scope.
- [x] Green: give nominal sum payloads ownership of borrowed arrays before their source locals exit.
- [x] Refactor and review: share provisional array field ownership with nominal products and verify fresh-array cleanup with runtime counters.
- [x] Verify the database: all 13 native tests passed, and the ordinary executable created two tables.
