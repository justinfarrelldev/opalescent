import path from 'node:path';
import { expect, test } from 'vitest';
import { findProjectRoot, isOpalescentFile } from '../project.js';

test('detects .op and .types.op files as Opalescent files', () => {
  expect(isOpalescentFile('/workspace/src/main.op')).toBe(true);
  expect(isOpalescentFile('/workspace/src/life.types.op')).toBe(true);
  expect(isOpalescentFile('/workspace/src/main.rs')).toBe(false);
});

test('finds nearest opal.toml project root by walking upward', () => {
  const root = path.resolve('/workspace/app');
  const nested = path.join(root, 'src', 'feature', 'main.op');
  const seen = new Set([path.join(root, 'opal.toml')]);

  const found = findProjectRoot(nested, (candidate) => seen.has(candidate));

  expect(found).toBe(root);
});
