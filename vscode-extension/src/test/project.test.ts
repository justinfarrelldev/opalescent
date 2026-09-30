import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { findProjectRoot, isOpalescentFile } from '../project.js';

test('detects .op and .types.op files as Opalescent files', () => {
  assert.equal(isOpalescentFile('/workspace/src/main.op'), true);
  assert.equal(isOpalescentFile('/workspace/src/life.types.op'), true);
  assert.equal(isOpalescentFile('/workspace/src/main.rs'), false);
});

test('finds nearest opal.toml project root by walking upward', () => {
  const root = path.resolve('/workspace/app');
  const nested = path.join(root, 'src', 'feature', 'main.op');
  const seen = new Set([path.join(root, 'opal.toml')]);

  const found = findProjectRoot(nested, (candidate) => seen.has(candidate));

  assert.equal(found, root);
});
