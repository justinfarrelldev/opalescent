# Module-interface Layout Manifests Implementation Plan

## Goal

Implement `language-proposals/user-defined-adts-cross-module/module-interface-layout-manifests/proposal.md` so public user-defined ADT layout metadata is exported through module interfaces and consumed by downstream type checking/codegen, including transitive imports.

## Required behavior

- Add canonical module-qualified ADT identity and structured layout manifests to `ModuleInterface`.
- Populate public product/sum layout manifests from `.types.op` declarations.
- Include ordered fields, ordered variants, discriminants, resolved core field types, ownership/drop hints, and deterministic layout hashes.
- Preserve nominal identity internally by using canonical layout keys and module-local aliases rather than relying on ambiguous short display names.
- Propagate layout manifests through direct and transitive module discovery.
- Lower imported product construction/field access, enum construction/variant checks, and payload variant construction/`into` payload access from manifest data.
- Detect same-short-name ambiguity instead of silently choosing a layout; explicit import aliases remain supported.
- Keep existing source syntax unchanged and existing working fixtures compiling.

## TDD plan

1. **Red**
   - Add unit coverage proving `.types.op` checking emits structured manifests with canonical module IDs, field/variant metadata, discriminants, and layout hashes.
   - Add an end-to-end proposal fixture exercising transitive manifest use, user sum discriminants, payload `is ... into`, and payload field access.
   - Add an end-to-end collision/alias fixture proving same short type names from different modules can be used safely through explicit aliases.
   - Run the targeted tests and capture the expected failures.
2. **Green**
   - Implement manifest data structures and deterministic layout hashes.
   - Populate manifests during type declaration signature registration.
   - Thread manifest-derived canonical layouts, aliases, field indices, and variant discriminants into codegen environments.
   - Update ADT constructor, field access, variant tag comparison, and branch-local refinement lowering to resolve through manifests.
   - Add sum-payload child-drop callbacks for RC-owned payload fields.
   - Run targeted tests until green.
3. **Refactor**
   - Consolidate duplicate layout conversion helpers.
   - Review manifest/alias ambiguity handling for maintainability and diagnostics.
   - Run broader relevant tests and lint/build checks.
   - Perform a final code review against the proposal Must NOT Have list.

## Atomic commit plan

1. Add plan/checklist and red tests/fixtures.
2. Add module-interface manifest model and type-checker population.
3. Thread manifest layouts/aliases into codegen and implement user sum variant lowering.
4. Add collision-safe alias behavior and cleanup/refactor.
5. Final review/test updates if needed.
