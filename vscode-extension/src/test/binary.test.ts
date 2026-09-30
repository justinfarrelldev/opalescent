import path from 'node:path';
import { expect, test } from 'vitest';
import { candidateBinaryPaths, shellQuote } from '../binary.js';

test('binary candidates prefer configured path then workspace build outputs then PATH names', () => {
  const root = path.resolve('/workspace/opalescent');

  expect(candidateBinaryPaths([root], '/custom/opal')).toEqual([
    '/custom/opal',
    path.join(root, 'target', 'debug', process.platform === 'win32' ? 'opalescent.exe' : 'opalescent'),
    path.join(root, 'target', 'release', process.platform === 'win32' ? 'opalescent.exe' : 'opalescent'),
    'opalescent',
    'opal'
  ]);
});

test('shellQuote quotes paths safely for VS Code terminals', () => {
  expect(shellQuote('/tmp/opal binary')).toBe("'/tmp/opal binary'");
  expect(shellQuote("/tmp/it's-opal")).toBe("'/tmp/it'\\''s-opal'");
});
