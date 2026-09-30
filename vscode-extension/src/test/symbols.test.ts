import { expect, test } from 'vitest';

import { collectSymbolsFromSource, findEntryLines, wordAtPosition } from '../symbols.js';

const source = `import helper from ./helper

public let helper = f(value: int32): int32 =>
    return value

public type LifeConfig:
    width: int64

public error set AppErrors = ParseError

##
  Description: Entry point for symbol indexing.
##
entry main = f(args: string[]): void =>
    let answer: int32 = helper(1)
    return void
`;

test('collects current Opalescent declarations for navigation', () => {
  const symbols = collectSymbolsFromSource(source, '/p/src/main.op');

  expect(symbols.map((symbol) => [symbol.kind, symbol.name, symbol.line, symbol.exported])).toEqual([
    ['function', 'helper', 2, true],
    ['type', 'LifeConfig', 5, true],
    ['error_set', 'AppErrors', 8, true],
    ['entry', 'main', 13, true],
    ['let', 'answer', 14, false]
  ]);
});

test('finds entry function lines for code lenses', () => {
  expect(findEntryLines(source)).toEqual([13]);
});

test('extracts identifier word at a zero-based editor position', () => {
  expect(wordAtPosition(source, 14, 25)).toBe('helper');
  expect(wordAtPosition(source, 0, 0)).toBe('import');
});
