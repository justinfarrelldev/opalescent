export interface OpalescentCommandContext {
  filePath: string;
  projectRoot?: string;
}

export function checkArgsForContext(context: OpalescentCommandContext): string[] {
  if (context.projectRoot) {
    return ['check', '--json', '--project', context.projectRoot];
  }
  return ['check', '--json', context.filePath];
}

export function buildArgsForContext(context: OpalescentCommandContext): string[] {
  if (context.projectRoot) {
    return ['build'];
  }
  return [context.filePath];
}

export function runArgsForContext(context: OpalescentCommandContext): string[] {
  if (context.projectRoot) {
    return ['run'];
  }
  return ['run', context.filePath];
}

export function formatArgs(sourcePath: string, outputPath: string): string[] {
  return ['fmt', '--output', outputPath, sourcePath];
}
