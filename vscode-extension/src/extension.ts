import * as childProcess from 'node:child_process';
import * as fs from 'node:fs';
import * as fsp from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { candidateBinaryPaths, shellQuote } from './binary.js';
import { buildArgsForContext, checkArgsForContext, formatArgs, runArgsForContext, type OpalescentCommandContext } from './cli.js';
import { diagnosticsByFile, parseOpalescentDiagnosticReport, type OpalescentDiagnostic } from './diagnostics.js';
import { findProjectRoot, isOpalescentFile } from './project.js';
import { collectSymbolsFromSource, findEntryLines, wordAtPosition, type OpalescentSymbol } from './symbols.js';

const languageId = 'opalescent';
const diagnosticCollection = vscode.languages.createDiagnosticCollection('opalescent');
const outputChannel = vscode.window.createOutputChannel('Opalescent');
const diagnosticTimers = new Map<string, NodeJS.Timeout>();
let resolvedBinary: string | undefined;

interface CompilerResult {
  code: number;
  stdout: string;
  stderr: string;
}

export function activate(context: vscode.ExtensionContext): void {
  context.subscriptions.push(diagnosticCollection, outputChannel);
  context.subscriptions.push(
    vscode.commands.registerCommand('opalescent.selectBinary', async () => {
      await selectBinary();
    }),
    vscode.commands.registerCommand('opalescent.lintCurrentFile', async (uri?: vscode.Uri) => {
      const document = await documentFromCommand(uri);
      if (document) {
        await runDiagnostics(document, true);
      }
    }),
    vscode.commands.registerCommand('opalescent.formatDocument', async () => {
      await vscode.commands.executeCommand('editor.action.formatDocument');
    }),
    vscode.commands.registerCommand('opalescent.buildProject', async (uri?: vscode.Uri) => {
      const document = await documentFromCommand(uri);
      if (document) {
        await runBuildOrRun(document, 'build');
      }
    }),
    vscode.commands.registerCommand('opalescent.runProject', async (uri?: vscode.Uri) => {
      const document = await documentFromCommand(uri);
      if (document) {
        await runBuildOrRun(document, 'run');
      }
    })
  );

  context.subscriptions.push(
    vscode.languages.registerDocumentFormattingEditProvider(languageId, {
      provideDocumentFormattingEdits: async (document) => formatDocument(document)
    }),
    vscode.languages.registerCodeLensProvider(languageId, new OpalescentCodeLensProvider()),
    vscode.languages.registerDefinitionProvider(languageId, new OpalescentDefinitionProvider()),
    vscode.languages.registerImplementationProvider(languageId, new OpalescentImplementationProvider())
  );

  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((document) => {
      if (isOpalescentDocument(document) && config().get<boolean>('lintOnSave', true)) {
        scheduleDiagnostics(document, false);
      }
    }),
    vscode.workspace.onDidChangeTextDocument((event) => {
      if (isOpalescentDocument(event.document) && config().get<boolean>('lintOnChange', true)) {
        scheduleDiagnostics(event.document, false);
      }
    }),
    vscode.workspace.onDidCloseTextDocument((document) => {
      if (isOpalescentDocument(document)) {
        diagnosticCollection.delete(document.uri);
      }
    })
  );

  for (const document of vscode.workspace.textDocuments) {
    if (isOpalescentDocument(document)) {
      scheduleDiagnostics(document, false);
    }
  }
}

export function deactivate(): void {
  for (const timer of diagnosticTimers.values()) {
    clearTimeout(timer);
  }
  diagnosticTimers.clear();
}

function config(): vscode.WorkspaceConfiguration {
  return vscode.workspace.getConfiguration('opalescent');
}

async function documentFromCommand(uri?: vscode.Uri): Promise<vscode.TextDocument | undefined> {
  if (uri) {
    return vscode.workspace.openTextDocument(uri);
  }
  const document = vscode.window.activeTextEditor?.document;
  if (!document || !isOpalescentDocument(document)) {
    await vscode.window.showWarningMessage('Open an Opalescent file first.');
    return undefined;
  }
  return document;
}

function isOpalescentDocument(document: vscode.TextDocument): boolean {
  return document.languageId === languageId || isOpalescentFile(document.uri.fsPath);
}

function commandContextForDocument(document: vscode.TextDocument): OpalescentCommandContext {
  const filePath = document.uri.fsPath;
  const projectRoot = findProjectRoot(filePath, fs.existsSync);
  return { filePath, projectRoot };
}

function workspaceRoots(): string[] {
  return vscode.workspace.workspaceFolders?.map((folder) => folder.uri.fsPath) ?? [];
}

