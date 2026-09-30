# VS Code Extension Lint + Pre-commit Checklist

- [x] Read required project documentation before implementation.
- [x] Add ESLint configuration equivalent to the requested setup inside `vscode-extension`.
- [x] Add extension lint script that fails on any warning or error.
- [x] Update `vscode-extension` dependencies and lockfile for the lint setup.
- [x] Add extension lint and extension test gates to `.git/hooks/pre-commit` without changing other hook behavior.
- [x] Run enough validation to confirm the new commands are wired in place, without fixing lint or test failures.
- [x] Review changed files.
