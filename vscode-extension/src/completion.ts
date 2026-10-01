import type { OpalescentSymbol, OpalescentSymbolKind } from './symbols.js';

import { moduleSpecifierForLocalImport } from './project.js';

export type OpalescentCompletionKind = 'keyword' | 'snippet' | OpalescentSymbolKind;

export interface OpalescentCompletionImportEdit {
  character: number;
  line: number;
  text: string;
}

export interface OpalescentCompletionItem {
  autoImportEdit?: OpalescentCompletionImportEdit;
  detail?: string;
  documentation?: string;
  insertText: string;
  isSnippet?: boolean;
  kind: OpalescentCompletionKind;
  name: string;
  sourceModule?: string;
  symbol?: OpalescentSymbol;
}

export interface OpalescentCompletionRequest {
  currentFilePath: string;
  source: string;
  symbols: OpalescentSymbol[];
}

interface ExternalCompletionDefinition {
  detail: string;
  documentation: string;
  moduleName: string;
  name: string;
  typeOnly?: boolean;
}

const sameFileCompletionKinds = new Set<OpalescentSymbolKind>([
  'entry',
  'error_set',
  'function',
  'let',
  'parameter',
  'type'
]);
const autoImportCompletionKinds = new Set<OpalescentSymbolKind>(['error_set', 'function', 'let', 'type']);
const typeOnlyImportKinds = new Set<OpalescentSymbolKind>(['error_set', 'type']);
const keywordNames = [
  'and',
  'as',
  'band',
  'boolean',
  'bor',
  'break',
  'bnot',
  'bshl',
  'bshr',
  'bushr',
  'bxor',
  'continue',
  'div_euclid',
  'else',
  'entry',
  'error',
  'errors',
  'f',
  'false',
  'float32',
  'float64',
  'for',
  'from',
  'guard',
  'if',
  'import',
  'in',
  'int8',
  'int16',
  'int32',
  'int64',
  'into',
  'is',
  'let',
  'loop',
  'mod_euclid',
  'mutable',
  'new',
  'not',
  'or',
  'propagate',
  'public',
  'pure',
  'ref',
  'return',
  'set',
  'string',
  'true',
  'type',
  'type_of',
  'uint8',
  'uint16',
  'uint32',
  'uint64',
  'untested',
  'using',
  'void',
  'while',
  'with',
  'xor'
] as const;
const primitiveTypeDocumentation = new Map<string, string>([
  [
    'boolean',
    'Boolean truth value. Use `true` and `false`, and combine boolean expressions with word operators such as `and`, `or`, and `not`.'
  ],
  ['float32', '32-bit floating-point number. Use when a compact floating-point representation is required.'],
  ['float64', '64-bit floating-point number. Prefer this for general floating-point calculations.'],
  ['int8', '8-bit signed integer. Range: -128 to 127.'],
  ['int16', '16-bit signed integer. Range: -32,768 to 32,767.'],
  ['int32', '32-bit signed integer. Range: -2,147,483,648 to 2,147,483,647. Good default integer width for small examples.'],
  [
    'int64',
    '64-bit signed integer. Range: -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807. Commonly used for counts, lengths, and filesystem-sized values.'
  ],
  [
    'string',
    'Owned Unicode text value. Strings use single quotes, support interpolation with `{expression}`, expose `.length`, and support fallible Unicode-scalar `.at(index)` reads.'
  ],
  ['uint8', '8-bit unsigned integer. Range: 0 to 255.'],
  ['uint16', '16-bit unsigned integer. Range: 0 to 65,535.'],
  ['uint32', '32-bit unsigned integer. Range: 0 to 4,294,967,295.'],
  ['uint64', '64-bit unsigned integer. Range: 0 to 18,446,744,073,709,551,615.'],
  ['void', 'The unit-like return type for functions that intentionally return no meaningful value. End such functions with `return void`.']
]);
const keywordDocumentation = new Map<string, string>([
  ['entry', 'Declares the program entry point. Runnable projects normally define `entry main = f(args: string[]): void =>`.'],
  ['guard', 'Handles a fallible expression locally. Use `guard call() into value else err =>` when the success value should be bound.'],
  ['if', 'Runs an indented block when a boolean condition is true. Use `else:` for the alternate branch.'],
  ['import', 'Imports named symbols from a standard-library module or relative local module. Imports belong at the top of the file.'],
  ['let', 'Declares an immutable binding. Use `let mutable` when reassignment is needed.'],
  ['propagate', 'Propagates a fallible call\'s error to the current function\'s caller. The current function must declare compatible `errors`.'],
  ['type', 'Declares a product, sum, or enum type. Type declarations belong in `.types.op` files.'],
  ['while', 'Repeats an indented block while a boolean condition is true.']
]);
const externalCompletions: readonly ExternalCompletionDefinition[] = [
  {
    detail: 'standard: print(value): void',
    documentation: 'Prints a displayable value followed by the runtime\'s normal output behavior. Strings, booleans, integers, floats, and other supported display values are accepted.',
    moduleName: 'standard',
    name: 'print'
  },
  {
    detail: 'standard: println(text: string): void',
    documentation: 'Prints one string line. This is the string-only line-output helper from the `standard` module.',
    moduleName: 'standard',
    name: 'println'
  },
  {
    detail: 'standard: take_input(): string errors StandardInputReadError',
    documentation: 'Reads one line from standard input without the trailing newline. Fails with `StandardInputReadError` for input failures or empty EOF.',
    moduleName: 'standard',
    name: 'take_input'
  },
  ...numericParserCompletions(),
  ...numericToStringCompletions(),
  ...stringHelperCompletions(),
  ...arrayHelperCompletions(),
  ...bytesHelperCompletions(),
  ...filesystemHelperCompletions(),
  ...stdoutAndTerminalCompletions(),
  ...timeCompletions(),
  ...mathCompletions(),
  ...processCompletions(),
  ...standardErrorSetCompletions()
];
const snippetCompletions: readonly OpalescentCompletionItem[] = [
  {
    detail: 'Snippet: entry point',
    documentation: 'Creates an `entry main` function with the standard command-line argument shape.',
    insertText: 'entry main = f(args: string[]): void =>\n    ${0:return void}',
    isSnippet: true,
    kind: 'snippet',
    name: 'entry main'
  },
  {
    detail: 'Snippet: function declaration',
    documentation: 'Creates a top-level function declaration using Opalescent indentation syntax.',
    insertText: 'let ${1:name} = f(${2:args}): ${3:void} =>\n    ${0:return void}',
    isSnippet: true,
    kind: 'snippet',
    name: 'function'
  },
  {
    detail: 'Snippet: let binding',
    documentation: 'Creates an immutable let binding.',
    insertText: 'let ${1:name} = ${0:value}',
    isSnippet: true,
    kind: 'snippet',
    name: 'let binding'
  },
  {
    detail: 'Snippet: import from',
    documentation: 'Creates a named import declaration.',
    insertText: 'import ${1:symbol} from ${0:./module}',
    isSnippet: true,
    kind: 'snippet',
    name: 'import from'
  },
  {
    detail: 'Snippet: if block',
    documentation: 'Creates an indented if block.',
    insertText: 'if ${1:condition}:\n    ${0}',
    isSnippet: true,
    kind: 'snippet',
    name: 'if block'
  },
  {
    detail: 'Snippet: if/else block',
    documentation: 'Creates an indented if/else block.',
    insertText: 'if ${1:condition}:\n    ${2}\nelse:\n    ${0}',
    isSnippet: true,
    kind: 'snippet',
    name: 'if else block'
  },
  {
    detail: 'Snippet: while block',
    documentation: 'Creates an indented while block.',
    insertText: 'while ${1:condition}:\n    ${0}',
    isSnippet: true,
    kind: 'snippet',
    name: 'while block'
  },
  {
    detail: 'Snippet: for block',
    documentation: 'Creates an indented for-in loop.',
    insertText: 'for ${1:item} in ${2:items}:\n    ${0}',
    isSnippet: true,
    kind: 'snippet',
    name: 'for block'
  },
  {
    detail: 'Snippet: guard else',
    documentation: 'Creates a guard handler for a fallible expression when the success value is not needed.',
    insertText: 'guard ${1:fallible_call()} else ${2:err} =>\n    ${0}',
    isSnippet: true,
    kind: 'snippet',
    name: 'guard else'
  },
  {
    detail: 'Snippet: guard into else',
    documentation: 'Creates a guard handler that binds the successful result and handles the error branch.',
    insertText: 'guard ${1:fallible_call()} into ${2:value} else ${3:err} =>\n    ${0}',
    isSnippet: true,
    kind: 'snippet',
    name: 'guard into else'
  },
  {
    detail: 'Snippet: public product type',
    documentation: 'Creates a public product type declaration for `.types.op` files.',
    insertText: 'public type ${1:TypeName}:\n    ${2:field}: ${0:string}',
    isSnippet: true,
    kind: 'snippet',
    name: 'public type'
  }
];

