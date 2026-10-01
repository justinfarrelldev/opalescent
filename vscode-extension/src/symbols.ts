export type OpalescentSymbolKind = 'entry' | 'error_set' | 'function' | 'let' | 'parameter' | 'type';
export type OpalescentSymbolScopeKind = 'file' | 'function';

export interface OpalescentLookupPosition {
  character: number;
  filePath: string;
  line: number;
}

export interface OpalescentImportTarget {
  character: number;
  length: number;
  line: number;
  moduleSpecifier: string;
}

export interface OpalescentSymbol {
  character: number;
  documentation?: string;
  exported: boolean;
  filePath: string;
  kind: OpalescentSymbolKind;
  line: number;
  name: string;
  scopeEndLine: number;
  scopeKind: OpalescentSymbolScopeKind;
  scopeStartLine: number;
}

interface FunctionScope {
  endLine: number;
  name: string;
  signatureEndLine: number;
  startLine: number;
}

interface OffsetPosition {
  character: number;
  line: number;
}

interface SourceTextWithOffsets {
  lineOffsets: number[];
  startLine: number;
  text: string;
}

const identifierPattern = '[A-Za-z_][A-Za-z0-9_]*';
const functionLetPattern = new RegExp(`^(public\\s+)?let\\s+(mutable\\s+)?(${identifierPattern})\\s*=\\s*f\\b`);
const letPattern = new RegExp(`^\\s*(public\\s+)?let\\s+(mutable\\s+)?(${identifierPattern})\\b(?!\\s*=\\s*f\\b)`);
const entryPattern = new RegExp(`^(public\\s+)?entry\\s+(${identifierPattern})\\s*=\\s*f\\b`);
const typePattern = new RegExp(`^(public\\s+)?type\\s+([A-Z][A-Za-z0-9_]*)\\b`);
const errorSetPattern = new RegExp(`^(public\\s+)?error\\s+set\\s+([A-Z][A-Za-z0-9_]*)\\b`);
const topLevelDeclarationPattern = /^(?:public\s+)?(?:entry|error\s+set|let|type)\b/;

/**
 * Collects declarations that are useful for editor navigation.
 * @param source Opalescent source text to scan.
 * @param filePath Path associated with the source text.
 * @returns Symbols discovered in source order.
 */
export function collectSymbolsFromSource(source: string, filePath: string): OpalescentSymbol[] {
  const symbols: OpalescentSymbol[] = [];
  const lines = source.split('\n');
  const topLevelLines = collectTopLevelDeclarationLines(lines);
  const functionScopes: FunctionScope[] = [];

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex] ?? '';
    if (!topLevelDeclarationPattern.test(line)) {
      continue;
    }

    const functionMatch = line.match(functionLetPattern);
    if (functionMatch?.[3]) {
      const scope = functionScopeForDeclaration(lines, topLevelLines, lineIndex, functionMatch[3]);
      functionScopes.push(scope);
      symbols.push(fileSymbolFromMatch(lines, lineIndex, filePath, functionMatch[3], 'function', Boolean(functionMatch[1])));
      symbols.push(...parameterSymbolsForFunction(lines, filePath, scope));
      continue;
    }

    const typeMatch = line.match(typePattern);
    if (typeMatch?.[2]) {
      symbols.push(fileSymbolFromMatch(lines, lineIndex, filePath, typeMatch[2], 'type', Boolean(typeMatch[1])));
      continue;
    }

    const errorSetMatch = line.match(errorSetPattern);
    if (errorSetMatch?.[2]) {
      symbols.push(fileSymbolFromMatch(lines, lineIndex, filePath, errorSetMatch[2], 'error_set', Boolean(errorSetMatch[1])));
      continue;
    }

    const entryMatch = line.match(entryPattern);
    if (entryMatch?.[2]) {
      const scope = functionScopeForDeclaration(lines, topLevelLines, lineIndex, entryMatch[2]);
      functionScopes.push(scope);
      symbols.push(fileSymbolFromMatch(lines, lineIndex, filePath, entryMatch[2], 'entry', true));
      symbols.push(...parameterSymbolsForFunction(lines, filePath, scope));
      continue;
    }

    const letMatch = line.match(letPattern);
    if (letMatch?.[3]) {
      symbols.push(fileSymbolFromMatch(lines, lineIndex, filePath, letMatch[3], 'let', Boolean(letMatch[1])));
    }
  }

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex] ?? '';
    if (topLevelDeclarationPattern.test(line)) {
      continue;
    }

    const letMatch = line.match(letPattern);
    if (!letMatch?.[3]) {
      continue;
    }

    const containingScope = functionScopes.find((scope) => scope.startLine <= lineIndex && lineIndex <= scope.endLine);
    if (!containingScope) {
      continue;
    }

    symbols.push(functionScopedSymbolFromMatch(line, lineIndex, filePath, letMatch[3], 'let', containingScope));
  }

  return symbols.sort(compareSymbolsInSourceOrder);
}

