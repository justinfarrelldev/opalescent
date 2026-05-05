# Task 8 map feasibility check

Timestamp: 2026-05-05 04:49:25Z

Result: FEASIBLE without new generic method infrastructure.

Evidence:
- `src/type_system/checker/collections/collections_array.rs:59-72` already registers `[t].map` with parameter `f(T): U` and return type `U[]`.
- `src/type_system/checker/collections/collections_array.rs:149-193` only pre-binds receiver element type `T`, leaving `U` unresolved.
- `src/type_system/checker/call_resolution.rs:47-107` and `:179-360` instantiate unresolved variables fresh per call and unify parameter/return structure against call arguments.
- `src/type_system/checker/expressions.rs:735-877` types lambdas with declared parameter and return annotations, so a lambda like `f(x: int32): int32 => ...` can satisfy the callback shape and infer `U=int32`.
- Therefore `.map` can be implemented as a specialized compiler-lowered intrinsic using the existing call-site inference path; no broad new generic method framework is required.

Scope note:
- This feasibility result covers the explicit Task 8 slice `map<U>(f(T): U): U[]` only.
- It does not imply that broader generic member methods or captured closure infrastructure are required or implemented.