/**
 * Builds numeric parser completions from the standard module.
 * @returns Standard numeric parser completion definitions.
 */
function numericParserCompletions(): ExternalCompletionDefinition[] {
  const integerTypes = ['int8', 'int16', 'int32', 'int64', 'uint8', 'uint16', 'uint32', 'uint64'];
  const floatTypes = ['float32', 'float64'];
  return [...integerTypes, ...floatTypes].map((typeName) => ({
    detail: `standard: string_to_${typeName}(text: string): ${typeName} errors ParseError`,
    documentation: `Parses decimal text into a ${typeName} value. Leading Unicode whitespace is skipped, the whole trimmed input must be valid, and invalid digits, empty input, or out-of-range values fail with \`ParseError\`.`,
    moduleName: 'standard',
    name: `string_to_${typeName}`
  }));
}

/**
 * Builds value-to-string conversion completions from the standard module.
 * @returns Standard conversion completion definitions.
 */
function numericToStringCompletions(): ExternalCompletionDefinition[] {
  const sourceTypes = ['int8', 'int16', 'int32', 'int64', 'uint8', 'uint16', 'uint32', 'uint64', 'float32', 'float64', 'bool'];
  return sourceTypes.map((sourceType) => ({
    detail: `standard: ${sourceType}_to_string(value: ${sourceType === 'bool' ? 'boolean' : sourceType}): string`,
    documentation: `Converts a ${sourceType === 'bool' ? 'boolean' : sourceType} value to its string representation. This helper does not declare errors.`,
    moduleName: 'standard',
    name: `${sourceType}_to_string`
  }));
}

/**
 * Builds string helper completions from the standard module.
 * @returns Standard string helper completion definitions.
 */
