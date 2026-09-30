import { expect, test } from 'vitest';

import { diagnosticsByFile, parseOpalescentDiagnosticReport } from '../diagnostics.js';

test('parses compiler JSON diagnostics and groups them by source path', () => {
  const report = parseOpalescentDiagnosticReport(JSON.stringify({
    diagnostics: [
      {
        code: 'opalescent::type_system::type_mismatch',
        help: 'Change one type',
        message: 'Type mismatch',
        phase: 'type checker',
        range: { end: { character: 16, line: 4 }, start: { character: 10, line: 4 } },
        severity: 'error',
        source_path: '/p/src/main.op'
      },
      {
        message: 'Variable is never used',
        phase: 'type checker',
        range: { end: { character: 13, line: 1 }, start: { character: 8, line: 1 } },
        severity: 'warning',
        source_path: '/p/src/helper.op'
      }
    ],
    success: false
  }));

  const grouped = diagnosticsByFile(report);

  expect(report.success).toBe(false);
  expect(grouped.get('/p/src/main.op')?.[0]?.severity).toBe('error');
  expect(grouped.get('/p/src/helper.op')?.[0]?.severity).toBe('warning');
});

test('rejects malformed compiler diagnostic JSON', () => {
  expect(() => parseOpalescentDiagnosticReport('{"success": true}')).toThrow(/diagnostics/);
});
