import path from 'node:path';

/**
 * Checks whether a path points at an Opalescent source file.
 * @param filePath Path to inspect.
 * @returns Whether the path has an Opalescent extension.
 */
export function isOpalescentFile(filePath: string): boolean {
  return filePath.endsWith('.op') || filePath.endsWith('.types.op');
}

/**
 * Walks upward from a file or directory to find the nearest `opal.toml`.
 * @param startPath File or directory path where the search starts.
 * @param exists Predicate used to test candidate manifest paths.
 * @returns The nearest project root, or undefined when none is found.
 */
export function findProjectRoot(startPath: string, exists: (candidate: string) => boolean): string | undefined {
  let current = path.resolve(startPath);
  if (isOpalescentFile(current)) {
    current = path.dirname(current);
  }

  let previous = '';
  while (previous !== current) {
    if (exists(path.join(current, 'opal.toml'))) {
      return current;
    }

    previous = current;
    current = path.dirname(current);
  }

  return undefined;
}

/**
 * Resolves a relative Opalescent import specifier to an existing source file.
 * @param importerFilePath File containing the import.
 * @param specifier Import module specifier such as `./editor.types`.
 * @param exists Predicate used to test candidate source paths.
 * @returns The resolved source file path, or undefined for external/missing imports.
 */
export function resolveLocalImportPath(
  importerFilePath: string,
  specifier: string,
  exists: (candidate: string) => boolean
): string | undefined {
  if (!specifier.startsWith('./') && !specifier.startsWith('../')) {
    return undefined;
  }

  const basePath = path.resolve(path.dirname(importerFilePath), specifier);
  const candidates = isOpalescentFile(basePath) ? [basePath] : [`${basePath}.op`, `${basePath}.types.op`];
  return candidates.find((candidate) => exists(candidate));
}
