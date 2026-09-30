import path from 'node:path';

export function isOpalescentFile(filePath: string): boolean {
  return filePath.endsWith('.op') || filePath.endsWith('.types.op');
}

export function findProjectRoot(startPath: string, exists: (candidate: string) => boolean): string | undefined {
  let current = path.resolve(startPath);
  if (isOpalescentFile(current)) {
    current = path.dirname(current);
  }

  while (true) {
    if (exists(path.join(current, 'opal.toml'))) {
      return current;
    }

    const parent = path.dirname(current);
    if (parent === current) {
      return undefined;
    }
    current = parent;
  }
}
