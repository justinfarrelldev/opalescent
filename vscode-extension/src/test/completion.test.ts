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

test('includes core language keyword completions', () => {
  const completions = completionItemsForSymbols({ currentFilePath: currentFile, source: '', symbols: [] });

  expect(completions).toEqual(
    expect.arrayContaining([
      expect.objectContaining({ insertText: 'let', kind: 'keyword', name: 'let' }),
      expect.objectContaining({ insertText: 'while', kind: 'keyword', name: 'while' }),
      expect.objectContaining({ insertText: 'propagate', kind: 'keyword', name: 'propagate' })
    ])
  );
  expect(completions.some((completion) => completion.name === 'match')).toBe(false);
});

test('includes tab-stop snippets for common Opalescent structures', () => {
  const completions = completionItemsForSymbols({ currentFilePath: currentFile, source: '', symbols: [] });
  const guardSnippet = completions.find((completion) => completion.name === 'guard into else');
  const whileSnippet = completions.find((completion) => completion.name === 'while block');

  expect(guardSnippet).toMatchObject({
    insertText: 'guard ${1:fallible_call()} into ${2:value} else ${3:err} =>\n    ${0}',
    isSnippet: true,
    kind: 'snippet'
  });
  expect(whileSnippet).toMatchObject({
    insertText: 'while ${1:condition}:\n    ${0}',
    isSnippet: true,
    kind: 'snippet'
  });
});

test('keeps a small curated snippet set for common Opalescent templates', () => {
  const snippets = completionItemsForSymbols({ currentFilePath: currentFile, source: '', symbols: [] }).filter(
    (completion) => completion.kind === 'snippet'
  );

  expect(snippets.length).toBeLessThanOrEqual(16);
  expect(snippets).toEqual(
    expect.arrayContaining([
      expect.objectContaining({
        insertText: '##\n  Description: ${1:description}\n##',
        name: 'doc comment'
      }),
      expect.objectContaining({
        insertText: 'import type ${1:TypeName} from ${0:./module.types}',
        name: 'import type'
      }),
      expect.objectContaining({
        insertText: 'let ${1:name} = f(${2:args}): ${3:return_type} errors ${4:ErrorType} =>\n    ${0:return propagate fallible_call()}',
        name: 'fallible function'
      }),
      expect.objectContaining({
        insertText: 'loop =>\n    ${1}\n    break ${0:result: value}',
        name: 'loop break'
      }),
      expect.objectContaining({
        insertText: 'new ${1:TypeName}:\n    ${2:field}: ${0:value}',
        name: 'new value'
      })
    ])
  );
  expect(snippets.some((completion) => completion.name === 'match block')).toBe(false);
});

test('documents primitive type completions for user-facing IntelliSense', () => {
  const completions = completionItemsForSymbols({ currentFilePath: currentFile, source: '', symbols: [] });
  const int64Completion = completions.find((completion) => completion.name === 'int64');

  expect(int64Completion).toMatchObject({
    detail: 'Primitive type',
    documentation: expect.stringContaining('64-bit signed integer'),
    kind: 'keyword'
  });
  expect(int64Completion?.documentation).toContain('-9,223,372,036,854,775,808');
});

test('suggests standard-library functions with docs and auto-import edits', () => {
  const completions = completionItemsForSymbols({ currentFilePath: currentFile, source: '', symbols: [] });
  const pathFrom = completions.find((completion) => completion.name === 'path_from');
  const readText = completions.find((completion) => completion.name === 'read_text_sync');

  expect(pathFrom).toMatchObject({
    autoImportEdit: { character: 0, line: 0, text: 'import path_from from standard\n' },
    detail: 'standard: path_from(raw: string): FilesystemPath',
    documentation: expect.stringContaining('Wraps a raw string as a filesystem path.'),
    insertText: 'path_from',
    sourceModule: 'standard'
  });
  expect(readText).toMatchObject({
    autoImportEdit: { character: 0, line: 0, text: 'import read_text_sync from standard\n' },
    documentation: expect.stringContaining('Reads the whole file as UTF-8 text.'),
    sourceModule: 'standard'
  });
  expect(readText?.documentation).toContain('InvalidUtf8Error');
});

test('does not auto-import already imported standard-library completions', () => {
  const source = `import path_from from standard\n\nentry main = f(args: string[]): void =>\n    return void\n`;
  const pathFrom = completionItemsForSymbols({ currentFilePath: currentFile, source, symbols: [] }).find(
    (completion) => completion.name === 'path_from'
  );

  expect(pathFrom?.autoImportEdit).toBeUndefined();
});

test('suggests standard error sets with import type edits and docs', () => {
  const completions = completionItemsForSymbols({ currentFilePath: currentFile, source: '', symbols: [] });
  const filesystemReadErrors = completions.find((completion) => completion.name === 'FilesystemReadErrors');

  expect(filesystemReadErrors).toMatchObject({
    autoImportEdit: { character: 0, line: 0, text: 'import type FilesystemReadErrors from standard.errors\n' },
    documentation: expect.stringContaining('FileNotFoundError'),
    sourceModule: 'standard.errors'
  });
});
