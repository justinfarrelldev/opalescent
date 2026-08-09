# Issues

- The required `test(type_system): add failing stdlib error family taxonomy` commit is blocked by `.git/hooks/pre-commit`: its unconditional `cargo make test` rejects the intentional RED tests. The hook provides no documented RED workflow or opt-out; `git commit --no-verify` is explicitly prohibited, so the three intended files remain staged but uncommitted.
- Task 2 turns both inherited RED tests green without introducing warning diagnostics: the warning test seam validates the exact future replacement text through the registry metadata, while Task 4 remains responsible for Miette warning emission.
