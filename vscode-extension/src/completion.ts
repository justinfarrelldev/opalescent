import type { OpalescentSymbol, OpalescentSymbolKind } from './symbols.js';

import { moduleSpecifierForLocalImport } from './project.js';

export interface OpalescentCompletionImportEdit {
  character: number;
  line: number;
  text: string;
}

export interface OpalescentCompletionItem {
  autoImportEdit?: OpalescentCompletionImportEdit;
  documentation?: string;
  insertText: string;
  kind: OpalescentSymbolKind;
  name: string;
  sourceModule?: string;
  symbol: OpalescentSymbol;
}

export interface OpalescentCompletionRequest {
  currentFilePath: string;
  source: string;
  symbols: OpalescentSymbol[];
}

const sameFileCompletionKinds = new Set<OpalescentSymbolKind>([
  'entry',
  'error_set',
  'function',
  'let',
  'parameter',
  'type'
]);
const autoImportCompletionKinds = new Set<OpalescentSymbolKind>(['error_set', 'function', 'let', 'type']);
const typeOnlyImportKinds = new Set<OpalescentSymbolKind>(['error_set', 'type']);

/**
 * Builds completion candidates from project symbols, including auto-import edits for public cross-file declarations.
 * @param request Current file source, path, and project symbol index.
 * @returns Completion candidates suitable for conversion into VS Code completion items.
 */
export function completionItemsForSymbols(request: OpalescentCompletionRequest): OpalescentCompletionItem[] {
  const items: OpalescentCompletionItem[] = [];
  const seen = new Set<string>();

  for (const symbol of request.symbols) {
    if (!isCompletionSymbolForRequest(symbol, request.currentFilePath)) {
      continue;
    }

    const key = completionSymbolKey(symbol);
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);

    const autoImportEdit = autoImportEditForSymbol(request.source, request.currentFilePath, symbol);
    const sourceModule = sameFilePath(symbol.filePath, request.currentFilePath)
      ? undefined
      : moduleSpecifierForLocalImport(request.currentFilePath, symbol.filePath);
    items.push({
      autoImportEdit,
      documentation: symbol.documentation,
      insertText: symbol.name,
      kind: symbol.kind,
      name: symbol.name,
      sourceModule,
      symbol
    });
  }

  return items.sort(compareCompletionItems);
}

/**
 * Builds the import line required for a project symbol.
 * @param symbol Symbol to import.
 * @param currentFilePath File that will receive the import.
 * @returns Import line text, or undefined when the symbol is not importable.
 */
export function importLineForSymbol(symbol: OpalescentSymbol, currentFilePath: string): string | undefined {
  if (!isAutoImportableSymbol(symbol)) {
    return undefined;
  }

  const importKind = typeOnlyImportKinds.has(symbol.kind) ? 'import type' : 'import';
  const sourceModule = moduleSpecifierForLocalImport(currentFilePath, symbol.filePath);
  return `${importKind} ${symbol.name} from ${sourceModule}`;
}

/**
 * Builds an auto-import insertion edit for a symbol when one is needed.
 * @param source Current file source text.
 * @param currentFilePath File that will receive the import.
 * @param symbol Completion symbol.
 * @returns Import insertion edit, or undefined when no import should be added.
 */
export function autoImportEditForSymbol(
  source: string,
  currentFilePath: string,
  symbol: OpalescentSymbol
): OpalescentCompletionImportEdit | undefined {
  if (sameFilePath(symbol.filePath, currentFilePath) || !isAutoImportableSymbol(symbol)) {
    return undefined;
  }

  const importLine = importLineForSymbol(symbol, currentFilePath);
  if (!importLine) {
    return undefined;
  }

  const sourceModule = moduleSpecifierForLocalImport(currentFilePath, symbol.filePath);
  if (sourceAlreadyImportsSymbol(source, symbol.name, sourceModule)) {
    return undefined;
  }

  return insertImportLineEdit(source, importLine);
}

/**
 * Checks whether a symbol should be exposed in completion for this file.
 * @param symbol Candidate symbol.
 * @param currentFilePath File requesting completions.
 * @returns Whether the symbol should appear in completion.
 */
function isCompletionSymbolForRequest(symbol: OpalescentSymbol, currentFilePath: string): boolean {
  if (sameFilePath(symbol.filePath, currentFilePath)) {
    return sameFileCompletionKinds.has(symbol.kind);
  }

  return isAutoImportableSymbol(symbol);
}

/**
 * Checks whether a cross-file symbol can be imported automatically.
 * @param symbol Candidate symbol.
 * @returns Whether the symbol is a public file-scoped importable declaration.
 */