async function resolveBinary(promptUser: boolean): Promise<string | undefined> {
  if (resolvedBinary && (await binaryWorks(resolvedBinary))) {
    return resolvedBinary;
  }

  const configured = config().get<string>('binaryPath', '');
  for (const candidate of candidateBinaryPaths(workspaceRoots(), configured)) {
    if (await binaryWorks(candidate)) {
      resolvedBinary = candidate;
      return candidate;
    }
  }

  if (!promptUser) {
    return undefined;
  }

  const selection = await vscode.window.showWarningMessage(
    'Select the Opalescent compiler binary to enable formatting, diagnostics, build, and run.',
    'Select Binary'
  );
  if (selection !== 'Select Binary') {
    return undefined;
  }
  return selectBinary();
}

async function selectBinary(): Promise<string | undefined> {
  const selected = await vscode.window.showOpenDialog({
    canSelectFiles: true,
    canSelectFolders: false,
    canSelectMany: false,
    title: 'Select Opalescent compiler binary'
  });
  const picked = selected?.[0]?.fsPath;
  if (!picked) {
    return undefined;
  }
  await config().update('binaryPath', picked, vscode.ConfigurationTarget.Global);
  resolvedBinary = picked;
  return picked;
}

async function binaryWorks(binary: string): Promise<boolean> {
  return new Promise((resolve) => {
    childProcess.execFile(binary, ['help'], { timeout: 5000 }, (error) => {
      resolve(!error);
    });
  });
}

async function runCompiler(binary: string, args: string[], cwd?: string): Promise<CompilerResult> {
  outputChannel.appendLine(`$ ${binary} ${args.join(' ')}`);
  return new Promise((resolve) => {
    childProcess.execFile(binary, args, { cwd, maxBuffer: 16 * 1024 * 1024 }, (error, stdout, stderr) => {
      const code = typeof (error as childProcess.ExecFileException | null)?.code === 'number'
        ? Number((error as childProcess.ExecFileException).code)
        : 0;
      if (stderr) {
        outputChannel.appendLine(stderr);
      }
      resolve({ code, stdout, stderr });
    });
  });
}

function scheduleDiagnostics(document: vscode.TextDocument, promptUser: boolean): void {
  const key = document.uri.toString();
  const existing = diagnosticTimers.get(key);
  if (existing) {
    clearTimeout(existing);
  }
  const timer = setTimeout(() => {
    diagnosticTimers.delete(key);
    void runDiagnostics(document, promptUser);
  }, 350);
  diagnosticTimers.set(key, timer);
}

async function runDiagnostics(document: vscode.TextDocument, promptUser: boolean): Promise<void> {
  const binary = await resolveBinary(promptUser);
  if (!binary) {
    return;
  }

  const context = commandContextForDocument(document);
  const cwd = context.projectRoot ?? path.dirname(context.filePath);
  const result = await runCompiler(binary, checkArgsForContext(context), cwd);
  const jsonText = result.stdout.trim();
  if (!jsonText) {
    if (result.stderr) {
      outputChannel.appendLine(result.stderr);
    }
    return;
  }

  try {
    const report = parseOpalescentDiagnosticReport(jsonText);
    applyDiagnostics(report);
  } catch (error) {
    outputChannel.appendLine(`Failed to parse Opalescent diagnostics: ${String(error)}`);
    outputChannel.appendLine(jsonText);
  }
}

function applyDiagnostics(report: ReturnType<typeof parseOpalescentDiagnosticReport>): void {
  diagnosticCollection.clear();
  for (const [filePath, diagnostics] of diagnosticsByFile(report)) {
    diagnosticCollection.set(vscode.Uri.file(filePath), diagnostics.map(toVsCodeDiagnostic));
  }
}

function toVsCodeDiagnostic(diagnostic: OpalescentDiagnostic): vscode.Diagnostic {
  const range = new vscode.Range(
    diagnostic.range.start.line,
    diagnostic.range.start.character,
    diagnostic.range.end.line,
    diagnostic.range.end.character
  );
  const item = new vscode.Diagnostic(range, diagnostic.help ? `${diagnostic.message}\n${diagnostic.help}` : diagnostic.message, toSeverity(diagnostic.severity));
  item.source = `opalescent/${diagnostic.phase}`;
  item.code = diagnostic.code;
  return item;
}

function toSeverity(severity: OpalescentDiagnostic['severity']): vscode.DiagnosticSeverity {
  switch (severity) {
    case 'error':
      return vscode.DiagnosticSeverity.Error;
    case 'warning':
      return vscode.DiagnosticSeverity.Warning;
    case 'information':
      return vscode.DiagnosticSeverity.Information;
    case 'hint':
      return vscode.DiagnosticSeverity.Hint;
  }
}

