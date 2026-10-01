import * as childProcess from 'node:child_process';
import * as fs from 'node:fs';
import * as fsp from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';

import { candidateBinaryPaths, shellQuote } from './binary.js';
import { type OpalescentCommandContext, buildArgsForContext, checkArgsForContext, formatArgs, runArgsForContext } from './cli.js';
import { type OpalescentDiagnostic, diagnosticsByFile, parseOpalescentDiagnosticReport } from './diagnostics.js';
import { collectLocalLintDiagnostics } from './lint.js';
import { findProjectRoot, isOpalescentFile, resolveLocalImportPath } from './project.js';
import {
  type OpalescentLookupPosition,
  type OpalescentSymbol,
  collectSymbolsFromSource,
  definitionSymbolAtPosition,
  definitionSymbolsForWord,
  findEntryLines,
  hoverSymbolForWord,
  importTargetAtPosition,
  referenceTargetsForSymbol,
  wordAtPosition
} from './symbols.js';

const languageId = 'opalescent';
const diagnosticCollection = vscode.languages.createDiagnosticCollection('opalescent');
const outputChannel = vscode.window.createOutputChannel('Opalescent');
const diagnosticTimers = new Map<string, NodeJS.Timeout>();
let resolvedBinary: string | undefined;

interface CompilerResult {
  code: number;
  stderr: string;
  stdout: string;
}

/**
 * Registers extension commands, editor providers, and diagnostic hooks.
 * @param context VS Code extension context used to manage subscriptions.
 */
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
    vscode.languages.registerImplementationProvider(languageId, new OpalescentImplementationProvider()),
    vscode.languages.registerHoverProvider(languageId, new OpalescentHoverProvider())
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

/**
 * Clears pending diagnostic timers during extension shutdown.
 */
export function deactivate(): void {
  for (const timer of diagnosticTimers.values()) {
    clearTimeout(timer);
  }
  diagnosticTimers.clear();
}

/**
 * Reads the Opalescent workspace configuration section.
 * @returns Current Opalescent workspace configuration.
 */
function config(): vscode.WorkspaceConfiguration {
  return vscode.workspace.getConfiguration('opalescent');
}

/**
 * Resolves a command argument or active editor into an Opalescent document.
 * @param uri Optional URI supplied by a VS Code command invocation.
 * @returns The requested document, or undefined when no Opalescent file is active.
 */
async function documentFromCommand(uri?: vscode.Uri): Promise<undefined | vscode.TextDocument> {
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

/**
 * Checks whether a VS Code document should be treated as Opalescent source.
 * @param document Document to inspect.
 * @returns Whether the document uses the Opalescent language or file extension.
 */
function isOpalescentDocument(document: vscode.TextDocument): boolean {
  return document.languageId === languageId || isOpalescentFile(document.uri.fsPath);
}

/**
 * Builds compiler command context for a document.
 * @param document Document that a compiler command will target.
 * @returns File path and nearest project root for command generation.
 */
function commandContextForDocument(document: vscode.TextDocument): OpalescentCommandContext {
  const filePath = document.uri.fsPath;
  const projectRoot = findProjectRoot(filePath, fs.existsSync);
  return { filePath, projectRoot };
}

/**
 * Gets workspace folder paths currently open in VS Code.
 * @returns Workspace root paths.
 */
function workspaceRoots(): string[] {
  return vscode.workspace.workspaceFolders?.map((folder) => folder.uri.fsPath) ?? [];
}

/**
 * Finds a usable Opalescent compiler binary.
 * @param promptUser Whether to prompt the user when auto-detection fails.
 * @returns A compiler path, or undefined when none is available.
 */
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

/**
 * Prompts the user to choose and persist a compiler binary path.
 * @returns The selected compiler path, or undefined when cancelled.
 */
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

/**
 * Checks whether a candidate binary responds to the compiler help command.
 * @param binary Candidate compiler path or command name.
 * @returns Whether the candidate can be executed successfully.
 */
async function binaryWorks(binary: string): Promise<boolean> {
  return new Promise((resolve) => {
    childProcess.execFile(binary, ['help'], { timeout: 5000 }, (error) => {
      resolve(!error);
    });
  });
}

/**
 * Executes the compiler and captures its process output.
 * @param binary Compiler binary path or command name.
 * @param args Arguments passed to the compiler.
 * @param cwd Optional working directory for the process.
 * @returns Compiler exit code, stdout, and stderr.
 */
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
      resolve({ code, stderr, stdout });
    });
  });
}

/**
 * Debounces compiler-backed diagnostics for a document.
 * @param document Document to diagnose.
 * @param promptUser Whether diagnostics may prompt for a compiler binary.
 */
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

/**
 * Runs compiler diagnostics and applies parsed VS Code diagnostics.
 * @param document Document to diagnose.
 * @param promptUser Whether diagnostics may prompt for a compiler binary.
 */
