import type { OpalescentDiagnostic } from './diagnostics.js';

const identifierPattern = '[A-Za-z_][A-Za-z0-9_]*';
const letPattern = new RegExp(`\\blet\\s+(?:mutable\\s+)?(${identifierPattern})\\b`, 'g');
const documentedFunctionPattern = new RegExp(`^\\s*(?:public\\s+let\\s+(?:mutable\\s+)?${identifierPattern}\\s*=\\s*f\\b|entry\\s+${identifierPattern}\\s*=\\s*f\\b)`);
const documentedTypePattern = /^\s*(?:public\s+)?type\s+[A-Z][A-Za-z0-9_]*\b/;
const snakeCasePattern = /^_?[a-z][a-z0-9]*(?:_[a-z0-9]+)*$/;

/**
 * Collects lightweight editor-side lint diagnostics for unsaved Opalescent buffers.
 * @param source Current document text.
 * @param sourcePath Path associated with the document.
 * @returns Local lint diagnostics that do not require a compiler process.
 */
export function collectLocalLintDiagnostics(source: string, sourcePath: string): OpalescentDiagnostic[] {
  const diagnostics: OpalescentDiagnostic[] = [];
  const lines = source.split('\n');
  let insideDocumentationBlock = false;

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex] ?? '';
    if (line.trim() === '##') {
      insideDocumentationBlock = !insideDocumentationBlock;
      continue;
    }
    if (insideDocumentationBlock || line.trimStart().startsWith('#')) {
      continue;
    }

    diagnostics.push(...lintLetNames(line, lineIndex, sourcePath));

    if ((documentedFunctionPattern.test(line) || documentedTypePattern.test(line)) && !hasDocumentationBeforeLine(lines, lineIndex)) {
      diagnostics.push(missingDocumentationDiagnostic(sourcePath, lineIndex, firstNonWhitespaceCharacter(line)));
    }

    if (line.includes('= f')) {
      diagnostics.push(...lintFunctionParameters(lines, lineIndex, sourcePath));
    }
  }

  return diagnostics;
}

/**
 * Checks whether a declaration has a documentation block immediately above it.
 * @param lines Source lines.
 * @param lineIndex Declaration line index.
 * @returns Whether a preceding documentation block exists and contains text.
 */
function hasDocumentationBeforeLine(lines: string[], lineIndex: number): boolean {
  let cursor = lineIndex - 1;
  while (cursor >= 0 && (lines[cursor] ?? '').trim() === '') {
    cursor -= 1;
  }
  if ((lines[cursor] ?? '').trim() !== '##') {
    return false;
  }

  cursor -= 1;
  let documentationLength = 0;
  while (cursor >= 0 && (lines[cursor] ?? '').trim() !== '##') {
    documentationLength += (lines[cursor] ?? '').trim().length;
    cursor -= 1;
  }
  return cursor >= 0 && documentationLength >= 30;
}

/**
 * Finds the first non-whitespace character on a line.
 * @param line Source line.
 * @returns Zero-based character offset.
 */
function firstNonWhitespaceCharacter(line: string): number {
  const match = /\S/.exec(line);
  return match?.index ?? 0;
}

/**
 * Reports snake_case violations for let-bound identifiers on one line.
 * @param line Source line.
 * @param lineIndex Source line index.
 * @param sourcePath Source path.
 * @returns Naming diagnostics for the line.
 */
function lintLetNames(line: string, lineIndex: number, sourcePath: string): OpalescentDiagnostic[] {
  const diagnostics: OpalescentDiagnostic[] = [];
  letPattern.lastIndex = 0;
  let match = letPattern.exec(line);
  while (match?.[1]) {
    const name = match[1];
    if (!isSnakeCase(name)) {
      diagnostics.push(snakeCaseDiagnostic(sourcePath, lineIndex, match.index + match[0].lastIndexOf(name), name));
    }
    match = letPattern.exec(line);
  }
  return diagnostics;
}

/**
 * Reports snake_case violations for function parameters in a signature.
 * @param lines Source lines.
 * @param startLine First line of the function declaration.
 * @param sourcePath Source path.
 * @returns Naming diagnostics for parameters.
 */
