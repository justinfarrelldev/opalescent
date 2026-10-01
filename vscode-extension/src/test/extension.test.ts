import path from 'node:path';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

const vscodeState = vi.hoisted(() => ({
  diagnosticCollection: {
    clear: vi.fn(),
    delete: vi.fn(),
    dispose: vi.fn(),
    set: vi.fn()
  },
  openHandlers: [] as Array<(document: unknown) => void>,
  outputChannel: {
    appendLine: vi.fn(),
    dispose: vi.fn()
  },
  subscriptions: [] as unknown[],
  workspaceFolders: [] as Array<{ uri: { fsPath: string } }>
}));

const childProcessMock = vi.hoisted(() => ({
  execFile: vi.fn()
}));

vi.mock('node:child_process', () => childProcessMock);

vi.mock('vscode', () => {
  class Diagnostic {
    public code?: string;
    public source?: string;

    public constructor(
      public readonly range: unknown,
      public readonly message: string,
      public readonly severity: unknown
    ) {}
  }

  class Position {
    public constructor(
      public readonly line: number,
      public readonly character: number
    ) {}
  }

  class Range {
    public readonly end: Position;
    public readonly start: Position;

    public constructor(startLine: Position | number, startCharacter: Position | number, endLine?: number, endCharacter?: number) {
      if (startLine instanceof Position && startCharacter instanceof Position) {
        this.start = startLine;
        this.end = startCharacter;
        return;
      }
      this.start = new Position(Number(startLine), Number(startCharacter));
      this.end = new Position(Number(endLine), Number(endCharacter));
    }
  }

  return {
    CodeActionKind: { QuickFix: 'quickfix' },
    CompletionItem: class CompletionItem {},
    CompletionItemKind: {
      Class: 6,
      Enum: 12,
      EnumMember: 20,
      Field: 5,
      Function: 2,
      Keyword: 14,
      Snippet: 15,
      Variable: 4
    },
    Diagnostic,
    DiagnosticSeverity: {
      Error: 0,
      Hint: 3,
      Information: 2,
      Warning: 1
    },
    Hover: class Hover {},
    MarkdownString: class MarkdownString {
      public isTrusted = false;

      public constructor(public readonly value: string) {}
    },
    Position,
    Range,
    SnippetString: class SnippetString {
      public constructor(public readonly value: string) {}
    },
    TextEdit: {
      insert: vi.fn(),
      replace: vi.fn()
    },
    Uri: {
      file: (fsPath: string) => ({ fsPath, toString: () => `file://${fsPath}` })
    },
    WorkspaceEdit: class WorkspaceEdit {
      public replace = vi.fn();
    },
    commands: {
      executeCommand: vi.fn(),
      registerCommand: vi.fn(() => ({ dispose: vi.fn() }))
    },
    languages: {
      createDiagnosticCollection: vi.fn(() => vscodeState.diagnosticCollection),
      registerCodeActionsProvider: vi.fn(() => ({ dispose: vi.fn() })),
      registerCodeLensProvider: vi.fn(() => ({ dispose: vi.fn() })),
      registerCompletionItemProvider: vi.fn(() => ({ dispose: vi.fn() })),
      registerDefinitionProvider: vi.fn(() => ({ dispose: vi.fn() })),
      registerDocumentFormattingEditProvider: vi.fn(() => ({ dispose: vi.fn() })),
      registerHoverProvider: vi.fn(() => ({ dispose: vi.fn() })),
      registerImplementationProvider: vi.fn(() => ({ dispose: vi.fn() }))
    },
    window: {
      activeTextEditor: undefined,
      createOutputChannel: vi.fn(() => vscodeState.outputChannel),
      createTerminal: vi.fn(),
      showErrorMessage: vi.fn(),
      showOpenDialog: vi.fn(),
      showWarningMessage: vi.fn()
    },
    workspace: {
      getConfiguration: vi.fn(() => ({
        get: vi.fn((_key: string, defaultValue: unknown) => defaultValue),
        update: vi.fn()
      })),
      onDidChangeTextDocument: vi.fn(() => ({ dispose: vi.fn() })),
      onDidCloseTextDocument: vi.fn(() => ({ dispose: vi.fn() })),
      onDidOpenTextDocument: vi.fn((handler: (document: unknown) => void) => {
        vscodeState.openHandlers.push(handler);
        return { dispose: vi.fn() };
      }),
      onDidSaveTextDocument: vi.fn(() => ({ dispose: vi.fn() })),
      openTextDocument: vi.fn(),
      get textDocuments() {
        return [];
      },
      get workspaceFolders() {
        return vscodeState.workspaceFolders;
      }
    }
  };
});

beforeEach(() => {
  vi.useFakeTimers();
  vi.resetModules();
  vscodeState.diagnosticCollection.clear.mockClear();
  vscodeState.diagnosticCollection.delete.mockClear();
  vscodeState.diagnosticCollection.dispose.mockClear();
  vscodeState.diagnosticCollection.set.mockClear();
  vscodeState.openHandlers = [];
  vscodeState.outputChannel.appendLine.mockClear();
  vscodeState.outputChannel.dispose.mockClear();
  vscodeState.subscriptions = [];
  vscodeState.workspaceFolders = [];
  childProcessMock.execFile.mockReset();
  childProcessMock.execFile.mockImplementation((_binary, args, optionsOrCallback, maybeCallback) => {
    const callback = typeof optionsOrCallback === 'function' ? optionsOrCallback : maybeCallback;
    if (args[0] === 'help') {
      callback(null, '', '');
      return;
    }
    callback(
      null,
      JSON.stringify({
        diagnostics: [
          {
            code: 'opalescent::type_system::unused_variable',
            message: "Variable 'close_outcome' is never used",
            phase: 'type checker',
            range: { end: { character: 21, line: 86 }, start: { character: 8, line: 86 } },
            severity: 'warning',
            source_path: args.at(-1)
          }
        ],
        success: true
      }),
      ''
    );
  });
});

afterEach(() => {
  vi.useRealTimers();
});

test('runs compiler-backed diagnostics when an Opalescent document is opened', async () => {
  const workspaceRoot = path.join(path.sep, 'workspace');
  const filePath = path.join(workspaceRoot, 'src', 'main.op');
  vscodeState.workspaceFolders = [{ uri: { fsPath: workspaceRoot } }];

  const { activate, deactivate } = await import('../extension.js');
  activate({ subscriptions: vscodeState.subscriptions } as never);

  expect(vscodeState.openHandlers).toHaveLength(1);

  vscodeState.openHandlers[0]?.({
    getText: () => 'entry main = f(args: string[]): void =>\n    return void\n',
    languageId: 'opalescent',
    uri: { fsPath: filePath, toString: () => `file://${filePath}` }
  });
  await vi.advanceTimersByTimeAsync(351);

  const checkCall = childProcessMock.execFile.mock.calls.find((call) => Array.isArray(call[1]) && call[1][0] === 'check');
  expect(checkCall?.[1]).toEqual(['check', '--json', filePath]);
  expect(vscodeState.diagnosticCollection.set).toHaveBeenCalledWith(
    expect.objectContaining({ fsPath: filePath }),
    expect.arrayContaining([expect.objectContaining({ message: "Variable 'close_outcome' is never used" })])
  );
  deactivate();
});