function stringHelperCompletions(): ExternalCompletionDefinition[] {
  return externalDefinitions('standard', [
    ['string_length', 'string_length(text: string): int64', 'Returns the number of Unicode scalar values in `text`. Source code usually uses the `.length` member syntax.'],
    ['string_find_index_or', 'string_find_index_or(text: string, search_text: string, fallback_index: int64): int64', 'Returns the first Unicode scalar index where `search_text` appears, or `fallback_index` when the search text is empty or not found.'],
    ['string_find_last_index_of_text', 'string_find_last_index_of_text(text: string, search_text: string): int64 errors StringEmptySearchTextError, StringPatternNotFoundError', 'Returns the last Unicode scalar index where `search_text` appears. Fails when the search text is empty or absent.'],
    ['string_split_lines', 'string_split_lines(text: string): string[] errors AllocationFailureError', 'Splits text into logical lines. Recognizes `\\n`, `\\r\\n`, and bare `\\r`, and does not add an extra trailing empty line for a final terminator.'],
    ['string_is_blank', 'string_is_blank(text: string): boolean', 'Returns true when the text is empty or every Unicode scalar has the Unicode White_Space property.'],
    ['string_trim_whitespace', 'string_trim_whitespace(text: string): string errors AllocationFailureError', 'Returns text with leading and trailing Unicode whitespace removed while preserving interior whitespace.'],
    ['string_take_prefix', 'string_take_prefix(text: string, count: int64): string errors StringNegativeCountError, StringRangeOutOfBoundsError, AllocationFailureError', 'Returns the first `count` Unicode scalar values from `text`. Negative counts and out-of-range counts fail.'],
    ['string_take_suffix', 'string_take_suffix(text: string, count: int64): string errors StringNegativeCountError, StringRangeOutOfBoundsError, AllocationFailureError', 'Returns the last `count` Unicode scalar values from `text`. Negative counts and out-of-range counts fail.'],
    ['string_extract_range', 'string_extract_range(text: string, start: int64, end: int64): string errors StringRangeOrderError, StringRangeOutOfBoundsError, AllocationFailureError', 'Returns the Unicode scalar range `[start, end)`. Reversed or out-of-bounds ranges fail.'],
    ['string_insert_at', 'string_insert_at(text: string, scalar_index: int64, inserted: string): string errors StringRangeOutOfBoundsError, AllocationFailureError', 'Returns a new string with `inserted` placed before the Unicode scalar at `scalar_index`.'],
    ['string_delete_range', 'string_delete_range(text: string, start: int64, end: int64): string errors StringRangeOrderError, StringRangeOutOfBoundsError, AllocationFailureError', 'Returns a new string with the Unicode scalar range `[start, end)` removed.'],
    ['string_replace_range', 'string_replace_range(text: string, start: int64, end: int64, replacement: string): string errors StringRangeOrderError, StringRangeOutOfBoundsError, AllocationFailureError', 'Returns a new string with the Unicode scalar range `[start, end)` replaced by `replacement`.'],
    ['terminal_text_cell_width', 'terminal_text_cell_width(text: string): int64', 'Returns terminal display-cell width using Opalescent Unicode terminal width policy.'],
    ['terminal_text_clip_to_cells', 'terminal_text_clip_to_cells(text: string, max_cells: int64): string, int64 errors TerminalTextLayoutError, AllocationFailureError', 'Clips text to the largest grapheme-boundary prefix that fits within `max_cells` terminal cells and returns the clipped text and used cell count.'],
    ['string_join', 'string_join(parts: string[], separator: string): string errors AllocationFailureError', 'Joins strings by placing `separator` between each part. Allocation can fail, so callers must use `propagate` or `guard`.'],
    ['string_builder_new', 'string_builder_new(): StringBuilder', 'Creates an empty string builder for incremental string construction.'],
    ['string_builder_push', 'string_builder_push(builder: StringBuilder, text: string): void errors BuilderFinishedError, AllocationFailureError', 'Appends text to a string builder. Fails if the builder is already finished or allocation fails.'],
    ['string_builder_finish', 'string_builder_finish(builder: StringBuilder): string errors BuilderFinishedError, AllocationFailureError', 'Finishes a string builder and returns the accumulated string. Calling push or finish again after finishing fails.']
  ]);
}

/**
 * Builds array helper completions from the standard module.
 * @returns Standard array helper completion definitions.
 */
function arrayHelperCompletions(): ExternalCompletionDefinition[] {
  return externalDefinitions('standard', [
    ['array_length', 'array_length<T>(array: T[]): int64', 'Returns the number of elements in an array. Source code usually uses the `.length` member syntax.'],
    ['array_filled', 'array_filled<T>(count: int64, value: T): T[]', 'Creates an array containing `count` copies of `value`.'],
    ['reserve', 'reserve<T>(array: T[], capacity: int64): T[]', 'Returns an array with storage reserved for at least `capacity` elements.'],
    ['clear', 'clear<T>(array: T[]): T[]', 'Returns an empty array of the same element type.']
  ]);
}

/**
 * Builds bytes helper completions from the standard module.
 * @returns Standard bytes helper completion definitions.
 */
function bytesHelperCompletions(): ExternalCompletionDefinition[] {
  return externalDefinitions('standard', [
    ['bytes_new', 'bytes_new(): Bytes', 'Returns an empty `Bytes` value. New code can also use `new Bytes`.'],
    ['bytes_length', 'bytes_length(buffer: Bytes): int32', 'Returns the number of bytes in an immutable byte buffer.'],
    ['bytes_to_hex', 'bytes_to_hex(buffer: Bytes): string', 'Encodes a byte buffer as lowercase hexadecimal text.'],
    ['bytes_from_hex', 'bytes_from_hex(text: string): Bytes errors HexDecodeError', 'Decodes hexadecimal text into bytes. Accepts uppercase and lowercase digits; odd length or non-hex characters fail with `HexDecodeError`.'],
    ['bytes_concatenate', 'bytes_concatenate(left: Bytes, right: Bytes): Bytes', 'Returns a new byte buffer containing `left` followed by `right`.'],
    ['bytes_slice', 'bytes_slice(source: Bytes, start: int32, end: int32): Bytes errors SliceRangeError', 'Returns bytes in the half-open range `[start, end)`. Inverted or out-of-bounds ranges fail.']
  ]);
}