function lintFunctionParameters(lines: string[], startLine: number, sourcePath: string): OpalescentDiagnostic[] {
  const signature = functionSignatureText(lines, startLine);
  const bounds = parameterBounds(signature.text);
  if (!bounds) {
    return [];
  }

  const diagnostics: OpalescentDiagnostic[] = [];
  const parameterText = signature.text.slice(bounds.startOffset, bounds.endOffset);
  const parameterPattern = new RegExp(`\\b(${identifierPattern})\\s*:`, 'g');
  let match = parameterPattern.exec(parameterText);
  while (match?.[1]) {
    const name = match[1];
    if (!isSnakeCase(name)) {
      const position = positionAtOffset(signature.lineOffsets, startLine, bounds.startOffset + match.index);
      diagnostics.push(snakeCaseDiagnostic(sourcePath, position.line, position.character, name));
    }
    match = parameterPattern.exec(parameterText);
  }
  return diagnostics;
}

/**
 * Builds a joined function signature and line offsets.
 * @param lines Source lines.
 * @param startLine First signature line.
 * @returns Joined signature text and line offsets.
 */
function functionSignatureText(lines: string[], startLine: number): { lineOffsets: number[]; text: string } {
  const lineOffsets: number[] = [];
  const parts: string[] = [];
  let offset = 0;
  for (let lineIndex = startLine; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex] ?? '';
    lineOffsets.push(offset);
    parts.push(line);
    offset += line.length;
    if (line.includes('=>')) {
      break;
    }
    offset += 1;
  }
  return { lineOffsets, text: parts.join('\n') };
}

/**
 * Locates a function parameter list inside a signature.
 * @param signatureText Function signature text.
 * @returns Parameter-list bounds, or undefined when no complete list exists.
 */
function parameterBounds(signatureText: string): { endOffset: number; startOffset: number } | undefined {
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
 * Converts an offset in joined signature text back to a document position.
 * @param lineOffsets Joined-signature line offsets.
 * @param startLine First document line included in the signature.
 * @param offset Offset in joined text.
 * @returns Document line and character.
 */
function positionAtOffset(lineOffsets: number[], startLine: number, offset: number): { character: number; line: number } {
  let relativeLine = 0;
  for (let index = 0; index < lineOffsets.length; index += 1) {
    const nextOffset = lineOffsets[index + 1];
    if (nextOffset === undefined || offset < nextOffset) {
      relativeLine = index;
      break;
    }
  }
  return {
    character: offset - (lineOffsets[relativeLine] ?? 0),
    line: startLine + relativeLine
  };
}

/**
 * Checks whether a value name follows Opalescent snake_case rules.
 * @param name Identifier to inspect.
 * @returns Whether the identifier is snake_case.
 */
function isSnakeCase(name: string): boolean {
  return snakeCasePattern.test(name);
}

/**
 * Creates a missing-documentation diagnostic.
 * @param sourcePath Source path.
 * @param line Source line.
 * @param character Source character.
 * @returns Diagnostic object.
 */
function missingDocumentationDiagnostic(sourcePath: string, line: number, character: number): OpalescentDiagnostic {
  return {
    code: 'opalescent.vscode.documentation.missing',
    help: 'Add a ## documentation block with a Description before this declaration.',
    message: 'Opalescent declarations require documentation comments.',
    phase: 'vscode local lint',
    range: { end: { character: character + 1, line }, start: { character, line } },
    severity: 'warning',
    source_path: sourcePath
  };
}

/**
 * Creates a snake_case naming diagnostic.
 * @param sourcePath Source path.
 * @param line Source line.
 * @param character Identifier start character.
 * @param name Invalid identifier.
 * @returns Diagnostic object.
 */
function snakeCaseDiagnostic(sourcePath: string, line: number, character: number, name: string): OpalescentDiagnostic {
  return {
    code: 'opalescent.vscode.naming.snake_case',
    help: 'Rename this identifier to snake_case.',
    message: `Identifier '${name}' should use snake_case.`,
    phase: 'vscode local lint',
    range: { end: { character: character + name.length, line }, start: { character, line } },
    severity: 'warning',
    source_path: sourcePath
  };
}