async function runDiagnostics(document: vscode.TextDocument, promptUser: boolean): Promise<void> {
  const localDiagnostics = collectLocalLintDiagnostics(document.getText(), document.uri.fsPath);
  const binary = await resolveBinary(promptUser);
  if (!binary) {
    applyDiagnostics({ diagnostics: localDiagnostics, success: localDiagnostics.length === 0 });
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
    applyDiagnostics({ diagnostics: localDiagnostics, success: localDiagnostics.length === 0 });
    return;
  }

  try {
    const report = parseOpalescentDiagnosticReport(jsonText);
    applyDiagnostics({ ...report, diagnostics: mergeDiagnostics(report.diagnostics, localDiagnostics) });
  } catch (error) {
    outputChannel.appendLine(`Failed to parse Opalescent diagnostics: ${String(error)}`);
    outputChannel.appendLine(jsonText);
    applyDiagnostics({ diagnostics: localDiagnostics, success: localDiagnostics.length === 0 });
  }
}

/**
 * Merges compiler and local diagnostics while avoiding exact duplicates.
 * @param primary Diagnostics from the compiler.
 * @param secondary Diagnostics from local editor linting.
 * @returns Combined diagnostics.
 */
function mergeDiagnostics(primary: OpalescentDiagnostic[], secondary: OpalescentDiagnostic[]): OpalescentDiagnostic[] {
  const seen = new Set(primary.map(diagnosticKey));
  const merged = [...primary];
  for (const diagnostic of secondary) {
    const key = diagnosticKey(diagnostic);
    if (!seen.has(key)) {
      seen.add(key);
      merged.push(diagnostic);
    }
  }
  return merged;
}

/**
 * Builds a stable key for duplicate diagnostic suppression.
 * @param diagnostic Diagnostic to key.
 * @returns Stable key string.
 */
function diagnosticKey(diagnostic: OpalescentDiagnostic): string {
  return [
    diagnostic.source_path,
    diagnostic.code ?? diagnostic.message,
    diagnostic.range.start.line,
    diagnostic.range.start.character,
    diagnostic.range.end.line,
    diagnostic.range.end.character
  ].join(':');
}

/**
 * Replaces the extension diagnostic collection with compiler report contents.
 * @param report Parsed compiler diagnostic report.
 */
function applyDiagnostics(report: ReturnType<typeof parseOpalescentDiagnosticReport>): void {
  diagnosticCollection.clear();
  for (const [filePath, diagnostics] of diagnosticsByFile(report)) {
    diagnosticCollection.set(vscode.Uri.file(filePath), diagnostics.map(toVsCodeDiagnostic));
  }
}

/**
 * Converts a compiler diagnostic into a VS Code diagnostic.
 * @param diagnostic Compiler diagnostic to convert.
 * @returns VS Code diagnostic for display in the editor.
 */
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

/**
 * Maps compiler severity labels to VS Code severity values.
 * @param severity Compiler severity label.
 * @returns Matching VS Code diagnostic severity.
 */
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

/**
 * Formats a document through the Opalescent compiler formatter.
 * @param document Document to format.
 * @returns Replacement edits containing formatted text, or an empty list on failure.
 */
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
    await fsp.rm(tempDir, { force: true, recursive: true });
  }
}

/**
 * Builds a range covering the full contents of a document.
 * @param document Document whose full range is needed.
 * @returns Range from the first character through the final line ending position.
 */
function fullDocumentRange(document: vscode.TextDocument): vscode.Range {
  const lastLine = document.lineAt(Math.max(document.lineCount - 1, 0));
  return new vscode.Range(new vscode.Position(0, 0), lastLine.range.end);
}

/**
 * Opens a VS Code terminal and starts a build or run command.
 * @param document Document used to determine project context.
 * @param mode Command mode to execute.
 */
async function runBuildOrRun(document: vscode.TextDocument, mode: 'build' | 'run'): Promise<void> {
  const binary = await resolveBinary(true);
  if (!binary) {
    return;
  }
  const context = commandContextForDocument(document);
  const args = mode === 'build' ? buildArgsForContext(context) : runArgsForContext(context);
  const cwd = context.projectRoot ?? path.dirname(context.filePath);
  const terminal = vscode.window.createTerminal({ cwd, name: mode === 'build' ? 'Opalescent Build' : 'Opalescent Run' });
  terminal.show();
  terminal.sendText([shellQuote(binary), ...args.map(shellQuote)].join(' '));
}

class OpalescentCodeLensProvider implements vscode.CodeLensProvider {
  /**
   * Creates build and run code lenses for Opalescent entry declarations.
   * @param document Document to inspect for entry declarations.
   * @returns Code lenses for each entry declaration.
   */
  provideCodeLenses(document: vscode.TextDocument): vscode.CodeLens[] {
    return findEntryLines(document.getText()).flatMap((line) => {
      const range = new vscode.Range(line, 0, line, 0);
      return [
        new vscode.CodeLens(range, {
          arguments: [document.uri],
          command: 'opalescent.buildProject',
          title: 'Build Opalescent Project'
        }),
        new vscode.CodeLens(range, {
          arguments: [document.uri],
          command: 'opalescent.runProject',
          title: 'Run Opalescent Program'
        })
      ];
    });
  }
}