/**
 * Builds filesystem helper completions from the standard module.
 * @returns Standard filesystem helper completion definitions.
 */
function filesystemHelperCompletions(): ExternalCompletionDefinition[] {
  return externalDefinitions('standard', [
    ['path_from', 'path_from(raw: string): FilesystemPath', 'Wraps a raw string as a filesystem path.'],
    ['join_path_components', 'join_path_components(base: FilesystemPath, components: string[]): FilesystemPath', 'Joins a base path with one or more child components and normalizes separators.'],
    ['path_parent_directory', 'path_parent_directory(path: FilesystemPath): FilesystemPath', 'Returns the parent directory of a filesystem path.'],
    ['path_file_name', 'path_file_name(path: FilesystemPath): string', 'Returns the final path component, or an empty string when no file name exists.'],
    ['path_file_extension', 'path_file_extension(path: FilesystemPath): string', 'Returns the text after the final dot in the file name, or an empty string if no extension exists.'],
    ['normalize_path', 'normalize_path(path: FilesystemPath): FilesystemPath', 'Normalizes path syntax by removing redundant separators and resolving simple `.` and `..` segments where possible.'],
    ['path_to_string', 'path_to_string(path: FilesystemPath): string', 'Returns the string representation of a filesystem path.'],
    ['absolute_path_sync', 'absolute_path_sync(path: FilesystemPath): FilesystemPath errors InvalidPathError, PermissionDeniedError', 'Resolves a path to an absolute path using the host filesystem. Invalid or inaccessible paths fail.'],
    ['read_contents_sync', 'read_contents_sync(path: FilesystemPath): Bytes errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError', 'Reads the whole file as raw bytes.'],
    ['read_text_sync', 'read_text_sync(path: FilesystemPath): string errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error', 'Reads the whole file as UTF-8 text. Fails with `InvalidUtf8Error` when the bytes are not valid UTF-8.'],
    ['read_first_line_sync', 'read_first_line_sync(path: FilesystemPath): string errors FileNotFoundError, PermissionDeniedError, IsADirectoryError, InvalidUtf8Error, OffsetOutOfRangeError, ReadFailureError', 'Reads and returns the first line of a text file.'],
    ['read_lines_sync', 'read_lines_sync(path: FilesystemPath): string[] errors FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error', 'Reads a UTF-8 text file and returns its lines as a string array.'],
    ['read_bytes_at_offset_sync', 'read_bytes_at_offset_sync(path: FilesystemPath, offset: int64, count: int64): Bytes errors FileNotFoundError, PermissionDeniedError, ReadFailureError, OffsetOutOfRangeError, InvalidPathError', 'Reads `count` bytes beginning at `offset`. Fails if the range is outside the file.'],
    ['write_contents_sync', 'write_contents_sync(path: FilesystemPath, content: Bytes): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError', 'Overwrites a file with raw bytes, creating it when the platform permits.'],
    ['write_text_sync', 'write_text_sync(path: FilesystemPath, text: string): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError', 'Overwrites a file with UTF-8 text, creating it when the platform permits.'],
    ['write_contents_atomic_sync', 'write_contents_atomic_sync(path: FilesystemPath, content: Bytes): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError', 'Writes bytes through a temporary file and replaces the target to reduce partial-output risk.'],
    ['write_text_atomic_sync', 'write_text_atomic_sync(path: FilesystemPath, text: string): void errors PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError', 'Writes text through a temporary file and replaces the target to reduce partial-output risk.'],
    ['append_contents_sync', 'append_contents_sync(path: FilesystemPath, content: Bytes): void errors FileNotFoundError, PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError', 'Appends raw bytes to an existing file.'],
    ['append_text_sync', 'append_text_sync(path: FilesystemPath, text: string): void errors FileNotFoundError, PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError', 'Appends UTF-8 text to an existing file.'],
    ['write_bytes_at_offset_sync', 'write_bytes_at_offset_sync(path: FilesystemPath, offset: int64, content: Bytes): void errors FileNotFoundError, PermissionDeniedError, WriteFailureError, OffsetOutOfRangeError, InvalidPathError, FilesystemFullError', 'Writes bytes at a specific file offset. Fails when the offset is invalid or outside the allowed range.'],
    ['create_file_sync', 'create_file_sync(path: FilesystemPath): void errors FileAlreadyExistsError, PermissionDeniedError, CreateFailureError, InvalidPathError, FilesystemFullError', 'Creates a new empty file and fails if it already exists.'],
    ['delete_file_sync', 'delete_file_sync(path: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, DeleteFailureError, IsADirectoryError, InvalidPathError', 'Deletes a file. Fails if the path is missing or points to a directory.'],
    ['copy_file_sync', 'copy_file_sync(source: FilesystemPath, destination: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, CopyFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError', 'Copies one file to another path.'],
    ['move_path_sync', 'move_path_sync(source: FilesystemPath, destination: FilesystemPath): void errors FileNotFoundError, PermissionDeniedError, MoveFailureError, FileAlreadyExistsError, InvalidPathError', 'Moves or renames a path.'],
    ['path_exists_sync', 'path_exists_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError', 'Returns whether a path exists while still surfacing permission and invalid-path failures.'],
    ['read_metadata_sync', 'read_metadata_sync(path: FilesystemPath): FileMetadata errors FileNotFoundError, PermissionDeniedError, MetadataUnavailableError, InvalidPathError', 'Reads metadata such as size, directory status, symlink status, and modification time.'],
    ['read_metadata_nofollow_sync', 'read_metadata_nofollow_sync(path: FilesystemPath): FileMetadata errors FileNotFoundError, PermissionDeniedError, MetadataUnavailableError, InvalidPathError', 'Reads metadata without following symlinks.'],
    ['create_directory_sync', 'create_directory_sync(path: FilesystemPath): void errors FileAlreadyExistsError, PermissionDeniedError, CreateFailureError, InvalidPathError, FilesystemFullError', 'Creates one directory and fails if it already exists.'],
    ['create_directory_recursive_sync', 'create_directory_recursive_sync(path: FilesystemPath): void errors PermissionDeniedError, CreateFailureError, InvalidPathError, FilesystemFullError', 'Creates a directory and any missing parents.'],
    ['delete_directory_sync', 'delete_directory_sync(path: FilesystemPath): void errors DirectoryNotFoundError, PermissionDeniedError, DeleteFailureError, DirectoryNotEmptyError, IsNotADirectoryError, InvalidPathError', 'Deletes an empty directory.'],
    ['delete_directory_recursive_sync', 'delete_directory_recursive_sync(path: FilesystemPath): void errors DirectoryNotFoundError, PermissionDeniedError, DeleteFailureError, IsNotADirectoryError, InvalidPathError', 'Deletes a directory tree recursively.'],
    ['list_directory_sync', 'list_directory_sync(path: FilesystemPath): FilesystemPath[] errors DirectoryNotFoundError, PermissionDeniedError, ReadFailureError, IsNotADirectoryError, InvalidPathError', 'Returns entries inside a directory as filesystem paths.'],
    ['is_file_sync', 'is_file_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError', 'Returns true if the path is a file, following symlinks.'],
    ['is_file_nofollow_sync', 'is_file_nofollow_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError', 'Returns true if the path itself is a file without following symlinks.'],
    ['is_directory_sync', 'is_directory_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError', 'Returns true if the path is a directory, following symlinks.'],
    ['is_directory_nofollow_sync', 'is_directory_nofollow_sync(path: FilesystemPath): boolean errors PermissionDeniedError, InvalidPathError', 'Returns true if the path itself is a directory without following symlinks.']
  ]);
}