/**
 * Resolves the symbols that go-to-definition should surface for a word.
 * @param symbols Project symbols available for lookup.
 * @param word Identifier under the cursor.
 * @param position Cursor position in the requesting document.
 * @returns Matching definition symbols, or same-name implementations when already on a definition site.
 */
export function definitionSymbolsForWord(
  symbols: OpalescentSymbol[],
  word: string,
  position: OpalescentLookupPosition
): OpalescentSymbol[] {
  const sameNameSymbols = implementationSymbolsForWord(symbols, word);
  const definitionAtCursor = sameNameSymbols.find((symbol) => isPositionOnSymbolDefinition(symbol, position));
  if (definitionAtCursor) {
    return sameNameSymbols.filter((symbol) => sharesImplementationDomain(definitionAtCursor, symbol));
  }

  const bestSymbol = bestDefinitionSymbol(sameNameSymbols, position);
  return bestSymbol ? [bestSymbol] : [];
}

/**
 * Finds source lines containing entry declarations.
 * @param source Opalescent source text to scan.
 * @returns Zero-based line numbers for entry declarations.
 */
export function findEntryLines(source: string): number[] {
  const lines = source.split('\n');
  const entries: number[] = [];
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    if (entryPattern.test(lines[lineIndex] ?? '')) {
      entries.push(lineIndex);
    }
  }
  return entries;
}

/**
 * Resolves the most useful documented symbol for hover at a word position.
 * @param symbols Project symbols available for lookup.
 * @param word Identifier under the cursor.
 * @param position Cursor position in the requesting document.
 * @returns A documented symbol for hover, or undefined when none is available.
 */
export function hoverSymbolForWord(
  symbols: OpalescentSymbol[],
  word: string,
  position: OpalescentLookupPosition
): OpalescentSymbol | undefined {
  const sameNameSymbols = implementationSymbolsForWord(symbols, word);
  const definitionAtCursor = sameNameSymbols.find((symbol) => isPositionOnSymbolDefinition(symbol, position));
  if (definitionAtCursor?.documentation) {
    return definitionAtCursor;
  }

  return definitionSymbolsForWord(symbols, word, position).find((symbol) => Boolean(symbol.documentation));
}

/**
 * Finds a definition symbol exactly under a lookup position.
 * @param symbols Project symbols available for lookup.
 * @param word Identifier under the cursor.
 * @param position Cursor position in the requesting document.
 * @returns The definition symbol at the position, or undefined.
 */
export function definitionSymbolAtPosition(
  symbols: OpalescentSymbol[],
  word: string,
  position: OpalescentLookupPosition
): OpalescentSymbol | undefined {
  return implementationSymbolsForWord(symbols, word).find((symbol) => isPositionOnSymbolDefinition(symbol, position));
}

/**
 * Finds an import module specifier under the cursor.
 * @param source Source text to inspect.
 * @param line Zero-based line index.
 * @param character Zero-based character index.
 * @returns Import target range and specifier, or undefined.
 */
export function importTargetAtPosition(source: string, line: number, character: number): OpalescentImportTarget | undefined {
  const lineText = source.split('\n')[line];
  if (!lineText) {
    return undefined;
  }

  const match = /\bfrom\s+(?:'([^']+)'|"([^"]+)"|([^\s#]+))/.exec(lineText);
  const moduleSpecifier = match?.[1] ?? match?.[2] ?? match?.[3];
  if (!match || !moduleSpecifier) {
    return undefined;
  }

  const specifierOffset = match.index + match[0].lastIndexOf(moduleSpecifier);
  const specifierEnd = specifierOffset + moduleSpecifier.length;
  if (character < specifierOffset || character > specifierEnd) {
    return undefined;
  }

  return { character: specifierOffset, length: moduleSpecifier.length, line, moduleSpecifier };
}

