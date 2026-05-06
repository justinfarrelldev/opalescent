# Task 11 STOP: `Pair<T, U>` is not language-visible for `.zip`

Timestamp: 2026-05-05T02:10:52-04:00

## Decision
STOP the array-functionality plan at Task 11 before creating `test-projects/array-zip` or implementing `.zip`.

## Proof Summary
I confirmed that the language supports generic product ADTs *when the source program declares them itself*, but I did **not** find an existing built-in, prelude, or standard-module-visible `Pair<T, U>` type that user source can reference for `left.zip(right): Pair<T, U>[]`.

Because Task 11 requires using the **existing** `Pair<T, U>` return shape only if it is already language-visible, proceeding would require inventing new language-visible `Pair` support, which the plan explicitly forbids.

## Files searched and findings

### `.zip` return shape uses `Pair`
- `src/type_system/checker/collections/collections_array.rs`
  - `.zip` is registered with return type `Pair<T, U>[]` via `CoreType::Generic { name: "Pair", ... }`.

### Generic product constructor + field access syntax exists in principle
- `src/type_system/test_integration_generics.rs`
  - Inline test source declares:
    - `type Pair<T, U>:`
    - fields `first: T`, `second: U`
    - constructor syntax `new Pair:`
- `src/parser/expressions.rs`
  - parser supports `new ...` constructor expressions and `.member` field access syntax.
- `src/type_system/checker/expressions.rs`
  - checker resolves ADT field access for generic nominal types.
- `src/codegen/adts.rs`
  - codegen lowers product constructors and product field access.
- `src/type_system/test_integration_adt.rs`
  - product field access like `person.name` type-checks.

### Existing language-visible `Pair` type was **not** found
- `src/type_system/environment.rs`
  - built-in types include primitives, `void`, and `ParseError`; no `Pair` registration.
- `src/type_system/checker.rs`
  - standard builtins register `Option`, print/input/random/etc.; no `Pair` registration.
- `src/type_system/module_resolver/standard_modules.rs`
  - standard/math module interfaces and ADT field registrations do not export `Pair`.
- `stdlib/prelude.op`
  - no prelude declaration for `Pair`.
- `src/type_system/checker/module_checking.rs`
  - imported ADT field metadata only comes from module interfaces; there is no module-exported `Pair` to import.
- Repository searches for `Pair` under `src/type_system/module_resolver` and `stdlib` returned no built-in/module definition.
- Repository search of `test-projects/**/*.op` found no existing imported or built-in `Pair` usage; only locally declared ADTs and test-only inline `Pair` declarations were found.

## Missing capability
The missing prerequisite is **an existing language-visible `Pair<T, U>` type definition/registration that user programs can reference without declaring it themselves**. Task 11 requires `.zip` to reuse that existing shape rather than introducing a new built-in product type or new source-level feature.

## Stop-scope confirmation
- Did **not** create `test-projects/array-zip`
- Did **not** implement `.zip`
- Did **not** proceed to Task 12 or final verification tasks
- Did **not** create `feat(array): implement zip`

## Verification of STOP path
- Filesystem check confirmed `test-projects/array-zip` does not exist.

## Required next step
Update the plan/user contract to provide an existing language-visible `Pair<T, U>` path (or change `.zip` surface semantics) before resuming beyond Task 11.