/**
 * Builds stdout and terminal helper completions from the standard module.
 * @returns Standard output and terminal completion definitions.
 */
function stdoutAndTerminalCompletions(): ExternalCompletionDefinition[] {
  return externalDefinitions('standard', [
    ['print_text_sync', 'print_text_sync(text: string): void errors WriteFailureError, SinkClosedError', 'Writes text to standard output without adding a newline.'],
    ['flush_standard_output_sync', 'flush_standard_output_sync(): void errors FlushFailureError, SinkClosedError', 'Flushes standard output.'],
    ['stdout_writer', 'stdout_writer(): StdoutWriter errors StandardOutputHandleError', 'Returns a writer handle for standard output.'],
    ['writer_write_sync', 'writer_write_sync(writer: StdoutWriter, text: string): void errors WriteFailureError, SinkClosedError', 'Writes text through a writer handle.'],
    ['writer_flush_sync', 'writer_flush_sync(writer: StdoutWriter): void errors FlushFailureError, SinkClosedError', 'Flushes a writer handle.'],
    ['stdout_terminal', 'stdout_terminal(): StdoutTerminal errors StandardOutputHandleError', 'Returns a terminal handle for standard output.'],
    ['terminal_supports_ansi', 'terminal_supports_ansi(terminal: StdoutTerminal): boolean errors StandardOutputCapabilityError', 'Returns whether the terminal supports ANSI control sequences.'],
    ['terminal_clear_screen_on_sync', 'terminal_clear_screen_on_sync(terminal: StdoutTerminal): void errors TerminalWriteFailureError, SinkClosedError', 'Clears the screen for a terminal handle.'],
    ['terminal_move_cursor_on_sync', 'terminal_move_cursor_on_sync(terminal: StdoutTerminal, row: int32, column: int32): void errors TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError', 'Moves the cursor for a terminal handle. Invalid row or column values fail.'],
    ['terminal_draw_rows_sync', 'terminal_draw_rows_sync(terminal: StdoutTerminal, rows: string[]): void errors TerminalWriteFailureError, SinkClosedError', 'Draws multiple rows to the terminal.'],
    ['terminal_clear_screen_sync', 'terminal_clear_screen_sync(): void errors TerminalWriteFailureError, SinkClosedError', 'Convenience helper that clears standard output\'s terminal.'],
    ['terminal_move_cursor_sync', 'terminal_move_cursor_sync(row: int32, column: int32): void errors TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError', 'Convenience helper that moves the cursor on standard output\'s terminal.']
  ]);
}

/**
 * Builds time helper completions from the standard module.
 * @returns Standard time completion definitions.
 */
function timeCompletions(): ExternalCompletionDefinition[] {
  return externalDefinitions('standard', [
    ['sleep_ms_sync', 'sleep_ms_sync(milliseconds: int32): void errors InvalidDurationError', 'Blocks the current thread for the requested number of milliseconds. Negative or invalid durations fail.'],
    ['frame_clock_new', 'frame_clock_new(frames_per_second: int32): FrameClock errors InvalidFrameRateError', 'Creates a frame clock for fixed-rate loops. Invalid frame rates fail.'],
    ['frame_clock_wait_next_sync', 'frame_clock_wait_next_sync(clock: FrameClock): void errors InvalidFrameRateError', 'Waits until the next frame deadline for the frame clock and updates the next deadline.']
  ]);
}

/**
 * Builds random-number helper completions from the math module.
 * @returns Math module completion definitions.
 */
