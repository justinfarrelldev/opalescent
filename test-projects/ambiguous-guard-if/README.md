# ambiguous-guard-if

This fixture is intentionally invalid. It uses a bare `if ... else ...` as the guard subject
inside a statement guard, which triggers the dedicated parser diagnostic for ambiguous guarded if/else.

Expected compile error variant: `ParseError::GuardAmbiguousIfElse`
Expected diagnostic code: `opalescent::parser::guard_ambiguous_if_else`
