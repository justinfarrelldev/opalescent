import path from 'node:path';

/**
 * Builds the ordered list of compiler binary candidates to try.
 * @param workspaceRoots Workspace root directories that may contain build outputs.
 * @param configuredPath Optional user-configured compiler path.
 * @returns Candidate binary paths in preference order, without duplicates.
 */
export function candidateBinaryPaths(workspaceRoots: string[], configuredPath?: string): string[] {
  const candidates: string[] = [];
  if (configuredPath?.trim()) {
    candidates.push(configuredPath.trim());
  }

  const executableName = process.platform === 'win32' ? 'opalescent.exe' : 'opalescent';
  for (const root of workspaceRoots) {
    candidates.push(path.join(root, 'target', 'debug', executableName));
    candidates.push(path.join(root, 'target', 'release', executableName));
  }

  candidates.push('opalescent');
  candidates.push('opal');
  return [...new Set(candidates)];
}

/**
 * Quotes a value for safe use in a VS Code terminal command line.
 * @param value Raw argument text to quote.
 * @returns Shell-quoted argument text for the current platform.
 */
export function shellQuote(value: string): string {
  if (process.platform === 'win32') {
    return `"${value.replaceAll('"', '\\"')}"`;
  }
  return `'${value.replaceAll("'", "'\\''")}'`;
}
