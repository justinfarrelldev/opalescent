export type OpalescentDiagnosticSeverity = 'error' | 'hint' | 'information' | 'warning';

export const PROPAGATE_ERROR_MISMATCH_CODE = 'opalescent::type_system::propagate_error_mismatch';

export interface OpalescentDiagnosticPosition {
  character: number;
  line: number;
}

export interface OpalescentDiagnosticRange {
  end: OpalescentDiagnosticPosition;
  start: OpalescentDiagnosticPosition;
}

export interface OpalescentDiagnostic {
  code?: string;
  help?: string;
  message: string;
  phase: string;
  range: OpalescentDiagnosticRange;
  severity: OpalescentDiagnosticSeverity;
  source_path: string;
}

export interface OpalescentTextEdit {
  addedErrorNames: string[];
  newText: string;
  range: OpalescentDiagnosticRange;
}

interface ErrorClauseSpan {
  namesEnd: number;
  namesStart: number;
}

export interface OpalescentDiagnosticReport {
  diagnostics: OpalescentDiagnostic[];
  success: boolean;
}

const severities = new Set(['error', 'warning', 'information', 'hint']);
const errorsKeyword = 'errors';

/**
 * Parses and validates the compiler's JSON diagnostic report.
 * @param jsonText Raw JSON emitted by the compiler.
 * @returns A validated diagnostic report.
 */
export function parseOpalescentDiagnosticReport(jsonText: string): OpalescentDiagnosticReport {
  const parsed: unknown = JSON.parse(jsonText);
  if (!isRecord(parsed)) {
    throw new Error('Opalescent diagnostics JSON must be an object');
  }
  if (typeof parsed.success !== 'boolean') {
    throw new Error('Opalescent diagnostics JSON must contain boolean success');
  }
  if (!Array.isArray(parsed.diagnostics)) {
    throw new Error('Opalescent diagnostics JSON must contain diagnostics array');
  }

  const diagnostics = parsed.diagnostics.map((diagnostic, index) => validateDiagnostic(diagnostic, index));
  return { diagnostics, success: parsed.success };
}

/**
 * Groups diagnostics by source file path.
 * @param report Validated compiler diagnostic report.
 * @returns Diagnostics keyed by source path.
 */
export function diagnosticsByFile(report: OpalescentDiagnosticReport): Map<string, OpalescentDiagnostic[]> {
  const grouped = new Map<string, OpalescentDiagnostic[]>();
  for (const diagnostic of report.diagnostics) {
    const existing = grouped.get(diagnostic.source_path) ?? [];
    existing.push(diagnostic);
    grouped.set(diagnostic.source_path, existing);
  }
  return grouped;
}

/**
 * Builds a source edit that adds propagated-error mismatch missing errors.
 * @param source Complete source text for the diagnostic file.
 * @param diagnostic Compiler diagnostic that may carry propagated-error mismatch help.
 * @returns Text edit for the function errors clause, or undefined when unavailable.
 */
export function propagateErrorMismatchQuickFixEdit(
  source: string,
  diagnostic: OpalescentDiagnostic
): OpalescentTextEdit | undefined {
  if (diagnostic.code !== PROPAGATE_ERROR_MISMATCH_CODE) {
    return undefined;
  }

  const missingErrorNames = missingErrorsFromDiagnostic(diagnostic);
  if (missingErrorNames.length === 0) {
    return undefined;
  }

  const clause = findErrorClauseSpan(source, diagnostic.range);
  if (!clause) {
    return undefined;
  }

  const originalText = source.slice(clause.namesStart, clause.namesEnd);
  const existingErrorNames = parseErrorNames(originalText);
  const existing = new Set(existingErrorNames);
  const addedErrorNames = missingErrorNames.filter((name) => !existing.has(name));
  if (addedErrorNames.length === 0) {
    return undefined;
  }

  const updatedErrorNames = [...existingErrorNames, ...addedErrorNames];
  return {
    addedErrorNames,
    newText: formatErrorsClauseReplacement(originalText, updatedErrorNames),
    range: {
      end: positionAt(source, clause.namesEnd),
      start: positionAt(source, clause.namesStart)
    }
  };
}

/**
 * Extracts missing error names from compiler help text.
 * @param diagnostic Diagnostic carrying propagated-error mismatch help.
 * @returns Missing error names in suggested insertion order.
 */
function missingErrorsFromDiagnostic(diagnostic: OpalescentDiagnostic): string[] {
  const text = `${diagnostic.help ?? ''}\n${diagnostic.message}`;
  const match = /Suggested fix:\s*Add\s+(.+?)\s+to\s+the\s+errors\s+list\b/s.exec(text);
  if (!match?.[1]) {
    return [];
  }
  return match[1]
    .split(',')
    .map((name) => name.trim())
    .filter((name) => name.length > 0);
}

/**
 * Locates the text span occupied by a function's declared error names.
 * @param source Complete source text.
 * @param diagnosticRange Range covering the function declaration diagnostic.
 * @returns Offset span between `errors` and the signature `=>`, if found.
 */
function findErrorClauseSpan(
  source: string,
  diagnosticRange: OpalescentDiagnosticRange
): ErrorClauseSpan | undefined {
  const rangeStart = offsetAt(source, diagnosticRange.start);
  const rangeEnd = offsetAt(source, diagnosticRange.end);
  const searchEnd = Math.max(rangeEnd, rangeStart + 1);
  const searchText = source.slice(rangeStart, searchEnd);
  const errorsMatch = /\berrors\b/.exec(searchText);
  if (!errorsMatch) {
    return undefined;
  }

  const errorsOffset = rangeStart + errorsMatch.index;
  const namesStart = errorsOffset + errorsKeyword.length;
  const namesEnd = source.indexOf('=>', namesStart);
  if (namesEnd < 0) {
    return undefined;
  }
  return { namesEnd, namesStart };
}