function mathCompletions(): ExternalCompletionDefinition[] {
  return ['int8', 'int16', 'int32', 'int64', 'uint8', 'uint16', 'uint32', 'uint64'].map((typeName) => ({
    detail: `math: random_${typeName}(min: ${typeName}, max: ${typeName}): ${typeName}`,
    documentation: `Returns a pseudo-random ${typeName} value in the inclusive range from \`min\` to \`max\`.`,
    moduleName: 'math',
    name: `random_${typeName}`
  }));
}

/**
 * Builds process helper completions from the process module.
 * @returns Process module completion definitions.
 */
function processCompletions(): ExternalCompletionDefinition[] {
  return externalDefinitions('process', [
    ['current_working_directory_sync', 'current_working_directory_sync(): FilesystemPath', 'Returns the absolute path of the current working directory. Errors include permission, invalid path, and unavailable current directory failures.'],
    ['current_executable_path_sync', 'current_executable_path_sync(): FilesystemPath', 'Returns the absolute path of the currently running executable. Errors include permission, invalid path, and unavailable executable path failures.'],
    ['current_executable_directory_sync', 'current_executable_directory_sync(): FilesystemPath', 'Returns the directory containing the currently running executable.'],
    ['set_current_working_directory_sync', 'set_current_working_directory_sync(path: FilesystemPath): void', 'Changes the current working directory. Errors include missing path, permission, not-a-directory, and invalid path failures.'],
    ['get_environment_variable', 'get_environment_variable(name: string): string', 'Returns the value of an environment variable. Errors include missing variable, invalid name, and invalid UTF-8.'],
    ['get_environment_variable_or', 'get_environment_variable_or(name: string, default_value: string): string', 'Returns an environment variable value, or `default_value` when the variable is absent. Invalid names and invalid UTF-8 still fail.'],
    ['environment_variable_exists', 'environment_variable_exists(name: string): boolean', 'Returns whether an environment variable exists. Invalid names fail.'],
    ['exit_process', 'exit_process(code: int32): void', 'Immediately terminates the current process with the given exit code. Treat it as if it never returns even though it currently has type `void`.']
  ]);
}

/**
 * Builds named error-set completions from the standard.errors module.
 * @returns Type-only standard error-set completion definitions.
 */
function standardErrorSetCompletions(): ExternalCompletionDefinition[] {
  return externalDefinitions(
    'standard.errors',
    [
      ['ParseErrors', 'ParseErrors = ParseError', 'Covers parse failures from numeric string parsing.'],
      ['BytesErrors', 'BytesErrors = HexDecodeError, SliceRangeError', 'Covers byte hex decoding and byte slicing failures.'],
      ['StringSearchErrors', 'StringSearchErrors = StringEmptySearchTextError, StringPatternNotFoundError', 'Covers string search failures.'],
      ['StringRangeErrors', 'StringRangeErrors = StringNegativeCountError, StringRangeOutOfBoundsError, StringRangeOrderError', 'Covers string range, count, and ordering failures.'],
      ['StringBuilderErrors', 'StringBuilderErrors = BuilderFinishedError, AllocationFailureError', 'Covers string-builder use-after-finish and allocation failures.'],
      ['StdoutWriterErrors', 'StdoutWriterErrors = WriteFailureError, FlushFailureError, SinkClosedError', 'Covers standard-output writer write, flush, and closed-sink failures.'],
      ['TerminalControlErrors', 'TerminalControlErrors = TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError', 'Covers terminal control write failures, invalid cursor positions, and closed sinks.'],
      ['TimeErrors', 'TimeErrors = InvalidDurationError, InvalidFrameRateError', 'Covers invalid sleep durations and frame-clock rates.'],
      ['ProcessPathErrors', 'ProcessPathErrors = PermissionDeniedError, InvalidPathError, CurrentWorkingDirectoryUnavailableError, CurrentExecutablePathUnavailableError, FileNotFoundError, IsNotADirectoryError', 'Covers process current-directory and executable-path failures.'],
      ['ProcessEnvErrors', 'ProcessEnvErrors = EnvironmentVariableNotFoundError, InvalidEnvironmentVariableNameError, InvalidUtf8Error', 'Covers process environment variable lookup failures.'],
      ['FilesystemPathErrors', 'FilesystemPathErrors = InvalidPathError, PermissionDeniedError', 'Covers filesystem path validation and permission failures.'],
      ['FilesystemReadErrors', 'FilesystemReadErrors = FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error, OffsetOutOfRangeError', 'Covers filesystem read failures including FileNotFoundError, PermissionDeniedError, ReadFailureError, IsADirectoryError, InvalidPathError, InvalidUtf8Error, and OffsetOutOfRangeError.'],
      ['FilesystemWriteErrors', 'FilesystemWriteErrors = FileNotFoundError, PermissionDeniedError, WriteFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError, OffsetOutOfRangeError', 'Covers filesystem write and append failures.'],
      ['FilesystemCreateErrors', 'FilesystemCreateErrors = FileAlreadyExistsError, PermissionDeniedError, CreateFailureError, InvalidPathError, FilesystemFullError', 'Covers file and directory creation failures.'],
      ['FilesystemDeleteErrors', 'FilesystemDeleteErrors = FileNotFoundError, PermissionDeniedError, DeleteFailureError, IsADirectoryError, InvalidPathError', 'Covers file deletion failures.'],
      ['FilesystemDirectoryDeleteErrors', 'FilesystemDirectoryDeleteErrors = DirectoryNotFoundError, PermissionDeniedError, DeleteFailureError, DirectoryNotEmptyError, IsNotADirectoryError, InvalidPathError', 'Covers directory deletion failures.'],
      ['FilesystemCopyMoveErrors', 'FilesystemCopyMoveErrors = FileNotFoundError, PermissionDeniedError, CopyFailureError, MoveFailureError, IsADirectoryError, FileAlreadyExistsError, InvalidPathError, FilesystemFullError', 'Covers filesystem copy and move failures.'],
      ['FilesystemMetadataErrors', 'FilesystemMetadataErrors = FileNotFoundError, PermissionDeniedError, MetadataUnavailableError, InvalidPathError', 'Covers filesystem metadata failures.'],
      ['FilesystemListErrors', 'FilesystemListErrors = DirectoryNotFoundError, PermissionDeniedError, ReadFailureError, IsNotADirectoryError, InvalidPathError', 'Covers directory listing failures.'],
      ['FilesystemErrors', 'FilesystemErrors = union of filesystem error leaves', 'Covers the broad filesystem error set, including read, write, create, delete, copy/move, metadata, and list-directory leaves.'],
      ['IndexAccessErrors', 'IndexAccessErrors = IndexOutOfBoundsError', 'Covers array and string `.at(index)` out-of-bounds access failures.']
    ],
    true
  );
}

