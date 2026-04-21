

## [2026-04-21] T4 decisions
- Kept validator insertion point strictly between parse and type-check in both single-file pipelines (`compile_to_module` and `run_check_command`) so file-role errors surface as type-phase diagnostics before deeper semantic checks.
- Reused existing `CompilationErrorReport` rendering/accumulation behavior rather than introducing a new error path.

## [2026-04-21] T5 decisions
- Preserved compile_project validation ordering by adding module file-role validation directly after existing entry-placement validation, so both structural checks happen before module type-checking.
- Chose to return `CompileError::Report` (with one type error) for project file-role violations to match the established error accumulation/reporting idiom called out by task context.
- Skipped `.types.op` modules in the codegen emission loop (instead of filtering discovery) to keep dependency/type-check traversal unchanged while preventing invalid object generation.