async function formatDocument(document: vscode.TextDocument): Promise<vscode.TextEdit[]> {
  const binary = await resolveBinary(true);
  if (!binary) {
    return [];
  }

  const tempDir = await fsp.mkdtemp(path.join(os.tmpdir(), 'opalescent-vscode-'));
  const extension = document.uri.fsPath.endsWith('.types.op') ? '.types.op' : '.op';
  const sourcePath = path.join(tempDir, `buffer${extension}`);
  const outputPath = path.join(tempDir, `formatted${extension}`);
  try {
    await fsp.writeFile(sourcePath, document.getText(), 'utf8');
    const context = commandContextForDocument(document);
    const cwd = context.projectRoot ?? path.dirname(context.filePath);
    const result = await runCompiler(binary, formatArgs(sourcePath, outputPath), cwd);
    if (result.code !== 0) {
      outputChannel.appendLine(result.stderr || result.stdout);
      await vscode.window.showErrorMessage('Opalescent formatting failed. See the Opalescent output channel.');
      return [];
    }
    const formatted = await fsp.readFile(outputPath, 'utf8');
    return [vscode.TextEdit.replace(fullDocumentRange(document), formatted)];
  } finally {
    await fsp.rm(tempDir, { recursive: true, force: true });
  }
}

function fullDocumentRange(document: vscode.TextDocument): vscode.Range {
  const lastLine = document.lineAt(Math.max(document.lineCount - 1, 0));
  return new vscode.Range(new vscode.Position(0, 0), lastLine.range.end);
}

async function runBuildOrRun(document: vscode.TextDocument, mode: 'build' | 'run'): Promise<void> {
  const binary = await resolveBinary(true);
  if (!binary) {
    return;
  }
  const context = commandContextForDocument(document);
  const args = mode === 'build' ? buildArgsForContext(context) : runArgsForContext(context);
  const cwd = context.projectRoot ?? path.dirname(context.filePath);
  const terminal = vscode.window.createTerminal({ name: mode === 'build' ? 'Opalescent Build' : 'Opalescent Run', cwd });
  terminal.show();
  terminal.sendText([shellQuote(binary), ...args.map(shellQuote)].join(' '));
}

class OpalescentCodeLensProvider implements vscode.CodeLensProvider {
  provideCodeLenses(document: vscode.TextDocument): vscode.CodeLens[] {
    return findEntryLines(document.getText()).flatMap((line) => {
      const range = new vscode.Range(line, 0, line, 0);
      return [
        new vscode.CodeLens(range, {
          title: 'Build Opalescent Project',
          command: 'opalescent.buildProject',
          arguments: [document.uri]
        }),
        new vscode.CodeLens(range, {
          title: 'Run Opalescent Program',
          command: 'opalescent.runProject',
          arguments: [document.uri]
        })
      ];
    });
  }
}

class OpalescentDefinitionProvider implements vscode.DefinitionProvider {
  async provideDefinition(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.Location | undefined> {
    const word = wordAtPosition(document.getText(), position.line, position.character);
    if (!word) {
      return undefined;
    }
    const symbols = await collectProjectSymbols(document);
    const symbol = symbols.find((candidate) => candidate.name === word);
    return symbol ? symbolLocation(symbol) : undefined;
  }
}

class OpalescentImplementationProvider implements vscode.ImplementationProvider {
  async provideImplementation(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.Location[]> {
    const word = wordAtPosition(document.getText(), position.line, position.character);
    if (!word) {
      return [];
    }
    const symbols = await collectProjectSymbols(document);
    return symbols
      .filter((candidate) => candidate.name === word)
      .map(symbolLocation);
  }
}

async function collectProjectSymbols(document: vscode.TextDocument): Promise<OpalescentSymbol[]> {
  const currentFile = document.uri.fsPath;
  const projectRoot = findProjectRoot(currentFile, fs.existsSync);
  const files = projectRoot ? await collectOpalescentFiles(projectRoot) : [currentFile];
  const symbols: OpalescentSymbol[] = [];
  for (const file of files) {
    const source = path.resolve(file) === path.resolve(currentFile) ? document.getText() : await fsp.readFile(file, 'utf8');
    symbols.push(...collectSymbolsFromSource(source, file));
  }
  return symbols;
}

async function collectOpalescentFiles(root: string): Promise<string[]> {
  const files: string[] = [];
  async function walk(directory: string): Promise<void> {
    const entries = await fsp.readdir(directory, { withFileTypes: true });
    for (const entry of entries) {
      const entryPath = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === '.git' || entry.name === 'target' || entry.name === 'node_modules') {
          continue;
        }
        await walk(entryPath);
      } else if (entry.isFile() && isOpalescentFile(entryPath)) {
        files.push(entryPath);
      }
    }
  }
  await walk(root);
  return files.sort();
}

function symbolLocation(symbol: OpalescentSymbol): vscode.Location {
  return new vscode.Location(
    vscode.Uri.file(symbol.filePath),
    new vscode.Range(symbol.line, symbol.character, symbol.line, symbol.character + symbol.name.length)
  );
}
