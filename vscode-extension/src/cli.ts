export interface OpalescentCommandContext {
  filePath: string;
  projectRoot?: string;
}

/**
 * Creates compiler arguments for JSON diagnostics.
 * @param context File and optional project root for the command.
 * @returns Arguments for `opal check`.
 */
export function checkArgsForContext(context: OpalescentCommandContext): string[] {
  if (context.projectRoot) {
    return ['check', '--json', '--project', context.projectRoot];
  }
  return ['check', '--json', context.filePath];
}

/**
 * Creates compiler arguments for a build command.
 * @param context File and optional project root for the command.
 * @returns Arguments for project or single-file build execution.
 */
export function buildArgsForContext(context: OpalescentCommandContext): string[] {
  if (context.projectRoot) {
    return ['build'];
  }
  return [context.filePath];
}

/**
 * Creates compiler arguments for a run command.
 * @param context File and optional project root for the command.
 * @returns Arguments for project or single-file run execution.
 */
export function runArgsForContext(context: OpalescentCommandContext): string[] {
  if (context.projectRoot) {
    return ['run'];
  }
  return ['run', context.filePath];
}

/**
 * Creates compiler arguments for formatting a source file to an output path.
 * @param sourcePath Source file to format.
 * @param outputPath Destination file for formatted text.
 * @returns Arguments for `opal fmt`.
 */
export function formatArgs(sourcePath: string, outputPath: string): string[] {
  return ['fmt', '--output', outputPath, sourcePath];
}
