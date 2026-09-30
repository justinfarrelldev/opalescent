import fs from 'node:fs';
import path from 'node:path';
import { expect, test } from 'vitest';

const extensionRoot = path.resolve(__dirname, '..', '..');
const repoRoot = path.resolve(extensionRoot, '..');

/**
 * Reads a JSON file from the extension root for metadata assertions.
 * @param relativePath Path to the JSON file relative to the extension root.
 * @returns Parsed JSON content.
 */
function readJson<T>(relativePath: string): T {
  return JSON.parse(fs.readFileSync(path.join(extensionRoot, relativePath), 'utf8')) as T;
}

test('extension package uses pnpm/Vitest and command categories for palette ergonomics', () => {
  const manifest = readJson<{
    contributes?: { commands?: Array<{ category?: string; command: string }> };
    packageManager?: string;
    scripts?: Record<string, string>;
  }>('package.json');

  expect(manifest.packageManager).toMatch(/^pnpm@/);
  expect(manifest.scripts?.test).toContain('vitest run');
  expect(manifest.scripts?.package).toMatch(/^pnpm compile && vsce package/);
  expect(manifest.contributes?.commands?.every((command) => command.category === 'Opalescent')).toBe(true);
});

test('language configuration avoids brace-body affordances and indents block-introducing syntax', () => {
  const config = readJson<{
    autoClosingPairs?: Array<{ close: string; open: string } | string[]>;
    indentationRules?: { increaseIndentPattern?: string };
  }>('language-configuration.json');

  expect(config.autoClosingPairs).not.toContainEqual({ close: '}', open: '{' });
  expect(config.indentationRules?.increaseIndentPattern).toContain('=>');
  expect(config.indentationRules?.increaseIndentPattern).toContain(':');
});

test('TextMate grammar tracks current public keywords and excludes removed public primitives', () => {
  const grammarText = fs.readFileSync(path.join(extensionRoot, 'syntaxes', 'opalescent.tmLanguage.json'), 'utf8');

  expect(grammarText).toContain('error');
  expect(grammarText).toContain('set');
  expect(grammarText).toContain('untested');
  expect(grammarText).toContain('type_of');
  expect(grammarText).not.toContain('string|char|void');
});

test('VSIX packaging files and cargo-make tasks are present', () => {
  expect(fs.existsSync(path.join(extensionRoot, '.vscodeignore'))).toBe(true);
  expect(fs.existsSync(path.join(extensionRoot, 'README.md'))).toBe(true);

  const makefile = fs.readFileSync(path.join(repoRoot, 'Makefile.toml'), 'utf8');
  expect(makefile).toContain('[tasks.vscode-extension-test]');
  expect(makefile).toContain('[tasks.vscode-extension-vsix]');
  expect(makefile).toContain('pnpm install --frozen-lockfile');
});
