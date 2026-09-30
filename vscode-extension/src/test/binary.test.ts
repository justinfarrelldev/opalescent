import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { candidateBinaryPaths, shellQuote } from '../binary.js';

test('binary candidates prefer configured path then workspace build outputs then PATH names', () => {
  const root = path.resolve('/workspace/opalescent');

  assert.deepEqual(candidateBinaryPaths([root], '/custom/opal'), [
    '/custom/opal',
    path.join(root, 'target', 'debug', process.platform === 'win32' ? 'opalescent.exe' : 'opalescent'),
    path.join(root, 'target', 'release', process.platform === 'win32' ? 'opalescent.exe' : 'opalescent'),
    'opalescent',
    'opal'
  ]);
});

test('shellQuote quotes paths safely for VS Code terminals', () => {
  assert.equal(shellQuote('/tmp/opal binary'), "'/tmp/opal binary'");
  assert.equal(shellQuote("/tmp/it's-opal"), "'/tmp/it'\\''s-opal'");
});
