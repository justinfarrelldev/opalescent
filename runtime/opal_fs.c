/*
 * opal_fs.c - Runtime implementation of the path-object-centric file-I/O stdlib.
 *
 * Ownership contracts
 * -------------------
 * - Caller owns all returned heap strings (char*) and must free them.
 * - Caller owns returned OpalBytes* values and must free them via bytes_free.
 * - Error strings in result structs are static string literals — do NOT free them.
 * - FilesystemPath values are heap-allocated char* (the raw path string).
 * - FsPathArrayResult / FsStringArrayResult: caller frees each element and the array.
 *
 * Error model
 * -----------
 * All fallible functions return an Fs*Result struct where a non-NULL `error`
 * field indicates failure and the `value` field is undefined.  This mirrors
 * the ParseResult* / BytesResult convention so guard/propagate lowering is
 * identical across all stdlib surfaces.
 *
 * Function bodies are populated in T5–T10 (infrastructure batches).
 * This file is a skeleton that satisfies the linker during T3.
 */

#include "opal_portability.h"
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
