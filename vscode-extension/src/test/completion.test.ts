import path from 'node:path';
import { expect, test } from 'vitest';

import {
  completionItemsForSymbols,
  importLineForSymbol
} from '../completion.js';
import { collectSymbolsFromSource } from '../symbols.js';

const currentFile = path.resolve('/workspace/app/src/main.op');
const nestedHelperFile = path.resolve('/workspace/app/src/features/deep/down/helper.op');

const nestedHelperSource = `##
  Description: Function exported from a deeply nested helper module.
##
public let my_function = f(): int32 =>
    return 1

let secret_function = f(): int32 =>
    return 2
`;

const currentSource = `entry main = f(args: string[]): void =>
    my_
    return void
`;

test('suggests public project symbols from nested files with an auto-import edit', () => {
  const symbols = [
    ...collectSymbolsFromSource(currentSource, currentFile),
    ...collectSymbolsFromSource(nestedHelperSource, nestedHelperFile)
  ];

  const completions = completionItemsForSymbols({ currentFilePath: currentFile, source: currentSource, symbols });
  const item = completions.find((completion) => completion.name === 'my_function');

  expect(item).toMatchObject({
    autoImportEdit: {
      character: 0,
      line: 0,
      text: 'import my_function from ./features/deep/down/helper\n\n'
    },
    insertText: 'my_function',
    name: 'my_function',
    sourceModule: './features/deep/down/helper'
  });
  expect(completions.some((completion) => completion.name === 'secret_function')).toBe(false);
});

test('appends auto-import edits after an existing top import block', () => {
  const source = `import existing_helper from ./existing

entry main = f(args: string[]): void =>
    my_
    return void
`;
  const symbols = [
    ...collectSymbolsFromSource(source, currentFile),
    ...collectSymbolsFromSource(nestedHelperSource, nestedHelperFile)
  ];

  const item = completionItemsForSymbols({ currentFilePath: currentFile, source, symbols }).find(
    (completion) => completion.name === 'my_function'
  );

  expect(item?.autoImportEdit).toEqual({
    character: 0,
    line: 1,
    text: 'import my_function from ./features/deep/down/helper\n'
  });
});

test('does not add an auto-import edit for already imported or same-file symbols', () => {
  const importedSource = `import my_function from ./features/deep/down/helper

entry main = f(args: string[]): void =>
    my_
    return void
`;
  const importedSymbols = [
    ...collectSymbolsFromSource(importedSource, currentFile),
    ...collectSymbolsFromSource(nestedHelperSource, nestedHelperFile)
  ];
  const importedItem = completionItemsForSymbols({ currentFilePath: currentFile, source: importedSource, symbols: importedSymbols }).find(
    (completion) => completion.name === 'my_function'
  );

  const sameFileSource = `${nestedHelperSource}
entry main = f(args: string[]): void =>
    my_
    return void
`;
  const sameFileItem = completionItemsForSymbols({
    currentFilePath: currentFile,
    source: sameFileSource,
    symbols: collectSymbolsFromSource(sameFileSource, currentFile)
  }).find((completion) => completion.name === 'my_function');

  expect(importedItem?.autoImportEdit).toBeUndefined();
  expect(sameFileItem?.autoImportEdit).toBeUndefined();
});

test('uses import type for public types from type modules', () => {
  const typeFile = path.resolve('/workspace/app/src/model/life.types.op');
  const typeSource = `public type LifeConfig:
    width: int64
`;
  const typeSymbol = collectSymbolsFromSource(typeSource, typeFile).find((symbol) => symbol.name === 'LifeConfig');

  expect(typeSymbol ? importLineForSymbol(typeSymbol, currentFile) : undefined).toBe(
    'import type LifeConfig from ./model/life.types'
  );
});
