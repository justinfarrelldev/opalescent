# VS Code Extension Open Diagnostics Checklist

- [x] Read required project documentation before implementation.
- [x] Identify that editor diagnostics are not registered for newly opened Opalescent documents.
- [x] RED: add a failing extension test proving `onDidOpenTextDocument` schedules compiler-backed diagnostics for an opened Opalescent file.
- [x] GREEN: register an open-document diagnostic hook so files report compiler diagnostics as soon as they are opened.
- [x] REFACTOR: keep event wiring concise and avoid duplicate behavior.
- [x] Run focused/full extension tests and lint.
- [x] Review changed files.
