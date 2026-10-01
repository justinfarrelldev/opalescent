import path from 'node:path';
import { expect, test } from 'vitest';

import { findProjectRoot, isOpalescentFile, moduleSpecifierForLocalImport, resolveLocalImportPath } from '../project.js';

test('detects .op and .types.op files as Opalescent files', () => {
  expect(isOpalescentFile('/workspace/src/main.op')).toBe(true);
  expect(isOpalescentFile('/workspace/src/life.types.op')).toBe(true);
  expect(isOpalescentFile('/workspace/src/main.rs')).toBe(false);
});

test('resolves relative Opalescent import specifiers to source files', () => {
  const importer = path.resolve('/workspace/app/src/input.op');
  const editorTypes = path.resolve('/workspace/app/src/editor.types.op');
  const buffer = path.resolve('/workspace/app/src/buffer.op');
  const seen = new Set([editorTypes, buffer]);

  expect(resolveLocalImportPath(importer, './editor.types', (candidate) => seen.has(candidate))).toBe(editorTypes);
  expect(resolveLocalImportPath(importer, './buffer', (candidate) => seen.has(candidate))).toBe(buffer);
  expect(resolveLocalImportPath(importer, 'standard', (candidate) => seen.has(candidate))).toBeUndefined();
});

test('finds nearest opal.toml project root by walking upward', () => {
  const root = path.resolve('/workspace/app');
  const nested = path.join(root, 'src', 'feature', 'main.op');
  const seen = new Set([path.join(root, 'opal.toml')]);

  const found = findProjectRoot(nested, (candidate) => seen.has(candidate));

  expect(found).toBe(root);
});

test('builds local import specifiers from source files', () => {
  const importer = path.resolve('/workspace/app/src/main.op');

  expect(moduleSpecifierForLocalImport(importer, path.resolve('/workspace/app/src/feature/deep/helper.op'))).toBe(
    './feature/deep/helper'
  );
  expect(moduleSpecifierForLocalImport(importer, path.resolve('/workspace/app/shared/math.op'))).toBe('../shared/math');
  expect(moduleSpecifierForLocalImport(importer, path.resolve('/workspace/app/src/life.types.op'))).toBe('./life.types');
});
