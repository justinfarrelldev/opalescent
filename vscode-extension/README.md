# Opalescent Language Support

VS Code support for Opalescent source files (`.op`) and type-definition files (`.types.op`).

## Features

- TextMate syntax highlighting for current Opalescent syntax.
- Compiler-backed diagnostics through `opalescent check --json`.
- Formatting through `opalescent fmt`.
- Command palette actions for lint/check, format, build, run, and selecting the compiler binary.
- Code lenses above `entry` declarations for build and run.
- Project-wide symbol completions with auto-import edits for public declarations.
- Conservative text-indexed go-to-definition and find-implementation support.

## Compiler binary

Set `opalescent.binaryPath` to the compiler binary, or run `Opalescent: Select Compiler Binary` from the command palette. If unset, the extension tries common workspace build outputs and PATH names.

## Commands

- `Opalescent: Select Compiler Binary`
- `Opalescent: Lint Current File`
- `Opalescent: Format Document`
- `Opalescent: Build Project`
- `Opalescent: Run Project`

## Settings

- `opalescent.binaryPath`
- `opalescent.lintOnSave`
- `opalescent.lintOnChange`
