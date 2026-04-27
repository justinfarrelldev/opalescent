/*
 * opal_fs.c - Runtime implementation of the path-object-centric file-I/O stdlib.
 *
 * Ownership contracts
 * -------------------
 * - Caller owns all returned heap strings (char*) and must free them.
 * - Caller owns returned OpalBytes* values and must free them via bytes_free.
 * - Legacy stubs still return static-literal errors; newer fs impls return heap
 *   error strings (caller-owned) per the fs error allocation contract.
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
#include "opal_fs_errors.h"
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#ifndef OPAL_BYTES_TYPE_DEFINED
typedef struct OpalBytes {
    size_t length;
    uint8_t* data;
} OpalBytes;
#define OPAL_BYTES_TYPE_DEFINED 1
#endif

typedef struct { void*      value; const char* error; } FsVoidResult;
typedef struct { OpalBytes* value; const char* error; } FsBytesResult;
typedef struct { char*      value; const char* error; } FsStringResult;
typedef struct { int8_t     value; const char* error; } FsBooleanResult;
typedef struct { int32_t    value; const char* error; } FsInt32Result;
typedef struct { int64_t    value; const char* error; } FsInt64Result;
typedef struct { char*      value; const char* error; } FsPathResult;
typedef struct { char**     value; int64_t count; const char* error; } FsPathArrayResult;
typedef struct { char**     value; int64_t count; const char* error; } FsStringArrayResult;
typedef struct {
    int64_t size_bytes;
    int8_t is_directory;
    int8_t is_symlink;
    int64_t modified_unix_seconds;
} OpalFileMetadata;

typedef struct { void*      value; const char* error; } FsMetadataResult;
typedef struct { void*      value; const char* error; } FsPermissionsResult;

#if OPAL_HAS_DIRENT
#  include <dirent.h>
#endif

/* ── Windows directory listing shim (forward declarations) ──────────────── */

#if !OPAL_HAS_DIRENT

/*
 * Forward declarations for Windows directory listing shim APIs.
 * These are used to provide POSIX-like directory iteration on Windows.
 * Full implementations are provided in Task 16.
 */

typedef struct opal_dir_s opal_dir_t;
typedef struct {
    char d_name[260];  /* MAX_PATH on Windows */
} opal_dirent_t;

opal_dir_t* opal_opendir(const char* path);
opal_dirent_t* opal_readdir(opal_dir_t* dir);
int opal_closedir(opal_dir_t* dir);

#endif /* !OPAL_HAS_DIRENT */

static char* safe_strdup(const char* value) {
    return strdup(value ? value : "");
}

static char* errno_to_fs_error(int err, const char* op_prefix);
static char* fwrite_all(FILE* f, const uint8_t* buf, size_t len);

static int opal_is_path_separator(char c) {
    char sep = opal_path_separator();
    return c == '/' || c == sep;
}

static void free_path_segments(char** segments, int64_t count) {
    if (!segments) return;
    for (int64_t i = 0; i < count; i++) {
        free(segments[i]);
    }
}

static void free_string_array_elements(char** values, size_t count) {
    if (!values) return;
    for (size_t i = 0; i < count; i++) {
        free(values[i]);
    }
}

static int compare_string_pointers(const void* lhs, const void* rhs) {
    const char* const* left = (const char* const*)lhs;
    const char* const* right = (const char* const*)rhs;
    return strcmp(*left, *right);
}

static char* lex_normalize_path(const char* path) {
    if (!path || path[0] == '\0') {
        return safe_strdup("");
    }

    size_t input_len = strlen(path);
    int is_absolute = path[0] == '/';
    int had_trailing_separator = input_len > 0 && opal_is_path_separator(path[input_len - 1]);

    char** segments = (char**)calloc(input_len + 1, sizeof(char*));
    if (!segments) {
        return safe_strdup("");
    }

    int64_t segment_count = 0;
    int escaped_root = 0;
    size_t i = 0;

    while (i < input_len) {
        while (i < input_len && opal_is_path_separator(path[i])) {
            i++;
        }
        if (i >= input_len) {
            break;
        }

        size_t start = i;
        while (i < input_len && !opal_is_path_separator(path[i])) {
            i++;
        }
        size_t length = i - start;

        if (length == 1 && path[start] == '.') {
            continue;
        }

        if (length == 2 && path[start] == '.' && path[start + 1] == '.') {
            if (segment_count > 0 && strcmp(segments[segment_count - 1], "..") != 0) {
                free(segments[segment_count - 1]);
                segment_count--;
                continue;
            }
            if (is_absolute) {
                escaped_root = 1;
                break;
            }
            segments[segment_count++] = safe_strdup("..");
            if (!segments[segment_count - 1]) {
                free_path_segments(segments, segment_count - 1);
                free(segments);
                return safe_strdup("");
            }
            continue;
        }

        char* segment = (char*)malloc(length + 1);
        if (!segment) {
            free_path_segments(segments, segment_count);
            free(segments);
            return safe_strdup("");
        }
        memcpy(segment, path + start, length);
        segment[length] = '\0';
        segments[segment_count++] = segment;
    }

    if (escaped_root) {
        free_path_segments(segments, segment_count);
        free(segments);
        return safe_strdup("");
    }

    if (segment_count == 0) {
        free(segments);
        if (is_absolute) {
            return safe_strdup("/");
        }
        return safe_strdup("");
    }

    char sep = opal_path_separator();
    size_t output_len = is_absolute ? 1 : 0;
    for (int64_t index = 0; index < segment_count; index++) {
        output_len += strlen(segments[index]);
        if (index > 0) {
            output_len += 1;
        }
    }

    if (had_trailing_separator) {
        output_len += 1;
    }

    char* output = (char*)malloc(output_len + 1);
    if (!output) {
        free_path_segments(segments, segment_count);
        free(segments);
        return safe_strdup("");
    }

    size_t position = 0;
    if (is_absolute) {
        output[position++] = sep;
    }

    for (int64_t index = 0; index < segment_count; index++) {
        if (index > 0) {
            output[position++] = sep;
        }
        size_t length = strlen(segments[index]);
        memcpy(output + position, segments[index], length);
        position += length;
    }

    if (had_trailing_separator) {
        output[position++] = sep;
    }
    output[position] = '\0';

    free_path_segments(segments, segment_count);
    free(segments);
    return output;
}

