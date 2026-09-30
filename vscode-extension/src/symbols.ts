export type OpalescentSymbolKind = 'entry' | 'function' | 'type' | 'error_set' | 'let';

export interface OpalescentSymbol {
  name: string;
  kind: OpalescentSymbolKind;
  filePath: string;
  line: number;
  character: number;
  exported: boolean;
}

const identifierPattern = '[A-Za-z_][A-Za-z0-9_]*';
const functionLetPattern = new RegExp(`^\\s*(public\\s+)?let\\s+(mutable\\s+)?(${identifierPattern})\\s*=\\s*f\\b`);
const letPattern = new RegExp(`^\\s*(public\\s+)?let\\s+(mutable\\s+)?(${identifierPattern})\\b`);
const entryPattern = new RegExp(`^\\s*(public\\s+)?entry\\s+(${identifierPattern})\\s*=\\s*f\\b`);
const typePattern = new RegExp(`^\\s*(public\\s+)?type\\s+([A-Z][A-Za-z0-9_]*)\\b`);
const errorSetPattern = new RegExp(`^\\s*(public\\s+)?error\\s+set\\s+([A-Z][A-Za-z0-9_]*)\\b`);

export function collectSymbolsFromSource(source: string, filePath: string): OpalescentSymbol[] {
  const symbols: OpalescentSymbol[] = [];
  const lines = source.split('\n');
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex] ?? '';

    const functionMatch = line.match(functionLetPattern);
    if (functionMatch?.[3]) {
      symbols.push(symbolFromMatch(line, lineIndex, filePath, functionMatch[3], 'function', Boolean(functionMatch[1])));
      continue;
    }

    const typeMatch = line.match(typePattern);
    if (typeMatch?.[2]) {
      symbols.push(symbolFromMatch(line, lineIndex, filePath, typeMatch[2], 'type', Boolean(typeMatch[1])));
      continue;
    }

    const errorSetMatch = line.match(errorSetPattern);
    if (errorSetMatch?.[2]) {
      symbols.push(symbolFromMatch(line, lineIndex, filePath, errorSetMatch[2], 'error_set', Boolean(errorSetMatch[1])));
      continue;
    }

    const entryMatch = line.match(entryPattern);
    if (entryMatch?.[2]) {
      symbols.push(symbolFromMatch(line, lineIndex, filePath, entryMatch[2], 'entry', true));
      continue;
    }

    const letMatch = line.match(letPattern);
    if (letMatch?.[3]) {
      symbols.push(symbolFromMatch(line, lineIndex, filePath, letMatch[3], 'let', Boolean(letMatch[1])));
    }
  }
  return symbols;
}

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

function symbolFromMatch(
  line: string,
  lineIndex: number,
  filePath: string,
  name: string,
  kind: OpalescentSymbolKind,
  exported: boolean
): OpalescentSymbol {
  return {
    name,
    kind,
    filePath,
    line: lineIndex,
    character: line.indexOf(name),
    exported
  };
}

function isWordCharacter(character: string): boolean {
  return /[A-Za-z0-9_]/.test(character);
}
