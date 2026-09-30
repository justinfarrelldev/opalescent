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