/**
 * Converts compact external completion tuples into full definitions.
 * @param moduleName Module imported by the completion definitions.
 * @param definitions Name, signature, and documentation tuples.
 * @param typeOnly Whether these symbols require `import type`.
 * @returns External completion definitions.
 */
function externalDefinitions(moduleName: string, definitions: Array<[string, string, string]>, typeOnly = false): ExternalCompletionDefinition[] {
  return definitions.map(([name, signature, documentation]) => ({
    detail: `${moduleName}: ${signature}`,
    documentation,
    moduleName,
    name,
    typeOnly
  }));
}

/**
 * Builds completion candidates from project symbols, including auto-import edits for public cross-file declarations.
 * @param request Current file source, path, and project symbol index.
 * @returns Completion candidates suitable for conversion into VS Code completion items.
 */
export function completionItemsForSymbols(request: OpalescentCompletionRequest): OpalescentCompletionItem[] {
  const items: OpalescentCompletionItem[] = staticCompletionItemsForRequest(request);
  const seen = new Set<string>();

  for (const symbol of request.symbols) {
    if (!isCompletionSymbolForRequest(symbol, request.currentFilePath)) {
      continue;
    }

    const key = completionSymbolKey(symbol);
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);

    const autoImportEdit = autoImportEditForSymbol(request.source, request.currentFilePath, symbol);
    const sourceModule = sameFilePath(symbol.filePath, request.currentFilePath)
      ? undefined
      : moduleSpecifierForLocalImport(request.currentFilePath, symbol.filePath);
    items.push({
      autoImportEdit,
      documentation: symbol.documentation,
      insertText: symbol.name,
      kind: symbol.kind,
      name: symbol.name,
      sourceModule,
      symbol
    });
  }

  return items.sort(compareCompletionItems);
}

/**
 * Builds built-in keyword, snippet, and standard-library completions.
 * @param request Current completion request.
 * @returns Static completion items.
 */
function staticCompletionItemsForRequest(request: OpalescentCompletionRequest): OpalescentCompletionItem[] {
  return [
    ...keywordNames.map(keywordCompletion),
    ...snippetCompletions,
    ...externalCompletions.map((completion) => externalCompletionForRequest(completion, request.source))
  ];
}

/**
 * Converts a keyword name into a documented completion item.
 * @param name Keyword text.
 * @returns Keyword completion item.
 */
function keywordCompletion(name: (typeof keywordNames)[number]): OpalescentCompletionItem {
  return {
    detail: primitiveTypeDocumentation.has(name) ? 'Primitive type' : 'Keyword',
    documentation: primitiveTypeDocumentation.get(name) ?? keywordDocumentation.get(name),
    insertText: name,
    kind: 'keyword',
    name
  };
}

/**
 * Converts an external importable standard-library symbol into a completion item for the current file.
 * @param completion Static external completion definition.
 * @param source Current file source text.
 * @returns Completion item with an auto-import edit when needed.
 */
function externalCompletionForRequest(completion: ExternalCompletionDefinition, source: string): OpalescentCompletionItem {
  const importKind = completion.typeOnly ? 'import type' : 'import';
  const autoImportEdit = sourceAlreadyImportsSymbol(source, completion.name, completion.moduleName)
    ? undefined
    : insertImportLineEdit(source, `${importKind} ${completion.name} from ${completion.moduleName}`);
  return {
    autoImportEdit,
    detail: completion.detail,
    documentation: completion.documentation,
    insertText: completion.name,
    kind: completion.typeOnly ? 'type' : 'function',
    name: completion.name,
    sourceModule: completion.moduleName
  };
}

/**
 * Builds the import line required for a project symbol.
 * @param symbol Symbol to import.
 * @param currentFilePath File that will receive the import.
 * @returns Import line text, or undefined when the symbol is not importable.
 */
export function importLineForSymbol(symbol: OpalescentSymbol, currentFilePath: string): string | undefined {
  if (!isAutoImportableSymbol(symbol)) {
    return undefined;
  }

  const importKind = typeOnlyImportKinds.has(symbol.kind) ? 'import type' : 'import';
  const sourceModule = moduleSpecifierForLocalImport(currentFilePath, symbol.filePath);
  return `${importKind} ${symbol.name} from ${sourceModule}`;
}

/**
 * Builds an auto-import insertion edit for a symbol when one is needed.
 * @param source Current file source text.
 * @param currentFilePath File that will receive the import.
 * @param symbol Completion symbol.
 * @returns Import insertion edit, or undefined when no import should be added.
 */
