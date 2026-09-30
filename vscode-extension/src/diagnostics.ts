export type OpalescentDiagnosticSeverity = 'error' | 'warning' | 'information' | 'hint';

export interface OpalescentDiagnosticPosition {
  line: number;
  character: number;
}

export interface OpalescentDiagnosticRange {
  start: OpalescentDiagnosticPosition;
  end: OpalescentDiagnosticPosition;
}

export interface OpalescentDiagnostic {
  source_path: string;
  severity: OpalescentDiagnosticSeverity;
  phase: string;
  code?: string;
  message: string;
  help?: string;
  range: OpalescentDiagnosticRange;
}

export interface OpalescentDiagnosticReport {
  success: boolean;
  diagnostics: OpalescentDiagnostic[];
}

const severities = new Set(['error', 'warning', 'information', 'hint']);

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
  return { success: parsed.success, diagnostics };
}

export function diagnosticsByFile(report: OpalescentDiagnosticReport): Map<string, OpalescentDiagnostic[]> {
  const grouped = new Map<string, OpalescentDiagnostic[]>();
  for (const diagnostic of report.diagnostics) {
    const existing = grouped.get(diagnostic.source_path) ?? [];
    existing.push(diagnostic);
    grouped.set(diagnostic.source_path, existing);
  }
  return grouped;
}

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
    source_path: value.source_path,
    severity: value.severity as OpalescentDiagnosticSeverity,
    phase: value.phase,
    code: value.code,
    message: value.message,
    help: value.help,
    range
  };
}

function validateRange(value: unknown, index: number): OpalescentDiagnosticRange {
  if (!isRecord(value)) {
    throw new Error(`diagnostics[${index}].range must be an object`);
  }
  return {
    start: validatePosition(value.start, `diagnostics[${index}].range.start`),
    end: validatePosition(value.end, `diagnostics[${index}].range.end`)
  };
}

function validatePosition(value: unknown, label: string): OpalescentDiagnosticPosition {
  if (!isRecord(value) || !Number.isInteger(value.line) || !Number.isInteger(value.character)) {
    throw new Error(`${label} must contain integer line and character`);
  }
  const line = value.line as number;
  const character = value.character as number;
  return { line, character };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
