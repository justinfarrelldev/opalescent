import { expect, test } from 'vitest';

import {
  type OpalescentDiagnostic,
  type OpalescentTextEdit,
  PROPAGATE_ERROR_MISMATCH_CODE,
  diagnosticsByFile,
  parseOpalescentDiagnosticReport,
  propagateErrorMismatchQuickFixEdit
} from '../diagnostics.js';

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

test('builds a propagate mismatch quick fix for multiline errors clauses', () => {
  const source = `entry main = f(args: string[]): void errors
    InvalidFrameRateError,
    StdoutWriterErrors,
    TerminalWriteFailureError
=>
    propagate prepare_display()
    return void
`;
  const diagnostic = propagateMismatchDiagnostic(
    'The errors from the called function must be a subset of the errors declared by the current function. Suggested fix: Add StandardOutputHandleError, StandardOutputCapabilityError to the errors list of main like this: `errors InvalidFrameRateError, StdoutWriterErrors, TerminalWriteFailureError, StandardOutputHandleError, StandardOutputCapabilityError`.',
    0,
    0,
    5,
    15
  );

  const edit = propagateErrorMismatchQuickFixEdit(source, diagnostic);

  expect(applyEdit(source, edit)).toBe(`entry main = f(args: string[]): void errors
    InvalidFrameRateError,
    StdoutWriterErrors,
    TerminalWriteFailureError,
    StandardOutputHandleError,
    StandardOutputCapabilityError
=>
    propagate prepare_display()
    return void
`);
});

test('builds a propagate mismatch quick fix for single-line errors clauses', () => {
  const source = `let load = f(): void errors ParseError =>
    propagate read_file()
    return void
`;
  const diagnostic = propagateMismatchDiagnostic(
    'Suggested fix: Add IoError, NetworkError to the errors list of load like this: `errors ParseError, IoError, NetworkError`.',
    0,
    0,
    2,
    15
  );

  const edit = propagateErrorMismatchQuickFixEdit(source, diagnostic);

  expect(applyEdit(source, edit)).toBe(`let load = f(): void errors ParseError, IoError, NetworkError =>
    propagate read_file()
    return void
`);
});

/**
 * Builds a propagated-error mismatch diagnostic fixture.
 * @param help Diagnostic help text carrying the suggested fix.
 * @param startLine Start line for the diagnostic range.
 * @param startCharacter Start character for the diagnostic range.
 * @param endLine End line for the diagnostic range.
 * @param endCharacter End character for the diagnostic range.
 * @returns Diagnostic fixture.
 */
function propagateMismatchDiagnostic(
  help: string,
  startLine: number,
  startCharacter: number,
  endLine: number,
  endCharacter: number
): OpalescentDiagnostic {
  return {
    code: PROPAGATE_ERROR_MISMATCH_CODE,
    help,
    message: 'Propagated error types are not compatible with the function\'s declared errors',
    phase: 'type checker',
    range: {
      end: { character: endCharacter, line: endLine },
      start: { character: startCharacter, line: startLine }
    },
    severity: 'error',
    source_path: '/p/src/main.op'
  };
}

/**
 * Applies a pure text edit fixture to source.
 * @param source Source text to edit.
 * @param edit Edit returned by the quick-fix helper.
 * @returns Edited source text.
 */
function applyEdit(source: string, edit: OpalescentTextEdit | undefined): string {
  expect(edit).toBeDefined();
  const checkedEdit = edit as OpalescentTextEdit;
  const start = offsetAt(source, checkedEdit.range.start.line, checkedEdit.range.start.character);
  const end = offsetAt(source, checkedEdit.range.end.line, checkedEdit.range.end.character);
  return `${source.slice(0, start)}${checkedEdit.newText}${source.slice(end)}`;
}

/**
 * Converts a line/character pair to a string offset for test edits.
 * @param source Source text to inspect.
 * @param line Zero-based line.
 * @param character Zero-based character.
 * @returns String offset.
 */
function offsetAt(source: string, line: number, character: number): number {
  let currentLine = 0;
  let currentCharacter = 0;
  for (let index = 0; index < source.length; index += 1) {
    if (currentLine === line && currentCharacter === character) {
      return index;
    }
    const char = source[index];
    if (char === '\n') {
      currentLine += 1;
      currentCharacter = 0;
    } else {
      currentCharacter += 1;
    }
  }
  return source.length;
}

