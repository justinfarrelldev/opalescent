import { expect, test } from 'vitest';

import { collectLocalLintDiagnostics } from '../lint.js';

test('reports snake_case diagnostics for unsaved function and local bindings', () => {
  const source = `##
  Description: Example function with enough documentation.
##
public let thisCase = f(badParam: int32): void =>
    let alsoBad = badParam
    return void
`;

  const diagnostics = collectLocalLintDiagnostics(source, '/p/src/main.op');

  expect(diagnostics.map((diagnostic) => [diagnostic.code, diagnostic.range.start.line, diagnostic.range.start.character])).toEqual([
    ['opalescent.vscode.naming.snake_case', 3, 11],
    ['opalescent.vscode.naming.snake_case', 3, 24],
    ['opalescent.vscode.naming.snake_case', 4, 8]
  ]);
});

test('reports missing documentation comments for public declarations', () => {
  const source = `public let missing_doc = f(): void =>
    return void

public type MissingType:
    Value
`;

  const diagnostics = collectLocalLintDiagnostics(source, '/p/src/main.op');

  expect(diagnostics.map((diagnostic) => [diagnostic.code, diagnostic.range.start.line])).toEqual([
    ['opalescent.vscode.documentation.missing', 0],
    ['opalescent.vscode.documentation.missing', 3]
  ]);
});