char* path_from(const char* raw) {
    if (!raw || raw[0] == '\0') return safe_strdup("");
    return safe_strdup(raw);
}

char* join_path_components(const char* base, const char** components, int64_t count) {
    const char* seed = base ? base : "";
    if (count <= 0) {
        return lex_normalize_path(seed);
    }
    if (!components) {
        return safe_strdup(seed);
    }

    char* accumulator = safe_strdup(seed);
    if (!accumulator) {
        return safe_strdup("");
    }

    char sep = opal_path_separator();
    for (int64_t i = 0; i < count; i++) {
        const char* component = components[i];
        if (!component || component[0] == '\0') {
            continue;
        }

        if (component[0] == '/') {
            free(accumulator);
            accumulator = safe_strdup(component);
            if (!accumulator) {
                return safe_strdup("");
            }
            continue;
        }

        size_t base_len = strlen(accumulator);
        size_t component_start = 0;
        while (component[component_start] && opal_is_path_separator(component[component_start])) {
            component_start++;
        }

        size_t component_len = strlen(component + component_start);
        int need_separator = base_len > 0 && !opal_is_path_separator(accumulator[base_len - 1]) && component_len > 0;

        size_t next_len = base_len + (need_separator ? 1 : 0) + component_len;
        char* next = (char*)malloc(next_len + 1);
        if (!next) {
            free(accumulator);
            return safe_strdup("");
        }

        memcpy(next, accumulator, base_len);
        size_t position = base_len;
        if (need_separator) {
            next[position++] = sep;
        }
        if (component_len > 0) {
            memcpy(next + position, component + component_start, component_len);
            position += component_len;
        }
        next[position] = '\0';

        free(accumulator);
        accumulator = next;
    }

    char* normalized = lex_normalize_path(accumulator);
    free(accumulator);
    return normalized;
}

char* path_parent_directory(const char* path) {
    if (!path || path[0] == '\0') return strdup(".");
    char* copy = strdup(path);
    char* last = strrchr(copy, '/');
    if (!last) { free(copy); return strdup("."); }
    if (last == copy) { free(copy); return strdup("/"); }
    *last = '\0';
    char* result = strdup(copy);
    free(copy);
    return result;
}

char* path_file_name(const char* path) {
    if (!path || path[0] == '\0') return strdup("");
    const char* last = strrchr(path, '/');
    return strdup(last ? last + 1 : path);
}

char* path_file_extension(const char* path) {
    if (!path || path[0] == '\0') return strdup("");
    const char* name = strrchr(path, '/');
    name = name ? name + 1 : path;
    const char* dot = strrchr(name, '.');
    if (!dot || dot == name) return strdup("");
    return strdup(dot + 1);
}

char* normalize_path(const char* path) {
    return lex_normalize_path(path);
}

char* path_to_string(const char* path) {
    return safe_strdup(path);
}

FsPathResult absolute_path_sync(const char* path) {
    FsPathResult r;
    if (!path || path[0] == '\0') {
        r.value = NULL;
        r.error = "InvalidPathError: empty path";
        return r;
    }
    char resolved_buf[OPAL_PATH_MAX];
    char* resolved = opal_realpath(path, resolved_buf, sizeof(resolved_buf)) ? strdup(resolved_buf) : NULL;
    if (!resolved) {
        r.value = NULL;
        r.error = "InvalidPathError: could not resolve path";
        return r;
    }
    r.value = resolved;
    r.error = NULL;
    return r;
}

