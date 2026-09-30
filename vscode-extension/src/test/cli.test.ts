import { expect, test } from 'vitest';
import { buildArgsForContext, checkArgsForContext, formatArgs, runArgsForContext } from '../cli.js';

test('check args use project-aware JSON diagnostics when project root is known', () => {
  expect(checkArgsForContext({ filePath: '/p/src/main.op', projectRoot: '/p' })).toEqual([
    'check',
    '--json',
    '--project',
    '/p'
  ]);
});

test('check args fall back to single-file JSON diagnostics outside projects', () => {
  expect(checkArgsForContext({ filePath: '/tmp/scratch.op' })).toEqual([
    'check',
    '--json',
    '/tmp/scratch.op'
  ]);
});

test('build and run args mirror compiler project ergonomics', () => {
  expect(buildArgsForContext({ filePath: '/p/src/main.op', projectRoot: '/p' })).toEqual(['build']);
  expect(runArgsForContext({ filePath: '/p/src/main.op', projectRoot: '/p' })).toEqual(['run']);
  expect(buildArgsForContext({ filePath: '/tmp/main.op' })).toEqual(['/tmp/main.op']);
  expect(runArgsForContext({ filePath: '/tmp/main.op' })).toEqual(['run', '/tmp/main.op']);
});

test('format args write compiler formatter output to a caller-provided file', () => {
  expect(formatArgs('/tmp/main.op', '/tmp/formatted.op')).toEqual([
    'fmt',
    '--output',
    '/tmp/formatted.op',
    '/tmp/main.op'
  ]);
});
