import test from 'node:test';
import assert from 'node:assert/strict';
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

  assert.deepEqual(
    symbols.map((symbol) => [symbol.kind, symbol.name, symbol.line, symbol.exported]),
    [
      ['function', 'helper', 2, true],
      ['type', 'LifeConfig', 5, true],
      ['error_set', 'AppErrors', 8, true],
      ['entry', 'main', 13, true],
      ['let', 'answer', 14, false]
    ]
  );
});

test('finds entry function lines for code lenses', () => {
  assert.deepEqual(findEntryLines(source), [13]);
});

test('extracts identifier word at a zero-based editor position', () => {
  assert.equal(wordAtPosition(source, 14, 25), 'helper');
  assert.equal(wordAtPosition(source, 0, 0), 'import');
});