FsBytesResult read_contents_sync(const char* path) {
    FsBytesResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat(path, &stat_result) != 0) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    if (stat_result.is_directory) {
        r.error = opal_fs_format_err(OPAL_FS_ERR_IS_DIRECTORY, "path is a directory");
        if (!r.error) {
            r.error = safe_strdup("IsADirectoryError: path is a directory");
        }
        return r;
    }

    FILE* file = fopen(path, "rb");
    if (!file) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    size_t capacity = 4096;
    unsigned char* buffer = (unsigned char*)malloc(capacity);
    if (!buffer) {
        fclose(file);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: out of memory");
        }
        return r;
    }

    size_t length = 0;
    for (;;) {
        if (length == capacity) {
            size_t new_capacity = capacity <= (SIZE_MAX / 2) ? capacity * 2 : SIZE_MAX;
            if (new_capacity <= capacity) {
                free(buffer);
                fclose(file);
                r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "file too large");
                if (!r.error) {
                    r.error = safe_strdup("ReadFailureError: file too large");
                }
                return r;
            }

            unsigned char* grown = (unsigned char*)realloc(buffer, new_capacity);
            if (!grown) {
                free(buffer);
                fclose(file);
                r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
                if (!r.error) {
                    r.error = safe_strdup("ReadFailureError: out of memory");
                }
                return r;
            }
            buffer = grown;
            capacity = new_capacity;
        }

        size_t read_now = fread(buffer + length, 1, capacity - length, file);
        length += read_now;

        if (read_now == 0) {
            if (ferror(file)) {
                int read_errno = errno ? errno : EIO;
                free(buffer);
                fclose(file);
                r.error = errno_to_fs_error(read_errno, OPAL_FS_ERR_IO);
                return r;
            }
            break;
        }
    }

    if (fclose(file) != 0) {
        int close_errno = errno ? errno : EIO;
        free(buffer);
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
        return r;
    }

    OpalBytes* bytes = (OpalBytes*)malloc(sizeof(OpalBytes));
    if (!bytes) {
        free(buffer);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: out of memory");
        }
        return r;
    }

    bytes->length = length;
    if (length == 0) {
        free(buffer);
        bytes->data = NULL;
    } else {
        bytes->data = buffer;
    }

    r.value = bytes;
    return r;
}

static char* fwrite_all(FILE* f, const uint8_t* buf, size_t len) {
    const char* write_prefix = "Io";
    if (!f) {
        return opal_fs_format_err(write_prefix, "invalid file handle");
    }
    if (!buf && len > 0) {
        return opal_fs_format_err(write_prefix, "invalid write buffer");
    }

    size_t total_written = 0;
    while (total_written < len) {
        size_t wrote_now = fwrite(buf + total_written, 1, len - total_written, f);
        if (wrote_now == 0) {
            if (ferror(f)) {
                int write_errno = errno ? errno : EIO;
                return errno_to_fs_error(write_errno, write_prefix);
            }

            char detail[96];
            snprintf(detail, sizeof(detail), "short write (%zu/%zu)", total_written, len);
            return opal_fs_format_err(write_prefix, detail);
        }

        total_written += wrote_now;
    }

    return NULL;
}

static char* errno_to_fs_error(int err, const char* op_prefix) {
    const char* prefix = OPAL_FS_ERR_IO;

    switch (err) {
        case ENOENT:
            prefix = OPAL_FS_ERR_NOT_FOUND;
            break;
        case EACCES:
        case EPERM:
            prefix = OPAL_FS_ERR_PERMISSION_DENIED;
            break;
        case EISDIR:
            prefix = OPAL_FS_ERR_IS_DIRECTORY;
            break;
        case ENOTDIR:
            prefix = OPAL_FS_ERR_NOT_A_DIRECTORY;
            break;
        default:
            if (op_prefix && op_prefix[0] != '\0') {
                prefix = op_prefix;
            }
            break;
    }

    char* formatted = opal_fs_format_err(prefix, strerror(err));
    if (formatted) {
        return formatted;
    }

    if (op_prefix && op_prefix[0] != '\0') {
        return opal_fs_format_err(op_prefix, "failed to allocate error message");
    }
    return opal_fs_format_err(OPAL_FS_ERR_IO, "failed to allocate error message");
}

static int opal_is_continuation_byte(unsigned char byte) {
    return (byte & 0xC0u) == 0x80u;
}

static int opal_validate_utf8(const unsigned char* bytes, size_t length, size_t* invalid_offset) {
    size_t i = 0;

    while (i < length) {
        unsigned char first = bytes[i];

        if (first <= 0x7F) {
            i++;
            continue;
        }

        if (first >= 0xC2 && first <= 0xDF) {
            if (i + 1 >= length || !opal_is_continuation_byte(bytes[i + 1])) {
                if (invalid_offset) *invalid_offset = i;
                return 0;
            }
            i += 2;
            continue;
        }

        if (first == 0xE0) {
            if (i + 2 >= length || bytes[i + 1] < 0xA0 || bytes[i + 1] > 0xBF || !opal_is_continuation_byte(bytes[i + 2])) {
                if (invalid_offset) *invalid_offset = i;
                return 0;
            }
            i += 3;
            continue;
        }

        if ((first >= 0xE1 && first <= 0xEC) || (first >= 0xEE && first <= 0xEF)) {
            if (i + 2 >= length || !opal_is_continuation_byte(bytes[i + 1]) || !opal_is_continuation_byte(bytes[i + 2])) {
                if (invalid_offset) *invalid_offset = i;
                return 0;
            }
            i += 3;
            continue;
        }

        if (first == 0xED) {
            if (i + 2 >= length || bytes[i + 1] < 0x80 || bytes[i + 1] > 0x9F || !opal_is_continuation_byte(bytes[i + 2])) {
                if (invalid_offset) *invalid_offset = i;
                return 0;
            }
            i += 3;
            continue;
        }

        if (first == 0xF0) {
            if (i + 3 >= length || bytes[i + 1] < 0x90 || bytes[i + 1] > 0xBF || !opal_is_continuation_byte(bytes[i + 2]) || !opal_is_continuation_byte(bytes[i + 3])) {
                if (invalid_offset) *invalid_offset = i;
                return 0;
            }
            i += 4;
            continue;
        }

        if (first >= 0xF1 && first <= 0xF3) {
            if (i + 3 >= length || !opal_is_continuation_byte(bytes[i + 1]) || !opal_is_continuation_byte(bytes[i + 2]) || !opal_is_continuation_byte(bytes[i + 3])) {
                if (invalid_offset) *invalid_offset = i;
                return 0;
            }
            i += 4;
            continue;
        }

        if (first == 0xF4) {
            if (i + 3 >= length || bytes[i + 1] < 0x80 || bytes[i + 1] > 0x8F || !opal_is_continuation_byte(bytes[i + 2]) || !opal_is_continuation_byte(bytes[i + 3])) {
                if (invalid_offset) *invalid_offset = i;
                return 0;
            }
            i += 4;
            continue;
        }

        if (invalid_offset) *invalid_offset = i;
        return 0;
    }

    return 1;
}

