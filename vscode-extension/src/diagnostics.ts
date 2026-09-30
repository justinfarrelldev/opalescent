export type OpalescentDiagnosticSeverity = 'error' | 'hint' | 'information' | 'warning';

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

export interface OpalescentDiagnosticReport {
  diagnostics: OpalescentDiagnostic[];
  success: boolean;
}

const severities = new Set(['error', 'warning', 'information', 'hint']);

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