/**
 * Finds references to a symbol within its source scope.
 * @param source Source text containing the symbol.
 * @param symbol Definition symbol whose references should be found.
 * @returns Reference-like symbols in source order, excluding the definition site.
 */
export function referenceTargetsForSymbol(source: string, symbol: OpalescentSymbol): OpalescentSymbol[] {
  const references: OpalescentSymbol[] = [];
  const lines = source.split('\n');
  const wordPattern = new RegExp(`\\b${escapeRegExp(symbol.name)}\\b`, 'g');
  for (let lineIndex = symbol.scopeStartLine; lineIndex <= symbol.scopeEndLine && lineIndex < lines.length; lineIndex += 1) {
    const lineText = lines[lineIndex] ?? '';
    let match = wordPattern.exec(lineText);
    while (match) {
      if (!(lineIndex === symbol.line && match.index === symbol.character)) {
        references.push({ ...symbol, character: match.index, exported: false, line: lineIndex });
      }
      match = wordPattern.exec(lineText);
    }
  }
  return references;
}

/**
 * Extracts an identifier near a zero-based editor position.
 * @param source Source text to inspect.
 * @param line Zero-based line index.
 * @param character Zero-based character index.
 * @returns The identifier at or before the position, or undefined.
 */
export function wordAtPosition(source: string, line: number, character: number): string | undefined {
  const lines = source.split('\n');
  const lineText = lines[line];
  if (lineText === undefined || lineText.length === 0) {
    return undefined;
  }

  let cursor = Math.min(Math.max(character, 0), Math.max(lineText.length - 1, 0));
  while (cursor > 0 && !isWordCharacter(lineText[cursor] ?? '')) {
    cursor -= 1;
  }
  if (!isWordCharacter(lineText[cursor] ?? '')) {
    return undefined;
  }

  let start = cursor;
  while (start > 0 && isWordCharacter(lineText[start - 1] ?? '')) {
    start -= 1;
  }

  let end = cursor + 1;
  while (end < lineText.length && isWordCharacter(lineText[end] ?? '')) {
    end += 1;
  }

  return lineText.slice(start, end);
}

/**
 * Chooses the best definition from matching symbols for a use position.
 * @param symbols Same-name candidate symbols.
 * @param position Lookup position.
 * @returns The best scoped definition, or undefined when no candidates exist.
 */
function bestDefinitionSymbol(symbols: OpalescentSymbol[], position: OpalescentLookupPosition): OpalescentSymbol | undefined {
  const sameFileFunctionSymbols = symbols
    .filter((symbol) => symbol.scopeKind === 'function')
    .filter((symbol) => sameFilePath(symbol.filePath, position.filePath))
    .filter((symbol) => symbol.scopeStartLine <= position.line && position.line <= symbol.scopeEndLine)
    .filter((symbol) => isDeclaredBeforePosition(symbol, position))
    .sort(compareScopedDefinitionPriority);
  if (sameFileFunctionSymbols[0]) {
    return sameFileFunctionSymbols[0];
  }

  const sameFileSymbols = symbols
    .filter((symbol) => sameFilePath(symbol.filePath, position.filePath))
    .filter((symbol) => symbol.scopeKind === 'file')
    .sort(compareSymbolsInSourceOrder);
  if (sameFileSymbols[0]) {
    return sameFileSymbols[0];
  }

  const exportedSymbols = symbols.filter((symbol) => symbol.exported && symbol.scopeKind === 'file').sort(compareSymbolsInSourceOrder);
  return exportedSymbols[0] ?? symbols.sort(compareSymbolsInSourceOrder)[0];
}

/**
 * Collects top-level declaration line numbers for scope boundary calculations.
 * @param lines Source lines.
 * @returns Lines that begin top-level declarations.
 */
function collectTopLevelDeclarationLines(lines: string[]): number[] {
  const declarations: number[] = [];
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    if (topLevelDeclarationPattern.test(lines[lineIndex] ?? '')) {
      declarations.push(lineIndex);
    }
  }
  return declarations;
}

/**
 * Compares symbols by path and source position.
 * @param left Left symbol.
 * @param right Right symbol.
 * @returns Sort ordering.
 */