FsStringResult read_text_sync(const char* path) {
    FsStringResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat(path, &stat_result) != 0) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    if (stat_result.is_directory) {
        r.error = opal_fs_format_err(OPAL_FS_ERR_IS_DIRECTORY, "path is a directory");
        if (!r.error) {
            r.error = safe_strdup("IsADirectoryError: path is a directory");
        }
        return r;
    }

    FILE* file = fopen(path, "rb");
    if (!file) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    size_t capacity = 4096;
    char* buffer = (char*)malloc(capacity + 1);
    if (!buffer) {
        fclose(file);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: out of memory");
        }
        return r;
    }

    size_t length = 0;
    for (;;) {
        if (length == capacity) {
            size_t new_capacity = capacity <= (SIZE_MAX / 2) ? capacity * 2 : SIZE_MAX;
            if (new_capacity <= capacity || new_capacity > SIZE_MAX - 1) {
                free(buffer);
                fclose(file);
                r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "file too large");
                if (!r.error) {
                    r.error = safe_strdup("ReadFailureError: file too large");
                }
                return r;
            }

            char* grown = (char*)realloc(buffer, new_capacity + 1);
            if (!grown) {
                free(buffer);
                fclose(file);
                r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
                if (!r.error) {
                    r.error = safe_strdup("ReadFailureError: out of memory");
                }
                return r;
            }
            buffer = grown;
            capacity = new_capacity;
        }

        size_t read_now = fread(buffer + length, 1, capacity - length, file);
        length += read_now;

        if (read_now == 0) {
            if (ferror(file)) {
                int read_errno = errno ? errno : EIO;
                free(buffer);
                fclose(file);
                r.error = errno_to_fs_error(read_errno, OPAL_FS_ERR_IO);
                return r;
            }
            break;
        }
    }

    if (fclose(file) != 0) {
        int close_errno = errno ? errno : EIO;
        free(buffer);
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
        return r;
    }

    buffer[length] = '\0';

    size_t invalid_offset = 0;
    if (!opal_validate_utf8((const unsigned char*)buffer, length, &invalid_offset)) {
        char detail[32];
        snprintf(detail, sizeof(detail), "%zu", invalid_offset);
        free(buffer);
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_UTF8, detail);
        if (!r.error) {
            r.error = safe_strdup("InvalidUtf8Error: failed to allocate error message");
        }
        return r;
    }

    r.value = buffer;
    return r;
}

FsStringResult read_first_line_sync(const char* path) {
    FsStringResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat(path, &stat_result) != 0) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    if (stat_result.is_directory) {
        r.error = opal_fs_format_err(OPAL_FS_ERR_IS_DIRECTORY, "path is a directory");
        if (!r.error) {
            r.error = safe_strdup("IsADirectoryError: path is a directory");
        }
        return r;
    }

    FILE* file = fopen(path, "rb");
    if (!file) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    size_t capacity = 256;
    char* buffer = (char*)malloc(capacity + 1);
    if (!buffer) {
        fclose(file);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: out of memory");
        }
        return r;
    }

    size_t length = 0;
    int saw_newline = 0;
    int first_char = fgetc(file);
    if (first_char == EOF) {
        if (ferror(file)) {
            int read_errno = errno ? errno : EIO;
            free(buffer);
            fclose(file);
            r.error = errno_to_fs_error(read_errno, OPAL_FS_ERR_IO);
            return r;
        }

        free(buffer);
        if (fclose(file) != 0) {
            int close_errno = errno ? errno : EIO;
            r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
            return r;
        }

        r.error = opal_fs_format_err(OPAL_FS_ERR_OUT_OF_BOUNDS, "file is empty");
        if (!r.error) {
            r.error = safe_strdup("OffsetOutOfRangeError: file is empty");
        }
        return r;
    }

    int current = first_char;
    while (current != EOF) {
        if (current == '\n') {
            saw_newline = 1;
            break;
        }

        if (length == capacity) {
            size_t doubled = capacity * 2;
            if (doubled > 16 * 1024 * 1024) {
                free(buffer);
                fclose(file);
                r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "first line exceeds 16MB limit");
                if (!r.error) {
                    r.error = safe_strdup("ReadFailureError: first line exceeds 16MB limit");
                }
                return r;
            }

            char* grown = (char*)realloc(buffer, doubled + 1);
            if (!grown) {
                free(buffer);
                fclose(file);
                r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
                if (!r.error) {
                    r.error = safe_strdup("ReadFailureError: out of memory");
                }
                return r;
            }
            buffer = grown;
            capacity = doubled;
        }

        buffer[length++] = (char)current;
        current = fgetc(file);
    }

    if (!saw_newline && current == EOF && ferror(file)) {
        int read_errno = errno ? errno : EIO;
        free(buffer);
        fclose(file);
        r.error = errno_to_fs_error(read_errno, OPAL_FS_ERR_IO);
        return r;
    }

    if (fclose(file) != 0) {
        int close_errno = errno ? errno : EIO;
        free(buffer);
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
        return r;
    }

    if (length > 0 && buffer[length - 1] == '\r') {
        buffer[--length] = '\0';
    }

    buffer[length] = '\0';

    size_t invalid_offset = 0;
    if (!opal_validate_utf8((const unsigned char*)buffer, length, &invalid_offset)) {
        char detail[32];
        snprintf(detail, sizeof(detail), "%zu", invalid_offset);
        free(buffer);
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_UTF8, detail);
        if (!r.error) {
            r.error = safe_strdup("InvalidUtf8Error: failed to allocate error message");
        }
        return r;
    }

    r.value = buffer;
    return r;
}

