import path from 'node:path';

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

export function shellQuote(value: string): string {
  if (process.platform === 'win32') {
    return `"${value.replaceAll('"', '\\"')}"`;
  }
  return `'${value.replaceAll("'", "'\\''")}'`;
}
