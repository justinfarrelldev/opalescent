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
  expect(grammarText).toContain('using');
  expect(grammarText).toContain('ref');
  expect(grammarText).not.toContain('string|char|void');
});

test('TextMate grammar distinguishes declaration keywords and noisy error propagation', () => {
  const grammar = readJson<{
    repository?: {
      declarations?: { patterns?: Array<{ captures?: Record<string, { name?: string }> }> };
      keyword?: { patterns?: Array<{ match?: string; name?: string }> };
    };
  }>('syntaxes/opalescent.tmLanguage.json');
  const declarationCaptures = grammar.repository?.declarations?.patterns?.flatMap((pattern) => Object.values(pattern.captures ?? {}).map((capture) => capture.name)) ?? [];
  const keywordPatterns = grammar.repository?.keyword?.patterns ?? [];

  expect(declarationCaptures).toContain('storage.type.let.opalescent');
  expect(keywordPatterns).toContainEqual(expect.objectContaining({ name: 'keyword.operator.error-propagation.opalescent' }));
});

test('TextMate grammar gives type members and fields TypeScript-like value scopes without breaking return labels', () => {
  const grammar = readJson<{
    patterns?: Array<{ include?: string }>;
    repository?: {
      'type-block'?: { patterns?: Array<{ include?: string }> };
      'type-member'?: { patterns?: Array<{ captures?: Record<string, { name?: string }> }> };
    };
  }>('syntaxes/opalescent.tmLanguage.json');
  const variantCaptures = grammar.repository?.['type-member']?.patterns?.flatMap((pattern) => Object.values(pattern.captures ?? {}).map((capture) => capture.name)) ?? [];

  expect(grammar.patterns).toContainEqual({ include: '#type-block' });
  expect(grammar.patterns).not.toContainEqual({ include: '#type-member' });
  expect(grammar.repository?.['type-block']?.patterns).toContainEqual({ include: '#type-member' });
  expect(variantCaptures).toContain('variable.other.property.opalescent variable.other.enummember.opalescent');
});

test('TextMate grammar highlights every type in errors clauses', () => {
  const grammar = readJson<{
    repository?: {
      'errors-clause'?: { begin?: string; end?: string; patterns?: Array<{ include?: string; name?: string }> };
    };
  }>('syntaxes/opalescent.tmLanguage.json');

  expect(grammar.repository?.['errors-clause']?.begin).toContain('errors');
  expect(grammar.repository?.['errors-clause']?.end).toBe('(?==>)');
  expect(grammar.repository?.['errors-clause']?.patterns).toContainEqual(expect.objectContaining({ include: '#type-name' }));
});

test('VSIX packaging files and cargo-make tasks are present', () => {
  expect(fs.existsSync(path.join(extensionRoot, '.vscodeignore'))).toBe(true);
  expect(fs.existsSync(path.join(extensionRoot, 'README.md'))).toBe(true);

  const makefile = fs.readFileSync(path.join(repoRoot, 'Makefile.toml'), 'utf8');
  expect(makefile).toContain('[tasks.vscode-extension-test]');
  expect(makefile).toContain('[tasks.vscode-extension-vsix]');
  expect(makefile).toContain('pnpm install --frozen-lockfile');
});