function isAutoImportableSymbol(symbol: OpalescentSymbol): boolean {
  return symbol.exported && symbol.scopeKind === 'file' && autoImportCompletionKinds.has(symbol.kind);
}

/**
 * Builds a stable de-duplication key for a symbol-backed completion.
 * @param symbol Symbol to key.
 * @returns Unique key for one declaration site.
 */
function completionSymbolKey(symbol: OpalescentSymbol): string {
  return [symbol.filePath, symbol.line, symbol.character, symbol.kind, symbol.name].join(':');
}

/**
 * Sorts local completions before auto-import completions, then by display name.
 * @param left Left completion.
 * @param right Right completion.
 * @returns Sort ordering.
 */
function compareCompletionItems(left: OpalescentCompletionItem, right: OpalescentCompletionItem): number {
  const leftImportPriority = left.autoImportEdit ? 1 : 0;
  const rightImportPriority = right.autoImportEdit ? 1 : 0;
  if (leftImportPriority !== rightImportPriority) {
    return leftImportPriority - rightImportPriority;
  }

  const nameCompare = left.name.localeCompare(right.name);
  if (nameCompare !== 0) {
    return nameCompare;
  }

  return left.sourceModule?.localeCompare(right.sourceModule ?? '') ?? (right.sourceModule ? -1 : 0);
}

/**
 * Finds the import insertion location for a new top-level import line.
 * @param source Current source text.
 * @param importLine Complete import line without trailing newline.
 * @returns Insertion edit that preserves a contiguous top import block.
 */
function insertImportLineEdit(source: string, importLine: string): OpalescentCompletionImportEdit {
  if (source.length === 0) {
    return { character: 0, line: 0, text: `${importLine}\n` };
  }

  const lines = source.split('\n');
  const lastTopImportLine = lastContiguousTopImportLine(lines);
  if (lastTopImportLine >= 0) {
    return { character: 0, line: lastTopImportLine + 1, text: `${importLine}\n` };
  }

  return { character: 0, line: 0, text: `${importLine}\n\n` };
}

/**
 * Finds the last import line in the initial import block.
 * @param lines Source lines.
 * @returns Last import line index, or -1 when no top import block exists.
 */
function lastContiguousTopImportLine(lines: string[]): number {
  let lastImportLine = -1;
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex] ?? '';
    const trimmed = line.trim();
    if (isImportLine(line)) {
      lastImportLine = lineIndex;
      continue;
    }
    if (trimmed === '' && lastImportLine >= 0) {
      break;
    }
    if (trimmed === '') {
      continue;
    }
    break;
  }
  return lastImportLine;
}

/**
 * Checks whether a source file already imports a symbol from a module.
 * @param source Current source text.
 * @param name Symbol name.
 * @param sourceModule Module specifier expected for the import.
 * @returns Whether the import already exists.
 */
function sourceAlreadyImportsSymbol(source: string, name: string, sourceModule: string): boolean {
  for (const line of source.split('\n')) {
    const parsed = parseImportLine(line);
    if (!parsed || parsed.sourceModule !== sourceModule) {
      continue;
    }
    if (parsed.names.includes(name)) {
      return true;
    }
  }
  return false;
}

/**
 * Parses an Opalescent named import line.
 * @param line Source line.
 * @returns Imported names and module specifier, or undefined for non-import lines.
 */
function parseImportLine(line: string): { names: string[]; sourceModule: string } | undefined {
  const match = /^\s*import\s+(?:type\s+)?(.+?)\s+from\s+(?:'([^']+)'|"([^"]+)"|([^\s#]+))/.exec(line);
  const namesText = match?.[1];
  const sourceModule = match?.[2] ?? match?.[3] ?? match?.[4];
  if (!namesText || !sourceModule) {
    return undefined;
  }

  const names = namesText
    .split(',')
    .map((name) => name.trim().split(/\s+as\s+/)[0]?.trim() ?? '')
    .filter((name) => name.length > 0);
  return { names, sourceModule };
}

/**
 * Checks whether a line is an Opalescent named import.
 * @param line Source line.
 * @returns Whether the line is an import declaration.
 */
function isImportLine(line: string): boolean {
  return /^\s*import\b/.test(line);
}

/**
 * Checks whether two paths identify the same file after separator normalization.
 * @param left Left file path.
 * @param right Right file path.
 * @returns Whether the paths match.
 */
function sameFilePath(left: string, right: string): boolean {
  return left.replace(/\\/g, '/') === right.replace(/\\/g, '/');
}
