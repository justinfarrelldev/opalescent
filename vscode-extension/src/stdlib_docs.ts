/* eslint-disable */

interface StdlibDocumentedCompletion {
  documentation: string;
  moduleName: string;
  name: string;
  typeOnly?: boolean;
}

const exactDocumentation = new Map<string, string>([
  ['standard:print', 'Prints a displayable value using Opalescent\'s standard display lowering. Use it for quick diagnostics, tutorials, and simple command-line output when you do not need explicit flushing, cursor control, or terminal-session trust boundaries.'],
  ['standard:println', 'Prints one string line. Use it when you specifically want line-oriented string output; use `print_text_sync` and `flush_standard_output_sync` when you need explicit write/flush control.'],
  ['standard:take_input', 'Reads one line from standard input without the trailing newline. Use it for simple interactive prompts; handle `StandardInputReadError` so EOF and input failures are explicit.'],
  ['standard:append', 'Returns a new array with one value appended. Prefer member-style `values.push(value)` while building mutable arrays, and use this helper when a function-style array append is clearer.'],
  ['standard:array_length', 'Returns the number of elements in an array. Most source should use `values.length`; import this only when you need the lowering helper as a first-class standard function.'],
  ['standard:array_filled', 'Creates an array containing `count` copies of one value. Use it to build fixed-size initial buffers before filling positions; avoid it when every element needs distinct initialization logic.'],
  ['standard:reserve', 'Returns an array with capacity reserved for at least the requested number of elements. Use it before many pushes when you know the approximate final size and want to reduce reallocations.'],
  ['standard:clear', 'Returns an empty array of the same element type. Use it when you want to discard all elements while preserving the element type in generic code.'],
  ['standard:bytes_new', 'Creates an empty immutable `Bytes` buffer. New code can also use `new Bytes`; use byte buffers for binary data rather than UTF-8 text.'],
  ['standard:bytes_length', 'Returns the byte length of a `Bytes` buffer. Source usually uses `buffer.length`; use this helper when a function-style call is needed.'],
  ['standard:bytes_to_hex', 'Encodes a byte buffer as lowercase hexadecimal text. Use it for stable logs, checksums, fixture output, and wire/debug formats where binary data must be printable.'],
  ['standard:bytes_from_hex', 'Decodes hexadecimal text into `Bytes`. Use it for test fixtures, hashes, IDs, or wire-format values that are represented as hex; handle `HexDecodeError` for odd-length or non-hex input.'],
  ['standard:bytes_concatenate', 'Returns a new byte buffer containing `left` followed by `right`. Use it for small binary fragments; prefer future streaming or builder support for large repeated concatenation.'],
  ['standard:bytes_slice', 'Returns bytes in the half-open range `[start, end)`. Use it for protocol fields and binary headers after validating offsets; handle `SliceRangeError` for inverted or out-of-bounds ranges.'],
  ['standard:string_length', 'Returns the number of Unicode scalar values in a string. Most source should use `text.length`; use this helper only when a function value or explicit import is clearer.'],
  ['standard:string_find_index_or', 'Finds the first Unicode-scalar index of a search string, returning your fallback when the pattern is empty or absent. Use it when missing text is an expected branch rather than an error.'],
  ['standard:string_find_last_index_of_text', 'Finds the last Unicode-scalar index of a non-empty search string. Use it when absence should be handled as an error with `guard` or `propagate`.'],
  ['standard:string_split_lines', 'Splits text into logical lines, recognizing `\\n`, `\\r\\n`, and bare `\\r`. Use it for human-readable files and command output; it does not include line terminators in the result.'],
  ['standard:string_is_blank', 'Reports whether a string is empty or contains only Unicode whitespace. Use it to validate user input and config fields before parsing or storing them.'],
  ['standard:string_trim_whitespace', 'Returns a copy with leading and trailing Unicode whitespace removed. Use it before parsing user input or config values when surrounding whitespace should be ignored.'],
  ['standard:string_take_prefix', 'Returns the first `count` Unicode scalar values. Use it for scalar-aware previews or limits; handle range errors when the count comes from user input.'],
  ['standard:string_take_suffix', 'Returns the last `count` Unicode scalar values. Use it for scalar-aware file extensions, endings, or display summaries; handle range errors for dynamic counts.'],
  ['standard:string_extract_range', 'Extracts the half-open Unicode-scalar range `[start, end)`. Use it when you need a substring by Unicode scalar positions rather than bytes; handle ordering and bounds errors.'],
  ['standard:string_insert_at', 'Returns a new string with text inserted at a Unicode-scalar position. Use it for small editor-style transformations where indices are already scalar positions.'],
  ['standard:string_delete_range', 'Returns a new string with a Unicode-scalar range removed. Use it for small text edits after validating selection bounds.'],
  ['standard:string_replace_range', 'Returns a new string with a Unicode-scalar range replaced by another string. Use it for simple edit operations where byte offsets would be unsafe.'],
  ['standard:terminal_text_cell_width', 'Measures how many terminal display cells a string occupies under Opalescent\'s Unicode width policy. Use it for terminal layout, alignment, clipping, and cursor math.'],
  ['standard:terminal_text_clip_to_cells', 'Clips text to the largest grapheme-boundary prefix that fits within a cell limit. Use it before drawing to fixed-width terminal regions so combining marks and emoji sequences are not split.'],
  ['standard:string_join', 'Joins strings with a separator. Use it to render line arrays or small accumulated text; handle `AllocationFailureError` because the result allocates.'],
  ['standard:string_builder_new', 'Creates an empty string builder. Use it when repeated concatenation would be noisy or inefficient and you want explicit push/finish steps.'],
  ['standard:string_builder_push', 'Appends text to a string builder. Use it while accumulating output incrementally; handle builder-finished and allocation errors.'],
  ['standard:string_builder_finish', 'Finishes a string builder and returns the accumulated string. Use it exactly once when construction is complete; later push or finish calls fail.'],
  ['standard:path_from', 'Wraps raw text as a `FilesystemPath`. Use it at the boundary where user/config text becomes a path value for filesystem APIs.'],
  ['standard:join_path_components', 'Joins a base filesystem path with child components and normalizes separators. Use it instead of string concatenation for portable path construction.'],
  ['standard:path_parent_directory', 'Returns the parent directory of a filesystem path. Use it when walking upward or deriving output directories from file paths.'],
  ['standard:path_file_name', 'Returns the final path component, or an empty string when there is none. Use it for display names and extension checks, not for security decisions by itself.'],
  ['standard:path_file_extension', 'Returns the text after the final dot in the file name, or an empty string when no extension exists. Use it for simple extension-based routing.'],
  ['standard:normalize_path', 'Normalizes path syntax by removing redundant separators and simple `.`/`..` segments. Use it to canonicalize user-facing path text before display or comparison; it is not a permission check.'],
  ['standard:path_to_string', 'Converts a `FilesystemPath` back to its textual representation. Use it for messages, logs, and passing paths to APIs that still require strings.'],
  ['standard:absolute_path_sync', 'Resolves a filesystem path to an absolute path using the host filesystem. Use it when relative paths must be anchored before display, storage, or process changes; handle invalid and permission failures.'],
  ['standard:read_contents_sync', 'Reads an entire file as raw bytes. Use it for binary formats and exact byte roundtrips rather than human-readable UTF-8 text.'],
  ['standard:read_text_sync', 'Reads an entire file as UTF-8 text. Use it for human-readable UTF-8 files when loading the whole file is acceptable; handle file, permission, directory, and `InvalidUtf8Error` failures explicitly.'],
  ['standard:read_first_line_sync', 'Reads only the first line of a text file. Use it for headers, shebang-like probes, or small metadata files without loading the whole file.'],
  ['standard:read_lines_sync', 'Reads a UTF-8 text file into an array of lines. Use it for line-oriented configuration and reports; use `read_text_sync` when preserving exact original line endings matters.'],
  ['standard:read_bytes_at_offset_sync', 'Reads a byte range from a file at a specific offset. Use it for headers, fixed records, or chunked binary processing; handle offset and file errors.'],
  ['standard:write_contents_sync', 'Overwrites or creates a file with raw bytes. Use it for binary output when partial writes are acceptable for your application.'],
  ['standard:write_text_sync', 'Overwrites or creates a file with UTF-8 text. Use it for ordinary text output when a failed run leaving partial content is acceptable.'],
  ['standard:write_contents_atomic_sync', 'Writes raw bytes through a temporary file and replaces the target. Use it for files that should not be left partially written after failure.'],
  ['standard:write_text_atomic_sync', 'Writes UTF-8 text through a temporary file and replaces the target. Use it for config files, manifests, and generated text where partial output would be harmful.'],
  ['standard:append_contents_sync', 'Appends raw bytes to an existing file. Use it for binary logs or journals where the file must already exist.'],
  ['standard:append_text_sync', 'Appends UTF-8 text to an existing file. Use it for simple logs and reports where preserving previous content matters.'],
  ['standard:write_bytes_at_offset_sync', 'Writes bytes at a specific file offset. Use it for fixed-layout files after validating offsets; handle out-of-range and filesystem errors.'],
  ['standard:create_file_sync', 'Creates a new empty file and fails if it already exists. Use it when accidental overwrite would be a bug.'],
  ['standard:delete_file_sync', 'Deletes a file and fails for missing paths or directories. Use it for file cleanup after checking that directory deletion is not intended.'],
  ['standard:copy_file_sync', 'Copies one file to another path. Use it for simple file duplication; handle source, destination, permission, directory, and capacity errors.'],
  ['standard:move_path_sync', 'Moves or renames a filesystem path. Use it for same-filesystem renames and application-level moves; handle missing source and existing destination failures.'],
  ['standard:path_exists_sync', 'Checks whether a path exists while still reporting invalid-path and permission failures. Use it when those failures should not be collapsed into `false`.'],
  ['standard:read_metadata_sync', 'Reads file metadata while following symlinks. Use it when you need size, directory status, symlink status, or modification time for the target.'],
  ['standard:read_metadata_nofollow_sync', 'Reads metadata for the path itself without following symlinks. Use it when symlink identity matters.'],
  ['standard:create_directory_sync', 'Creates exactly one directory and fails if it already exists. Use it when existing output directories should be reported rather than silently accepted.'],
  ['standard:create_directory_recursive_sync', 'Creates a directory and any missing parents. Use it for application setup where existing parent directories are expected.'],
  ['standard:delete_directory_sync', 'Deletes an empty directory. Use it when recursive deletion would be too dangerous or surprising.'],
  ['standard:delete_directory_recursive_sync', 'Deletes a directory tree recursively. Use it only when removing all descendants is intended and safe.'],
  ['standard:list_directory_sync', 'Lists entries inside a directory as filesystem paths. Use it for directory traversal; apply explicit ordering in tests if deterministic output matters.'],
  ['standard:is_file_sync', 'Reports whether a path is a file, following symlinks. Use it for ordinary user-facing path checks.'],
  ['standard:is_file_nofollow_sync', 'Reports whether the path itself is a file without following symlinks. Use it when symlink behavior is security- or policy-relevant.'],
  ['standard:is_directory_sync', 'Reports whether a path is a directory, following symlinks. Use it before directory-only operations when symlink targets should count.'],
  ['standard:is_directory_nofollow_sync', 'Reports whether the path itself is a directory without following symlinks. Use it when symlink targets must not be followed.'],
  ['standard:print_text_sync', 'Writes text to standard output without adding a newline. Use it when you need exact output control and explicit error handling.'],
  ['standard:flush_standard_output_sync', 'Flushes standard output. Use it after buffered writes when output must be visible before the program continues or exits.'],
  ['standard:stdout_writer', 'Acquires a standard-output writer handle. Use it when several explicit writes and flushes should share one handle.'],
  ['standard:writer_write_sync', 'Writes text through an acquired stdout writer. Use it after `stdout_writer` when explicit handle-based output is clearer than global helpers.'],
  ['standard:writer_flush_sync', 'Flushes an acquired stdout writer. Use it to force buffered writer output to the host.'],
  ['standard:stdout_terminal', 'Acquires a terminal-control handle for standard output. Use it before ANSI capability checks and direct terminal operations.'],
  ['standard:terminal_supports_ansi', 'Reports whether a stdout terminal handle supports ANSI control sequences. Use it before emitting ANSI-dependent rendering paths.'],
  ['standard:terminal_clear_screen_on_sync', 'Clears the screen through a specific terminal handle. Use it when you already manage stdout terminal acquisition yourself.'],
  ['standard:terminal_move_cursor_on_sync', 'Moves the cursor through a specific terminal handle. Use it for handle-based terminal rendering after validating row and column values.'],
  ['standard:terminal_draw_rows_sync', 'Draws multiple rows through a terminal handle. Use it for simple full-screen redraws outside the newer terminal-session API.'],
  ['standard:terminal_clear_screen_sync', 'Clears standard output\'s terminal using the convenience global path. Use it for simple programs that do not need an explicit terminal handle.'],
  ['standard:terminal_move_cursor_sync', 'Moves the cursor on standard output\'s terminal using the convenience global path. Use it for simple terminal programs with explicit error handling.'],
  ['standard:sleep_ms_sync', 'Blocks the current thread for a number of milliseconds. Use it for simple demos and fixed delays, not for responsive event loops that need cancellation.'],
  ['standard:frame_clock_new', 'Creates a fixed-rate frame clock. Use it for simple render loops that should wait between frames at a configured rate.'],
  ['standard:frame_clock_wait_next_sync', 'Waits until the next frame deadline and updates the clock. Use it once per loop iteration to pace animation or simulation.'],
  ['process:current_working_directory_sync', 'Returns the process current working directory as a filesystem path. Use it to anchor relative user paths or display process location.'],
  ['process:current_executable_path_sync', 'Returns the path of the running executable. Use it to locate resources relative to the installed binary.'],
  ['process:current_executable_directory_sync', 'Returns the directory containing the running executable. Use it as a base for bundled resources or sibling tools.'],
  ['process:set_current_working_directory_sync', 'Changes the process current working directory. Use it sparingly because it affects later relative path operations process-wide.'],
  ['process:get_environment_variable', 'Reads a required environment variable. Use it when absence should be an error rather than a default.'],
  ['process:get_environment_variable_or', 'Reads an environment variable with a fallback value. Use it for optional configuration with a clear default.'],
  ['process:environment_variable_exists', 'Checks whether an environment variable is present. Use it when only presence matters and the value is not needed.'],
  ['process:exit_process', 'Terminates the process with an exit code. Use it at the outer edge of command-line programs; treat it as never returning even though its current type is `void`.'],
  ['math:sqrt', 'Computes the square root of a float64 value. Use it for ordinary floating-point math where IEEE behavior is acceptable.'],
  ['math:abs', 'Computes the absolute value of an int32. Use it for small integer distances or magnitudes after considering overflow at the minimum int32 value.'],
  ['math:sin', 'Computes the sine of a float64 angle in radians. Use it for trigonometry and periodic calculations.'],
  ['math:cos', 'Computes the cosine of a float64 angle in radians. Use it for trigonometry and periodic calculations.']
]);

