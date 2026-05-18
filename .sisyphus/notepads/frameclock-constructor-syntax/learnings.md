
## 2026-05-18 21:xx:xxZ Task 14
- Codegen now prefers canonical `CoreType` identity when lowering registered fallible constructors, so imported aliases to `FrameClock` reuse the `frame_clock_new` registry entry instead of relying only on raw source spelling.
- `codegen_guard_expression` and `codegen_propagate_expression` now forward the caller-provided expected type into nested constructor lowering, which keeps error-flow lowering aligned with the typechecker's canonical constructor resolution.
- Added a focused codegen regression for `FrameClockAlias` plus the existing type-system canonical lookup regressions; all three targeted tests passed.
- When a review gate flags formatting drift only, the safest unblocker is to revert just the reflow hunk and keep the feature commit semantically identical.
- Scope-fidelity cleanup can be isolated to a single whitespace hunk, letting the amended commit stay behavior-neutral while unblocking verification.
