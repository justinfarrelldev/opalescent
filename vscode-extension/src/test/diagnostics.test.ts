import { expect, test } from 'vitest';
import { diagnosticsByFile, parseOpalescentDiagnosticReport } from '../diagnostics.js';

test('parses compiler JSON diagnostics and groups them by source path', () => {
  const report = parseOpalescentDiagnosticReport(JSON.stringify({
    success: false,
    diagnostics: [
      {
        source_path: '/p/src/main.op',
        severity: 'error',
        phase: 'type checker',
        code: 'opalescent::type_system::type_mismatch',
        message: 'Type mismatch',
        help: 'Change one type',
        range: { start: { line: 4, character: 10 }, end: { line: 4, character: 16 } }
      },
      {
        source_path: '/p/src/helper.op',
        severity: 'warning',
        phase: 'type checker',
        message: 'Variable is never used',
        range: { start: { line: 1, character: 8 }, end: { line: 1, character: 13 } }
      }
    ]
  }));

  const grouped = diagnosticsByFile(report);

  expect(report.success).toBe(false);
  expect(grouped.get('/p/src/main.op')?.[0]?.severity).toBe('error');
  expect(grouped.get('/p/src/helper.op')?.[0]?.severity).toBe('warning');
});

test('rejects malformed compiler diagnostic JSON', () => {
  expect(() => parseOpalescentDiagnosticReport('{"success": true}')).toThrow(/diagnostics/);
});