class OpalescentDefinitionProvider implements vscode.DefinitionProvider {
  /**
   * Resolves a symbol definition location for the word under the cursor.
   * @param document Document containing the lookup position.
   * @param position Position whose word should be resolved.
   * @returns Definition location or implementation locations, or undefined when no symbol matches.
   */
  async provideDefinition(document: vscode.TextDocument, position: vscode.Position): Promise<undefined | vscode.Definition> {
    const importLocation = importLocationForPosition(document, position);
    if (importLocation) {
      return importLocation;
    }

    const word = wordAtPosition(document.getText(), position.line, position.character);
    if (!word) {
      return undefined;
    }
    const symbols = await collectProjectSymbols(document);
    const lookupPosition = lookupPositionForDocument(document, position);
    const definitionAtCursor = definitionSymbolAtPosition(symbols, word, lookupPosition);
    if (definitionAtCursor?.scopeKind === 'function') {
      const references = referenceTargetsForSymbol(document.getText(), definitionAtCursor);
      if (references.length > 0) {
        return references.map(symbolLocation);
      }
    }

    const definitions = definitionSymbolsForWord(symbols, word, lookupPosition);
    if (definitions.length === 0) {
      return undefined;
    }
    return definitions.map(symbolLocation);
  }
}

class OpalescentImplementationProvider implements vscode.ImplementationProvider {
  /**
   * Resolves implementation locations for the word under the cursor.
   * @param document Document containing the lookup position.
   * @param position Position whose word should be resolved.
   * @returns Matching implementation locations.
   */
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

class OpalescentHoverProvider implements vscode.HoverProvider {
  /**
   * Resolves hover documentation for the word under the cursor.
   * @param document Document containing the lookup position.
   * @param position Position whose word should be resolved.
   * @returns Hover text for documented symbols, or undefined.
   */
  async provideHover(document: vscode.TextDocument, position: vscode.Position): Promise<undefined | vscode.Hover> {
    const word = wordAtPosition(document.getText(), position.line, position.character);
    if (!word) {
      return undefined;
    }

    const symbols = await collectProjectSymbols(document);
    const symbol = hoverSymbolForWord(symbols, word, lookupPositionForDocument(document, position));
    if (!symbol?.documentation) {
      return undefined;
    }

    const contents = new vscode.MarkdownString(symbol.documentation);
    contents.isTrusted = false;
    return new vscode.Hover(contents);
  }
}

/**
 * Collects navigation symbols from the current project or current file.
 * @param document Current document whose project should be scanned.
 * @returns Symbols available for navigation.
 */
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

/**
 * Recursively collects Opalescent files under a project root.
 * @param root Project root to scan.
 * @returns Sorted Opalescent source file paths.
 */
async function collectOpalescentFiles(root: string): Promise<string[]> {
  const files: string[] = [];
  /**
   * Walks a directory tree, appending Opalescent files to the outer list.
   * @param directory Directory to scan.
   */
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

/**
 * Resolves a source import specifier under the cursor to its target file.
 * @param document Document that owns the import.
 * @param position VS Code editor position.
 * @returns Import target location, or undefined.
 */
function importLocationForPosition(document: vscode.TextDocument, position: vscode.Position): undefined | vscode.Location {
  const importTarget = importTargetAtPosition(document.getText(), position.line, position.character);
  if (!importTarget) {
    return undefined;
  }

  const targetPath = resolveLocalImportPath(document.uri.fsPath, importTarget.moduleSpecifier, fs.existsSync);
  if (!targetPath) {
    return undefined;
  }

  return new vscode.Location(vscode.Uri.file(targetPath), new vscode.Position(0, 0));
}

/**
 * Converts a VS Code position to the pure symbol lookup position shape.
 * @param document Document that owns the position.
 * @param position VS Code editor position.
 * @returns Lookup position used by symbol helpers.
 */
function lookupPositionForDocument(document: vscode.TextDocument, position: vscode.Position): OpalescentLookupPosition {
  return { character: position.character, filePath: document.uri.fsPath, line: position.line };
}

/**
 * Converts a symbol into a VS Code location.
 * @param symbol Symbol to convert.
 * @returns Location spanning the symbol name.
 */
function symbolLocation(symbol: OpalescentSymbol): vscode.Location {
  return new vscode.Location(vscode.Uri.file(symbol.filePath), symbolRange(symbol));
}

/**
 * Converts a symbol into a VS Code range.
 * @param symbol Symbol to convert.
 * @returns Range spanning the symbol name.
 */
function symbolRange(symbol: OpalescentSymbol): vscode.Range {
  return new vscode.Range(symbol.line, symbol.character, symbol.line, symbol.character + symbol.name.length);
}