/**
 * Adds purpose-oriented docs to importable standard-library function completions.
 * @param completion Static completion metadata from the extension.
 * @returns Completion metadata with stronger documentation when this helper knows the symbol.
 */
export function withStdlibDocumentation<T extends StdlibDocumentedCompletion>(completion: T): T {
  if (completion.typeOnly) {
    return completion;
  }

  const exact = exactDocumentation.get(`${completion.moduleName}:${completion.name}`);
  if (exact) {
    return { ...completion, documentation: exact };
  }

  const generated = generatedDocumentation(completion.moduleName, completion.name);
  return generated ? { ...completion, documentation: generated } : completion;
}

function generatedDocumentation(moduleName: string, name: string): string | undefined {
  if (moduleName === 'standard.numeric' || (moduleName === 'standard' && isNumericConversionName(name))) {
    return numericConversionDocumentation(name);
  }
  if (moduleName === 'math' && name.startsWith('random_')) {
    const typeName = name.slice('random_'.length);
    return `Returns a pseudo-random ${typeName} in the inclusive range from \`min\` to \`max\`. Use it for simple non-cryptographic choices such as dice rolls, randomized examples, or fixture variation; do not use it for secrets or security decisions.`;
  }
  if (moduleName === 'standard' && name.startsWith('string_to_')) {
    const typeName = name.slice('string_to_'.length);
    return `Parses decimal text into ${typeName}. Use it for command-line input, config values, and file fields after trimming or validating surrounding text; handle \`ParseError\` for empty, malformed, or out-of-range values.`;
  }
  if (moduleName === 'standard' && /^[a-z0-9]+_to_string$/.test(name)) {
    const sourceType = name.slice(0, -'_to_string'.length);
    return `Formats a ${sourceType === 'bool' ? 'boolean' : sourceType} value as text. Use it when building messages, logs, or explicit strings outside interpolation.`;
  }
  return undefined;
}

function isNumericConversionName(name: string): boolean {
  return /^((?:u?int|float)\d+)_to_((?:u?int|float)\d+)$/.test(name);
}

function numericConversionDocumentation(name: string): string | undefined {
  const match = /^([a-z0-9]+)_to_([a-z0-9]+)$/.exec(name);
  if (!match) {
    return undefined;
  }
  const [, source, destination] = match;
  return `Performs a checked conversion from ${source} to ${destination}, succeeding only when the value can be represented exactly. Use it instead of \`as\` when the value is only known at runtime or when narrowing, signedness changes, or float-to-integer conversion could fail; handle \`IntegerRangeError\` for rejected values.`;
}