FsStringArrayResult read_lines_sync(const char* path) {
    FsStringArrayResult r;
    r.value = NULL;
    r.count = 0;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat(path, &stat_result) != 0) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    if (stat_result.is_directory) {
        r.error = opal_fs_format_err(OPAL_FS_ERR_IS_DIRECTORY, "path is a directory");
        if (!r.error) {
            r.error = safe_strdup("IsADirectoryError: path is a directory");
        }
        return r;
    }

    FILE* file = fopen(path, "rb");
    if (!file) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    size_t capacity = 4096;
    char* buffer = (char*)malloc(capacity + 1);
    if (!buffer) {
        fclose(file);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: out of memory");
        }
        return r;
    }

    size_t length = 0;
    for (;;) {
        if (length == capacity) {
            size_t new_capacity = capacity <= (SIZE_MAX / 2) ? capacity * 2 : SIZE_MAX;
            if (new_capacity <= capacity || new_capacity > SIZE_MAX - 1) {
                free(buffer);
                fclose(file);
                r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "file too large");
                if (!r.error) {
                    r.error = safe_strdup("ReadFailureError: file too large");
                }
                return r;
            }

            char* grown = (char*)realloc(buffer, new_capacity + 1);
            if (!grown) {
                free(buffer);
                fclose(file);
                r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
                if (!r.error) {
                    r.error = safe_strdup("ReadFailureError: out of memory");
                }
                return r;
            }
            buffer = grown;
            capacity = new_capacity;
        }

        size_t read_now = fread(buffer + length, 1, capacity - length, file);
        length += read_now;

        if (read_now == 0) {
            if (ferror(file)) {
                int read_errno = errno ? errno : EIO;
                free(buffer);
                fclose(file);
                r.error = errno_to_fs_error(read_errno, OPAL_FS_ERR_IO);
                return r;
            }
            break;
        }
    }

    if (fclose(file) != 0) {
        int close_errno = errno ? errno : EIO;
        free(buffer);
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
        return r;
    }

    buffer[length] = '\0';

    size_t invalid_offset = 0;
    if (!opal_validate_utf8((const unsigned char*)buffer, length, &invalid_offset)) {
        char detail[32];
        snprintf(detail, sizeof(detail), "%zu", invalid_offset);
        free(buffer);
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_UTF8, detail);
        if (!r.error) {
            r.error = safe_strdup("InvalidUtf8Error: failed to allocate error message");
        }
        return r;
    }

    char* normalized = (char*)malloc(length + 1);
    if (!normalized) {
        free(buffer);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: out of memory");
        }
        return r;
    }

    size_t normalized_len = 0;
    for (size_t i = 0; i < length; i++) {
        if (buffer[i] == '\r' && i + 1 < length && buffer[i + 1] == '\n') {
            continue;
        }
        normalized[normalized_len++] = buffer[i];
    }
    normalized[normalized_len] = '\0';
    free(buffer);

    if (normalized_len == 0) {
        free(normalized);
        return r;
    }

    size_t newline_count = 0;
    for (size_t i = 0; i < normalized_len; i++) {
        if (normalized[i] == '\n') {
            newline_count++;
        }
    }

    size_t line_count = newline_count + 1;
    if (normalized[normalized_len - 1] == '\n') {
        line_count--;
    }

    if (line_count == 0) {
        free(normalized);
        return r;
    }

    if (line_count > (size_t)INT64_MAX) {
        free(normalized);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "too many lines");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: too many lines");
        }
        return r;
    }

    char** lines = (char**)calloc(line_count, sizeof(char*));
    if (!lines) {
        free(normalized);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: out of memory");
        }
        return r;
    }

    size_t line_index = 0;
    size_t start = 0;
    for (size_t i = 0; i <= normalized_len; i++) {
        int at_end = i == normalized_len;
        if (!at_end && normalized[i] != '\n') {
            continue;
        }

        if (at_end && start == normalized_len && normalized[normalized_len - 1] == '\n') {
            break;
        }

        size_t line_len = i - start;
        char* line = (char*)malloc(line_len + 1);
        if (!line) {
            free_string_array_elements(lines, line_index);
            free(lines);
            free(normalized);
            r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
            if (!r.error) {
                r.error = safe_strdup("ReadFailureError: out of memory");
            }
            return r;
        }

        if (line_len > 0) {
            memcpy(line, normalized + start, line_len);
        }
        line[line_len] = '\0';

        lines[line_index++] = line;
        start = i + 1;
    }

    free(normalized);
    r.value = lines;
    r.count = (int64_t)line_index;
    return r;
}