function compareSymbolsInSourceOrder(left: OpalescentSymbol, right: OpalescentSymbol): number {
  const fileCompare = normalizeFilePath(left.filePath).localeCompare(normalizeFilePath(right.filePath));
  if (fileCompare !== 0) {
    return fileCompare;
  }
  if (left.line !== right.line) {
    return left.line - right.line;
  }
  if (left.character !== right.character) {
    return left.character - right.character;
  }
  return left.name.localeCompare(right.name);
}

/**
 * Prioritizes scoped definitions by narrowness and nearest preceding declaration.
 * @param left Left symbol.
 * @param right Right symbol.
 * @returns Sort ordering.
 */
function compareScopedDefinitionPriority(left: OpalescentSymbol, right: OpalescentSymbol): number {
  const leftSpan = left.scopeEndLine - left.scopeStartLine;
  const rightSpan = right.scopeEndLine - right.scopeStartLine;
  if (leftSpan !== rightSpan) {
    return leftSpan - rightSpan;
  }
  if (left.line !== right.line) {
    return right.line - left.line;
  }
  if (left.character !== right.character) {
    return right.character - left.character;
  }
  return left.name.localeCompare(right.name);
}

/**
 * Converts file paths to a stable comparison form.
 * @param filePath File path to normalize.
 * @returns Normalized file path.
 */
function normalizeFilePath(filePath: string): string {
  return filePath.replace(/\\/g, '/');
}

/**
 * Extracts a documentation comment immediately above a declaration line.
 * @param lines Source lines.
 * @param lineIndex Declaration line index.
 * @returns Cleaned documentation text, or undefined.
 */
function documentationBeforeLine(lines: string[], lineIndex: number): string | undefined {
  let cursor = lineIndex - 1;
  while (cursor >= 0 && (lines[cursor] ?? '').trim() === '') {
    cursor -= 1;
  }

  if ((lines[cursor] ?? '').trim() !== '##') {
    return undefined;
  }

  cursor -= 1;
  const documentationLines: string[] = [];
  while (cursor >= 0 && (lines[cursor] ?? '').trim() !== '##') {
    documentationLines.unshift(stripDocumentationIndent(lines[cursor] ?? ''));
    cursor -= 1;
  }

  if (cursor < 0) {
    return undefined;
  }

  const documentation = cleanDocumentationText(documentationLines.join('\n'));
  return documentation.length > 0 ? documentation : undefined;
}

/**
 * Finds same-name symbols that implementation lookup can show.
 * @param symbols Project symbols available for lookup.
 * @param word Identifier under the cursor.
 * @returns Symbols with the requested name.
 */
function implementationSymbolsForWord(symbols: OpalescentSymbol[], word: string): OpalescentSymbol[] {
  return symbols.filter((symbol) => symbol.name === word).sort(compareSymbolsInSourceOrder);
}

/**
 * Determines whether a symbol has already been declared at a lookup position.
 * @param symbol Candidate symbol.
 * @param position Lookup position.
 * @returns Whether the symbol can be visible from the position.
 */
function isDeclaredBeforePosition(symbol: OpalescentSymbol, position: OpalescentLookupPosition): boolean {
  return symbol.line < position.line || (symbol.line === position.line && symbol.character <= position.character);
}

/**
 * Checks whether a lookup position is on a symbol's declaration range.
 * @param symbol Candidate symbol.
 * @param position Lookup position.
 * @returns Whether the cursor is on the symbol definition.
 */
function isPositionOnSymbolDefinition(symbol: OpalescentSymbol, position: OpalescentLookupPosition): boolean {
  return (
    sameFilePath(symbol.filePath, position.filePath) &&
    symbol.line === position.line &&
    symbol.character <= position.character &&
    position.character <= symbol.character + symbol.name.length
  );
}

/**
 * Converts a top-level declaration into a file-scoped symbol.
 * @param lines Source lines.
 * @param lineIndex Declaration line index.
 * @param filePath Source file path.
 * @param name Symbol name.
 * @param kind Symbol kind.
 * @param exported Whether the declaration is public.
 * @returns A navigation symbol for the declaration.
 */
function fileSymbolFromMatch(
  lines: string[],
  lineIndex: number,
  filePath: string,
  name: string,
  kind: OpalescentSymbolKind,
  exported: boolean
): OpalescentSymbol {
  const line = lines[lineIndex] ?? '';
  return {
    character: line.indexOf(name),
    documentation: documentationBeforeLine(lines, lineIndex),
    exported,
    filePath,
    kind,
    line: lineIndex,
    name,
    scopeEndLine: Math.max(lines.length - 1, 0),
    scopeKind: 'file',
    scopeStartLine: 0
  };
}