/**
 * Parses comma-separated source error names from an errors clause.
 * @param text Source text between `errors` and `=>`.
 * @returns Declared error names without whitespace or empty entries.
 */
function parseErrorNames(text: string): string[] {
  return text
    .split(',')
    .map((name) => name.trim())
    .filter((name) => name.length > 0);
}

/**
 * Formats the replacement text for an updated errors clause.
 * @param originalText Original source text between `errors` and `=>`.
 * @param errorNames Updated error names to render.
 * @returns Replacement preserving one-line or multiline clause shape.
 */
function formatErrorsClauseReplacement(originalText: string, errorNames: string[]): string {
  if (!originalText.includes('\n')) {
    return ` ${errorNames.join(', ')} `;
  }

  const lineEnding = originalText.includes('\r\n') ? '\r\n' : '\n';
  const indent = inferErrorsClauseIndent(originalText);
  return `${lineEnding}${indent}${errorNames.join(`,${lineEnding}${indent}`)}${lineEnding}`;
}

/**
 * Infers indentation for each error item in a multiline errors clause.
 * @param originalText Original source text between `errors` and `=>`.
 * @returns Whitespace prefix for rendered error names.
 */
function inferErrorsClauseIndent(originalText: string): string {
  const match = /\r?\n([ \t]*)\S/.exec(originalText);
  return match?.[1] ?? '    ';
}

/**
 * Converts a diagnostic position to a UTF-16 source offset.
 * @param source Complete source text.
 * @param position Zero-based diagnostic position.
 * @returns UTF-16 offset in the source string.
 */
function offsetAt(source: string, position: OpalescentDiagnosticPosition): number {
  let line = 0;
  let character = 0;
  for (let index = 0; index < source.length; index += 1) {
    if (line === position.line && character === position.character) {
      return index;
    }
    if (source[index] === '\n') {
      line += 1;
      character = 0;
    } else {
      character += 1;
    }
  }
  return source.length;
}

/**
 * Converts a UTF-16 source offset to a diagnostic position.
 * @param source Complete source text.
 * @param offset UTF-16 source offset.
 * @returns Zero-based diagnostic position.
 */
function positionAt(source: string, offset: number): OpalescentDiagnosticPosition {
  let line = 0;
  let character = 0;
  const end = Math.min(offset, source.length);
  for (let index = 0; index < end; index += 1) {
    if (source[index] === '\n') {
      line += 1;
      character = 0;
    } else {
      character += 1;
    }
  }
  return { character, line };
}

/**
 * Validates one diagnostic object from the compiler payload.
 * @param value Candidate diagnostic value.
 * @param index Index of the diagnostic in the report.
 * @returns A validated diagnostic object.
 */
function validateDiagnostic(value: unknown, index: number): OpalescentDiagnostic {
  if (!isRecord(value)) {
    throw new Error(`diagnostics[${index}] must be an object`);
  }
  if (typeof value.source_path !== 'string') {
    throw new Error(`diagnostics[${index}].source_path must be a string`);
  }
  if (typeof value.severity !== 'string' || !severities.has(value.severity)) {
    throw new Error(`diagnostics[${index}].severity is invalid`);
  }
  if (typeof value.phase !== 'string') {
    throw new Error(`diagnostics[${index}].phase must be a string`);
  }
  if (value.code !== undefined && typeof value.code !== 'string') {
    throw new Error(`diagnostics[${index}].code must be a string when present`);
  }
  if (typeof value.message !== 'string') {
    throw new Error(`diagnostics[${index}].message must be a string`);
  }
  if (value.help !== undefined && typeof value.help !== 'string') {
    throw new Error(`diagnostics[${index}].help must be a string when present`);
  }
  const range = validateRange(value.range, index);
  return {
    code: value.code,
    help: value.help,
    message: value.message,
    phase: value.phase,
    range,
    severity: value.severity as OpalescentDiagnosticSeverity,
    source_path: value.source_path
  };
}

/**
 * Validates a diagnostic source range.
 * @param value Candidate range value.
 * @param index Index of the owning diagnostic in the report.
 * @returns A validated source range.
 */
function validateRange(value: unknown, index: number): OpalescentDiagnosticRange {
  if (!isRecord(value)) {
    throw new Error(`diagnostics[${index}].range must be an object`);
  }
  return {
    end: validatePosition(value.end, `diagnostics[${index}].range.end`),
    start: validatePosition(value.start, `diagnostics[${index}].range.start`)
  };
}

/**
 * Validates a zero-based editor position.
 * @param value Candidate position value.
 * @param label Human-readable path used in validation errors.
 * @returns A validated source position.
 */
function validatePosition(value: unknown, label: string): OpalescentDiagnosticPosition {
  if (!isRecord(value) || !Number.isInteger(value.line) || !Number.isInteger(value.character)) {
    throw new Error(`${label} must contain integer line and character`);
  }
  const line = value.line as number;
  const character = value.character as number;
  return { character, line };
}

/**
 * Checks whether a value can be inspected as an object record.
 * @param value Value to inspect.
 * @returns Whether the value is a non-null object.
 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