FsBytesResult read_bytes_at_offset_sync(const char* path, int64_t offset, int64_t length) {
    (void)path;
    (void)offset;
    (void)length;
    FsBytesResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsVoidResult write_contents_sync(const char* path, OpalBytes* data) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    const uint8_t* write_bytes = NULL;
    size_t write_len = 0;
    if (data) {
        write_len = data->length;
        write_bytes = data->data;
    }

    FILE* file = fopen(path, "wb");
    if (!file) {
        r.error = errno_to_fs_error(errno, "Io");
        return r;
    }

    char* write_error = fwrite_all(file, write_bytes, write_len);
    if (write_error) {
        r.error = write_error;
        fclose(file);
        return r;
    }

    if (fclose(file) != 0) {
        int close_errno = errno ? errno : EIO;
        char detail[256];
        snprintf(detail, sizeof(detail), "close failed: %s", strerror(close_errno));
        r.error = opal_fs_format_err("Io", detail);
        if (!r.error) {
            r.error = safe_strdup("Io: close failed");
        }
        return r;
    }

    return r;
}

FsVoidResult write_text_sync(const char* path, const char* text) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    const char* write_text = text ? text : "";
    size_t write_len = strlen(write_text);

    FILE* file = fopen(path, "wb");
    if (!file) {
        r.error = errno_to_fs_error(errno, "Io");
        return r;
    }

    char* write_error = fwrite_all(file, (const uint8_t*)write_text, write_len);
    if (write_error) {
        r.error = write_error;
        fclose(file);
        return r;
    }

    if (fclose(file) != 0) {
        int close_errno = errno ? errno : EIO;
        char detail[256];
        snprintf(detail, sizeof(detail), "close failed: %s", strerror(close_errno));
        r.error = opal_fs_format_err("Io", detail);
        if (!r.error) {
            r.error = safe_strdup("Io: close failed");
        }
        return r;
    }

    return r;
}