/**
 * Converts a local declaration into a function-scoped symbol.
 * @param line Source line.
 * @param lineIndex Declaration line index.
 * @param filePath Source file path.
 * @param name Symbol name.
 * @param kind Symbol kind.
 * @param scope Function scope containing the symbol.
 * @returns A navigation symbol for the local declaration.
 */
function functionScopedSymbolFromMatch(
  line: string,
  lineIndex: number,
  filePath: string,
  name: string,
  kind: OpalescentSymbolKind,
  scope: FunctionScope
): OpalescentSymbol {
  return {
    character: line.indexOf(name),
    exported: false,
    filePath,
    kind,
    line: lineIndex,
    name,
    scopeEndLine: scope.endLine,
    scopeKind: 'function',
    scopeStartLine: scope.startLine
  };
}

/**
 * Finds the top-level line that ends a function declaration's scope.
 * @param topLevelLines Top-level declaration line numbers.
 * @param declarationLine Current declaration line.
 * @param lineCount Total source line count.
 * @returns Last line covered by the declaration scope.
 */
function functionEndLine(topLevelLines: number[], declarationLine: number, lineCount: number): number {
  const nextDeclarationLine = topLevelLines.find((line) => line > declarationLine);
  return nextDeclarationLine === undefined ? Math.max(lineCount - 1, declarationLine) : nextDeclarationLine - 1;
}

/**
 * Builds a function scope for parameter and local resolution.
 * @param lines Source lines.
 * @param topLevelLines Top-level declaration line numbers.
 * @param declarationLine Function declaration line.
 * @param name Function name.
 * @returns Function scope metadata.
 */
function functionScopeForDeclaration(
  lines: string[],
  topLevelLines: number[],
  declarationLine: number,
  name: string
): FunctionScope {
  return {
    endLine: functionEndLine(topLevelLines, declarationLine, lines.length),
    name,
    signatureEndLine: signatureEndLine(lines, declarationLine),
    startLine: declarationLine
  };
}

/**
 * Finds where a function signature ends.
 * @param lines Source lines.
 * @param declarationLine Function declaration line.
 * @returns Line containing the signature's `=>`, or the declaration line when unknown.
 */
function signatureEndLine(lines: string[], declarationLine: number): number {
  for (let lineIndex = declarationLine; lineIndex < lines.length; lineIndex += 1) {
    if ((lines[lineIndex] ?? '').includes('=>')) {
      return lineIndex;
    }
    if (lineIndex > declarationLine && topLevelDeclarationPattern.test(lines[lineIndex] ?? '')) {
      return lineIndex - 1;
    }
  }
  return declarationLine;
}

/**
 * Collects parameter symbols from a function signature.
 * @param lines Source lines.
 * @param filePath Source file path.
 * @param scope Function scope being scanned.
 * @returns Parameter symbols in signature order.
 */
function parameterSymbolsForFunction(lines: string[], filePath: string, scope: FunctionScope): OpalescentSymbol[] {
  const sourceText = sourceTextWithOffsets(lines, scope.startLine, scope.signatureEndLine);
  const parameterBounds = parameterRegionBounds(sourceText.text);
  if (!parameterBounds) {
    return [];
  }

  const parameters: OpalescentSymbol[] = [];
  const parameterText = sourceText.text.slice(parameterBounds.startOffset, parameterBounds.endOffset);
  const parameterPattern = new RegExp(`\\b(${identifierPattern})\\s*:`, 'g');
  let match = parameterPattern.exec(parameterText);
  while (match?.[1]) {
    const offset = parameterBounds.startOffset + match.index;
    const position = positionAtOffset(sourceText, offset);
    parameters.push({
      character: position.character,
      exported: false,
      filePath,
      kind: 'parameter',
      line: position.line,
      name: match[1],
      scopeEndLine: scope.endLine,
      scopeKind: 'function',
      scopeStartLine: scope.startLine
    });
    match = parameterPattern.exec(parameterText);
  }
  return parameters;
}

/**
 * Locates the parameter list in a function signature text.
 * @param signatureText Function signature text.
 * @returns Start and end offsets inside the parameter list, or undefined.
 */