export function autoImportEditForSymbol(
  source: string,
  currentFilePath: string,
  symbol: OpalescentSymbol
): OpalescentCompletionImportEdit | undefined {
  if (sameFilePath(symbol.filePath, currentFilePath) || !isAutoImportableSymbol(symbol)) {
    return undefined;
  }

  const importLine = importLineForSymbol(symbol, currentFilePath);
  if (!importLine) {
    return undefined;
  }

  const sourceModule = moduleSpecifierForLocalImport(currentFilePath, symbol.filePath);
  if (sourceAlreadyImportsSymbol(source, symbol.name, sourceModule)) {
    return undefined;
  }

  return insertImportLineEdit(source, importLine);
}

/**
 * Checks whether a symbol should be exposed in completion for this file.
 * @param symbol Candidate symbol.
 * @param currentFilePath File requesting completions.
 * @returns Whether the symbol should appear in completion.
 */
function isCompletionSymbolForRequest(symbol: OpalescentSymbol, currentFilePath: string): boolean {
  if (sameFilePath(symbol.filePath, currentFilePath)) {
    return sameFileCompletionKinds.has(symbol.kind);
  }

  return isAutoImportableSymbol(symbol);
}

/**
 * Checks whether a cross-file symbol can be imported automatically.
 * @param symbol Candidate symbol.
 * @returns Whether the symbol is a public file-scoped importable declaration.
 */
function isAutoImportableSymbol(symbol: OpalescentSymbol): boolean {
  return symbol.exported && symbol.scopeKind === 'file' && autoImportCompletionKinds.has(symbol.kind);
}

/**
 * Builds a stable de-duplication key for a symbol-backed completion.
 * @param symbol Symbol to key.
 * @returns Unique key for one declaration site.
 */
function completionSymbolKey(symbol: OpalescentSymbol): string {
  return [symbol.filePath, symbol.line, symbol.character, symbol.kind, symbol.name].join(':');
}

/**
 * Sorts local completions before auto-import completions, then by display name.
 * @param left Left completion.
 * @param right Right completion.
 * @returns Sort ordering.
 */
function compareCompletionItems(left: OpalescentCompletionItem, right: OpalescentCompletionItem): number {
  const leftImportPriority = left.autoImportEdit ? 1 : 0;
  const rightImportPriority = right.autoImportEdit ? 1 : 0;
  if (leftImportPriority !== rightImportPriority) {
    return leftImportPriority - rightImportPriority;
  }

  const nameCompare = left.name.localeCompare(right.name);
  if (nameCompare !== 0) {
    return nameCompare;
  }

  return left.sourceModule?.localeCompare(right.sourceModule ?? '') ?? (right.sourceModule ? -1 : 0);
}

/**
 * Finds the import insertion location for a new top-level import line.
 * @param source Current source text.
 * @param importLine Complete import line without trailing newline.
 * @returns Insertion edit that preserves a contiguous top import block.
 */
function insertImportLineEdit(source: string, importLine: string): OpalescentCompletionImportEdit {
  if (source.length === 0) {
    return { character: 0, line: 0, text: `${importLine}\n` };
  }

  const lines = source.split('\n');
  const lastTopImportLine = lastContiguousTopImportLine(lines);
  if (lastTopImportLine >= 0) {
    return { character: 0, line: lastTopImportLine + 1, text: `${importLine}\n` };
  }

  return { character: 0, line: 0, text: `${importLine}\n\n` };
}

/**
 * Finds the last import line in the initial import block.
 * @param lines Source lines.
 * @returns Last import line index, or -1 when no top import block exists.
 */
function lastContiguousTopImportLine(lines: string[]): number {
  let lastImportLine = -1;
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex] ?? '';
    const trimmed = line.trim();
    if (isImportLine(line)) {
      lastImportLine = lineIndex;
      continue;
    }
    if (trimmed === '' && lastImportLine >= 0) {
      break;
    }
    if (trimmed === '') {
      continue;
    }
    break;
  }
  return lastImportLine;
}

/**
 * Checks whether a source file already imports a symbol from a module.
 * @param source Current source text.
 * @param name Symbol name.
 * @param sourceModule Module specifier expected for the import.
 * @returns Whether the import already exists.
 */
function sourceAlreadyImportsSymbol(source: string, name: string, sourceModule: string): boolean {
  for (const line of source.split('\n')) {
    const parsed = parseImportLine(line);
    if (!parsed || parsed.sourceModule !== sourceModule) {
      continue;
    }
    if (parsed.names.includes(name)) {
      return true;
    }
  }
  return false;
}

/**
 * Parses an Opalescent named import line.
 * @param line Source line.
 * @returns Imported names and module specifier, or undefined for non-import lines.
 */
function parseImportLine(line: string): { names: string[]; sourceModule: string } | undefined {
  const match = /^\s*import\s+(?:type\s+)?(.+?)\s+from\s+(?:'([^']+)'|"([^"]+)"|([^\s#]+))/.exec(line);
  const namesText = match?.[1];
  const sourceModule = match?.[2] ?? match?.[3] ?? match?.[4];
  if (!namesText || !sourceModule) {
    return undefined;
  }

  const names = namesText
    .split(',')
    .map((name) => name.trim().split(/\s+as\s+/)[0]?.trim() ?? '')
    .filter((name) => name.length > 0);
  return { names, sourceModule };
}

/**
 * Checks whether a line is an Opalescent named import.
 * @param line Source line.
 * @returns Whether the line is an import declaration.
 */
function isImportLine(line: string): boolean {
  return /^\s*import\b/.test(line);
}

/**
 * Checks whether two paths identify the same file after separator normalization.
 * @param left Left file path.
 * @param right Right file path.
 * @returns Whether the paths match.
 */
function sameFilePath(left: string, right: string): boolean {
  return left.replace(/\\/g, '/') === right.replace(/\\/g, '/');
}