FsVoidResult write_contents_atomic_sync(const char* path, OpalBytes* data) {
    (void)path;
    (void)data;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsVoidResult write_text_atomic_sync(const char* path, const char* text) {
    (void)path;
    (void)text;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsVoidResult append_contents_sync(const char* path, OpalBytes* data) {
    (void)path;
    (void)data;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsVoidResult append_text_sync(const char* path, const char* text) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    const char* write_text = text ? text : "";
    size_t write_len = strlen(write_text);

    FILE* file = fopen(path, "ab");
    if (!file) {
        r.error = errno_to_fs_error(errno, "Io");
        return r;
    }

    char* write_error = fwrite_all(file, (const uint8_t*)write_text, write_len);
    if (write_error) {
        r.error = write_error;
        fclose(file);
        return r;
    }

    if (fclose(file) != 0) {
        int close_errno = errno ? errno : EIO;
        char detail[256];
        snprintf(detail, sizeof(detail), "close failed: %s", strerror(close_errno));
        r.error = opal_fs_format_err("Io", detail);
        if (!r.error) {
            r.error = safe_strdup("Io: close failed");
        }
        return r;
    }

    return r;
}

FsVoidResult write_bytes_at_offset_sync(const char* path, int64_t offset, OpalBytes* data) {
    (void)path;
    (void)offset;
    (void)data;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsVoidResult create_file_sync(const char* path) {
    (void)path;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsVoidResult delete_file_sync(const char* path) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    if (opal_unlink(path) != 0) {
        int unlink_errno = errno;
        if (unlink_errno == EISDIR || unlink_errno == EPERM) {
            struct opal_stat_result stat_result;
            if (opal_stat(path, &stat_result) == 0 && stat_result.is_directory) {
                r.error = opal_fs_format_err(OPAL_FS_ERR_IS_DIRECTORY, "path is a directory");
                if (!r.error) {
                    r.error = safe_strdup("IsADirectoryError: path is a directory");
                }
                return r;
            }
        }
        r.error = errno_to_fs_error(unlink_errno, OPAL_FS_ERR_IO);
        return r;
    }

    return r;
}

FsVoidResult copy_file_sync(const char* source, const char* destination) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!source || source[0] == '\0' || !destination || destination[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    struct opal_stat_result source_stat;
    if (opal_stat(source, &source_stat) == 0 && source_stat.is_directory) {
        r.error = opal_fs_format_err(OPAL_FS_ERR_IS_DIRECTORY, "source is a directory");
        if (!r.error) {
            r.error = safe_strdup("IsADirectoryError: source is a directory");
        }
        return r;
    }

    struct opal_stat_result destination_stat;
    if (opal_stat(destination, &destination_stat) == 0 && destination_stat.is_directory) {
        r.error = opal_fs_format_err(OPAL_FS_ERR_IS_DIRECTORY, "destination is a directory");
        if (!r.error) {
            r.error = safe_strdup("IsADirectoryError: destination is a directory");
        }
        return r;
    }

#if OPAL_WINDOWS
    struct _stat64 source_native_stat;
    struct _stat64 destination_native_stat;
    if (_stat64(source, &source_native_stat) == 0 && _stat64(destination, &destination_native_stat) == 0) {
        if (source_native_stat.st_dev == destination_native_stat.st_dev &&
            source_native_stat.st_ino == destination_native_stat.st_ino) {
            return r;
        }
    }
#else
    struct stat source_native_stat;
    struct stat destination_native_stat;
    if (stat(source, &source_native_stat) == 0 && stat(destination, &destination_native_stat) == 0) {
        if (source_native_stat.st_dev == destination_native_stat.st_dev &&
            source_native_stat.st_ino == destination_native_stat.st_ino) {
            return r;
        }
    }
#endif

    FILE* source_file = fopen(source, "rb");
    if (!source_file) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    FILE* destination_file = fopen(destination, "wb");
    if (!destination_file) {
        int open_errno = errno;
        if (fclose(source_file) != 0 && open_errno == 0) {
            open_errno = errno ? errno : EIO;
        }
        r.error = errno_to_fs_error(open_errno ? open_errno : EIO, OPAL_FS_ERR_IO);
        return r;
    }

    uint8_t buffer[64 * 1024];
    for (;;) {
        size_t read_count = fread(buffer, 1, sizeof(buffer), source_file);
        if (read_count > 0) {
            size_t written_total = 0;
            while (written_total < read_count) {
                size_t written_now = fwrite(buffer + written_total, 1, read_count - written_total, destination_file);
                if (written_now == 0) {
                    int write_errno = ferror(destination_file) ? (errno ? errno : EIO) : EIO;
                    r.error = errno_to_fs_error(write_errno, OPAL_FS_ERR_IO);
                    break;
                }
                written_total += written_now;
            }
            if (r.error) {
                break;
            }
        }

        if (read_count < sizeof(buffer)) {
            if (ferror(source_file)) {
                int read_errno = errno ? errno : EIO;
                if (!r.error) {
                    r.error = errno_to_fs_error(read_errno, OPAL_FS_ERR_IO);
                }
            }
            break;
        }
    }

    if (fclose(destination_file) != 0 && !r.error) {
        int close_errno = errno ? errno : EIO;
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
    }

    if (fclose(source_file) != 0 && !r.error) {
        int close_errno = errno ? errno : EIO;
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
    }

    return r;
}

FsVoidResult move_path_sync(const char* source, const char* destination) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!source || source[0] == '\0' || !destination || destination[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    if (rename(source, destination) != 0) {
        int rename_errno = errno;
        if (rename_errno == EXDEV) {
            r.error = opal_fs_format_err("Io", "EXDEV: cross-device rename not supported (caller should copy+delete)");
            if (!r.error) {
                r.error = safe_strdup("Io: EXDEV: cross-device rename not supported (caller should copy+delete)");
            }
            return r;
        }

        r.error = errno_to_fs_error(rename_errno, "MoveFailureError");
        return r;
    }

    return r;
}

FsBooleanResult path_exists_sync(const char* path) {
    FsBooleanResult r;
    r.value = 0;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat(path, &stat_result) == 0) {
        r.value = 1;
        return r;
    }

    int stat_errno = errno;
    if (stat_errno == ENOENT) {
        r.value = 0;
        return r;
    }

    r.error = errno_to_fs_error(stat_errno, OPAL_FS_ERR_IO);
    return r;
}

FsMetadataResult read_metadata_sync(const char* path) {
    FsMetadataResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat(path, &stat_result) != 0) {
        int stat_errno = errno;
        if (stat_errno == ENOENT) {
            r.error = opal_fs_format_err(OPAL_FS_ERR_NOT_FOUND, "path not found");
            if (!r.error) {
                r.error = safe_strdup("FileNotFoundError: path not found");
            }
            return r;
        }

        r.error = errno_to_fs_error(stat_errno, OPAL_FS_ERR_METADATA_UNAVAILABLE);
        return r;
    }

    OpalFileMetadata* metadata = (OpalFileMetadata*)malloc(sizeof(OpalFileMetadata));
    if (!metadata) {
        r.error = opal_fs_format_err(OPAL_FS_ERR_METADATA_UNAVAILABLE, "out of memory");
        if (!r.error) {
            r.error = safe_strdup("MetadataUnavailableError: out of memory");
        }
        return r;
    }

    metadata->size_bytes = stat_result.size;
    metadata->modified_unix_seconds = stat_result.modified_time;
    metadata->is_directory = stat_result.is_directory ? 1 : 0;
    metadata->is_symlink = 0;

    r.value = metadata;
    return r;
}

FsMetadataResult read_metadata_nofollow_sync(const char* path) {
    (void)path;
    FsMetadataResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsVoidResult create_directory_sync(const char* path) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    if (opal_mkdir(path) != 0) {
        int mkdir_errno = errno;
        if (mkdir_errno == EEXIST) {
            r.error = opal_fs_format_err(OPAL_FS_ERR_ALREADY_EXISTS, strerror(mkdir_errno));
            if (!r.error) {
                r.error = safe_strdup("FileAlreadyExistsError: file exists");
            }
            return r;
        }

        r.error = errno_to_fs_error(mkdir_errno, "CreateFailureError");
        return r;
    }

    return r;
}

FsVoidResult create_directory_recursive_sync(const char* path) {
    (void)path;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsVoidResult delete_directory_sync(const char* path) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    if (opal_rmdir(path) != 0) {
        int rmdir_errno = errno;
        if (rmdir_errno == ENOTEMPTY || rmdir_errno == EEXIST) {
            r.error = opal_fs_format_err("Io", "directory not empty");
            if (!r.error) {
                r.error = safe_strdup("Io: directory not empty");
            }
            return r;
        }

        r.error = errno_to_fs_error(rmdir_errno, "DeleteFailureError");
        return r;
    }

    return r;
}

FsVoidResult delete_directory_recursive_sync(const char* path) {
    (void)path;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsPathArrayResult list_directory_sync(const char* path) {
    FsPathArrayResult r;
    r.value = NULL;
    r.count = 0;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    opal_dir_t* dir = opal_opendir(path);
    if (!dir) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    size_t capacity = 8;
    size_t count = 0;
    char** entries = (char**)calloc(capacity, sizeof(char*));
    if (!entries) {
        int saved_errno = errno ? errno : ENOMEM;
        opal_closedir(dir);
        r.error = errno_to_fs_error(saved_errno, OPAL_FS_ERR_IO);
        return r;
    }

    for (;;) {
        errno = 0;
        opal_dirent_t* entry = opal_readdir(dir);
        if (!entry) {
            if (errno != 0) {
                int read_errno = errno;
                free_string_array_elements(entries, count);
                free(entries);
                opal_closedir(dir);
                r.error = errno_to_fs_error(read_errno, OPAL_FS_ERR_IO);
                return r;
            }
            break;
        }

        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
            continue;
        }

        if (count == capacity) {
            size_t new_capacity = capacity * 2;
            if (new_capacity <= capacity) {
                free_string_array_elements(entries, count);
                free(entries);
                opal_closedir(dir);
                r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "directory entry overflow");
                if (!r.error) {
                    r.error = safe_strdup("ReadFailureError: directory entry overflow");
                }
                return r;
            }

            char** grown = (char**)realloc(entries, new_capacity * sizeof(char*));
            if (!grown) {
                int alloc_errno = errno ? errno : ENOMEM;
                free_string_array_elements(entries, count);
                free(entries);
                opal_closedir(dir);
                r.error = errno_to_fs_error(alloc_errno, OPAL_FS_ERR_IO);
                return r;
            }

            entries = grown;
            capacity = new_capacity;
        }

        entries[count] = safe_strdup(entry->d_name);
        if (!entries[count]) {
            int alloc_errno = errno ? errno : ENOMEM;
            free_string_array_elements(entries, count);
            free(entries);
            opal_closedir(dir);
            r.error = errno_to_fs_error(alloc_errno, OPAL_FS_ERR_IO);
            return r;
        }
        count++;
    }

    if (opal_closedir(dir) != 0) {
        int close_errno = errno ? errno : EIO;
        free_string_array_elements(entries, count);
        free(entries);
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
        return r;
    }

    if (count > 1) {
        qsort(entries, count, sizeof(char*), compare_string_pointers);
    }

    if (count > (size_t)INT64_MAX) {
        free_string_array_elements(entries, count);
        free(entries);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "directory entry overflow");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: directory entry overflow");
        }
        return r;
    }

    r.value = entries;
    r.count = (int64_t)count;
    return r;
}