function parameterRegionBounds(signatureText: string): { endOffset: number; startOffset: number } | undefined {
  const functionKeywordOffset = signatureText.indexOf('f');
  const openParenOffset = signatureText.indexOf('(', functionKeywordOffset);
  if (functionKeywordOffset < 0 || openParenOffset < 0) {
    return undefined;
  }

  let depth = 0;
  for (let offset = openParenOffset; offset < signatureText.length; offset += 1) {
    const character = signatureText[offset];
    if (character === '(') {
      depth += 1;
    } else if (character === ')') {
      depth -= 1;
      if (depth === 0) {
        return { endOffset: offset, startOffset: openParenOffset + 1 };
      }
    }
  }

  return undefined;
}

/**
 * Converts a range of source lines into text with offset metadata.
 * @param lines Source lines.
 * @param startLine First included line.
 * @param endLine Last included line.
 * @returns Text and line-start offsets.
 */
function sourceTextWithOffsets(lines: string[], startLine: number, endLine: number): SourceTextWithOffsets {
  const lineOffsets: number[] = [];
  const textParts: string[] = [];
  let currentOffset = 0;
  for (let lineIndex = startLine; lineIndex <= endLine; lineIndex += 1) {
    lineOffsets.push(currentOffset);
    const line = lines[lineIndex] ?? '';
    textParts.push(line);
    currentOffset += line.length;
    if (lineIndex < endLine) {
      currentOffset += 1;
    }
  }
  return { lineOffsets, startLine, text: textParts.join('\n') };
}

/**
 * Converts an offset in joined source text back to a line and character.
 * @param sourceText Source text with offset metadata.
 * @param offset Offset to convert.
 * @returns Zero-based source position.
 */
function positionAtOffset(sourceText: SourceTextWithOffsets, offset: number): OffsetPosition {
  let relativeLine = 0;
  for (let index = 0; index < sourceText.lineOffsets.length; index += 1) {
    const nextOffset = sourceText.lineOffsets[index + 1];
    if (nextOffset === undefined || offset < nextOffset) {
      relativeLine = index;
      break;
    }
  }

  return {
    character: offset - (sourceText.lineOffsets[relativeLine] ?? 0),
    line: sourceText.startLine + relativeLine
  };
}

/**
 * Checks whether two file paths point at the same normalized path.
 * @param left Left file path.
 * @param right Right file path.
 * @returns Whether the paths match after normalization.
 */
function sameFilePath(left: string, right: string): boolean {
  return normalizeFilePath(left) === normalizeFilePath(right);
}

/**
 * Checks whether two symbols should be returned together as implementations.
 * @param origin Symbol under the cursor.
 * @param candidate Candidate implementation symbol.
 * @returns Whether the candidate belongs to the origin's implementation set.
 */
function sharesImplementationDomain(origin: OpalescentSymbol, candidate: OpalescentSymbol): boolean {
  if (origin.scopeKind === 'file') {
    return candidate.scopeKind === 'file' && candidate.kind === origin.kind;
  }

  return (
    candidate.scopeKind === origin.scopeKind &&
    sameFilePath(candidate.filePath, origin.filePath) &&
    candidate.scopeStartLine === origin.scopeStartLine &&
    candidate.scopeEndLine === origin.scopeEndLine
  );
}

/**
 * Removes common indentation from a documentation comment line.
 * @param line Raw documentation line.
 * @returns Cleaned documentation text line.
 */
function stripDocumentationIndent(line: string): string {
  return line.replace(/^\s{0,2}/, '').trimEnd();
}

/**
 * Removes doc-comment labels that are language metadata rather than hover content.
 * @param documentation Raw cleaned documentation text.
 * @returns User-facing hover documentation.
 */
function cleanDocumentationText(documentation: string): string {
  return documentation
    .split('\n')
    .map((line, index) => (index === 0 ? line.replace(/^\s*Description:\s*/i, '') : line))
    .join('\n')
    .trim();
}

/**
 * Escapes a literal string for use in a regular expression.
 * @param text Text to escape.
 * @returns Escaped regular expression text.
 */
function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Checks whether a character can be part of an Opalescent identifier.
 * @param character Single character to inspect.
 * @returns Whether the character is alphanumeric or an underscore.
 */
function isWordCharacter(character: string): boolean {
  return /[A-Za-z0-9_]/.test(character);
}
