import test from 'node:test';
import assert from 'node:assert/strict';
import { buildArgsForContext, checkArgsForContext, formatArgs, runArgsForContext } from '../cli.js';

test('check args use project-aware JSON diagnostics when project root is known', () => {
  assert.deepEqual(checkArgsForContext({ filePath: '/p/src/main.op', projectRoot: '/p' }), [
    'check',
    '--json',
    '--project',
    '/p'
  ]);
});

test('check args fall back to single-file JSON diagnostics outside projects', () => {
  assert.deepEqual(checkArgsForContext({ filePath: '/tmp/scratch.op' }), [
    'check',
    '--json',
    '/tmp/scratch.op'
  ]);
});

test('build and run args mirror compiler project ergonomics', () => {
  assert.deepEqual(buildArgsForContext({ filePath: '/p/src/main.op', projectRoot: '/p' }), ['build']);
  assert.deepEqual(runArgsForContext({ filePath: '/p/src/main.op', projectRoot: '/p' }), ['run']);
  assert.deepEqual(buildArgsForContext({ filePath: '/tmp/main.op' }), ['/tmp/main.op']);
  assert.deepEqual(runArgsForContext({ filePath: '/tmp/main.op' }), ['run', '/tmp/main.op']);
});

test('format args write compiler formatter output to a caller-provided file', () => {
  assert.deepEqual(formatArgs('/tmp/main.op', '/tmp/formatted.op'), [
    'fmt',
    '--output',
    '/tmp/formatted.op',
    '/tmp/main.op'
  ]);
});