FsBooleanResult is_file_sync(const char* path) {
    FsBooleanResult r;
    r.value = 0;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat(path, &stat_result) != 0) {
        int stat_errno = errno;
        if (stat_errno == ENOENT) {
            r.value = 0;
            return r;
        }

        r.error = errno_to_fs_error(stat_errno, OPAL_FS_ERR_IO);
        return r;
    }

    r.value = stat_result.is_directory ? 0 : 1;
    return r;
}

FsBooleanResult is_file_nofollow_sync(const char* path) {
    (void)path;
    FsBooleanResult r;
    r.value = 0;
    r.error = "not implemented";
    return r;
}

FsBooleanResult is_directory_sync(const char* path) {
    FsBooleanResult r;
    r.value = 0;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat(path, &stat_result) != 0) {
        int stat_errno = errno;
        if (stat_errno == ENOENT) {
            r.value = 0;
            return r;
        }

        r.error = errno_to_fs_error(stat_errno, OPAL_FS_ERR_IO);
        return r;
    }

    r.value = stat_result.is_directory ? 1 : 0;
    return r;
}

FsBooleanResult is_directory_nofollow_sync(const char* path) {
    (void)path;
    FsBooleanResult r;
    r.value = 0;
    r.error = "not implemented";
    return r;
}
