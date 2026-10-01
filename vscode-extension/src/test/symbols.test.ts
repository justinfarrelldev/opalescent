import { expect, test } from 'vitest';

import {
  collectSymbolsFromSource,
  definitionSymbolAtPosition,
  definitionSymbolsForWord,
  findEntryLines,
  hoverSymbolForWord,
  importTargetAtPosition,
  referenceTargetsForSymbol,
  wordAtPosition
} from '../symbols.js';

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
    ['parameter', 'value', 2, false],
    ['type', 'LifeConfig', 5, true],
    ['error_set', 'AppErrors', 8, true],
    ['entry', 'main', 13, true],
    ['parameter', 'args', 13, false],
    ['let', 'answer', 14, false]
  ]);
});

test('attaches documentation comments to declarations for hover text', () => {
  const symbols = collectSymbolsFromSource(source, '/p/src/main.op');
  const main = symbols.find((symbol) => symbol.name === 'main');

  expect(main?.documentation).toBe('Entry point for symbol indexing.');
});

const labelsSource = `import type EditorStatus from ./editor.types

##
  Description: Returns a display label for the current editor status.
##
public let status_text_for = f(status: EditorStatus): string =>
    if status is EditorStatus.BellFailed into bell_failed:
        return bell_failed.message
    return 'ok'
`;

const inputSource = `import type EditorStatus from ./editor.types

##
  Description: Builds an invalid named-key transition with a safe owned label.
##
public let invalid_named_key_transition = f(key_text: string): void =>
    let status: EditorStatus = new EditorStatus.InvalidKey:
        key: key_text
    return void
`;

test('resolves same-file function parameters before same-named locals in other project files', () => {
  const symbols = [
    ...collectSymbolsFromSource(inputSource, '/p/src/input.op'),
    ...collectSymbolsFromSource(labelsSource, '/p/src/labels.op')
  ];
  const definitions = definitionSymbolsForWord(symbols, 'status', { character: 7, filePath: '/p/src/labels.op', line: 6 });

  expect(definitions.map((symbol) => [symbol.kind, symbol.filePath, symbol.line, symbol.name])).toEqual([
    ['parameter', '/p/src/labels.op', 5, 'status']
  ]);
});

test('go-to-definition on a definition site returns same-name implementations', () => {
  const firstSource = `public let render = f(): void =>
    return void
`;
  const secondSource = `public let render = f(value: int32): void =>
    return void
`;
  const symbols = [
    ...collectSymbolsFromSource(firstSource, '/p/src/first.op'),
    ...collectSymbolsFromSource(secondSource, '/p/src/second.op')
  ];
  const definitions = definitionSymbolsForWord(symbols, 'render', { character: 11, filePath: '/p/src/first.op', line: 0 });

  expect(definitions.map((symbol) => symbol.filePath)).toEqual(['/p/src/first.op', '/p/src/second.op']);
});

test('finds hover documentation through symbol uses', () => {
  const symbols = collectSymbolsFromSource(labelsSource, '/p/src/labels.op');
  const hovered = hoverSymbolForWord(symbols, 'status_text_for', { character: 14, filePath: '/p/src/labels.op', line: 5 });

  expect(hovered?.documentation).toBe('Returns a display label for the current editor status.');
});

test('finds local references when go-to-definition starts on a variable definition', () => {
  const referenceSource = `public let example = f(): void =>
    let changed = 1
    print(changed)
    return void
`;
  const symbols = collectSymbolsFromSource(referenceSource, '/p/src/example.op');
  const definition = definitionSymbolAtPosition(symbols, 'changed', { character: 8, filePath: '/p/src/example.op', line: 1 });
  const references = definition ? referenceTargetsForSymbol(referenceSource, definition) : [];

  expect(references.map((reference) => [reference.line, reference.character, reference.name])).toEqual([[2, 10, 'changed']]);
});

test('detects import module specifiers for ctrl-click file navigation', () => {
  const importSource = `import type EditorState from ./editor.types
import helper from ../helpers/buffer
`;

  expect(importTargetAtPosition(importSource, 0, 38)).toEqual({
    character: 29,
    length: 14,
    line: 0,
    moduleSpecifier: './editor.types'
  });
  expect(importTargetAtPosition(importSource, 1, 29)?.moduleSpecifier).toBe('../helpers/buffer');
});

test('finds entry function lines for code lenses', () => {
  expect(findEntryLines(source)).toEqual([13]);
});

test('extracts identifier word at a zero-based editor position', () => {
  expect(wordAtPosition(source, 14, 25)).toBe('helper');
  expect(wordAtPosition(source, 0, 0)).toBe('import');
});
