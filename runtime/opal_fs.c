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

#if !defined(OPAL_RC_DEBUG_NOTES_IMPLEMENTED) && (defined(__GNUC__) || defined(__clang__))
__attribute__((weak)) void opal_rc_debug_note_alloc(OpalRcDebugCounterKind kind) {
    (void)kind;
}

__attribute__((weak)) void opal_rc_debug_note_free(OpalRcDebugCounterKind kind) {
    (void)kind;
}
#endif

#ifndef OPAL_BYTES_TYPE_DEFINED
typedef struct OpalBytes {
    size_t length;
    uint8_t* data;
} OpalBytes;
#define OPAL_BYTES_TYPE_DEFINED 1
#endif

#ifndef OPAL_FS_VOID_RESULT_TYPE_DEFINED
typedef struct { void*      value; const char* error; } FsVoidResult;
#define OPAL_FS_VOID_RESULT_TYPE_DEFINED 1
#endif
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

/* Windows directory iteration shims are defined in opal_portability.h. */

static char* safe_strdup(const char* value) {
    char* copy = opal_strdup(value ? value : "");
    if (copy) {
        opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);
    }
    return copy;
}

static char* errno_to_fs_error(int err, const char* op_prefix);
static char* fwrite_all(FILE* f, const uint8_t* buf, size_t len, const char* write_prefix);
static char* fs_invalid_path_error(void);
static char* fs_out_of_memory_error(const char* prefix, const char* fallback);
static char* fs_close_error(const char* prefix, const char* fallback);
static char* fs_offset_out_of_range_error(const char* detail);
static int fs_write_via_mode(const char* path, const uint8_t* bytes, size_t length, const char* mode, const char* write_prefix, FsVoidResult* out);
static FsVoidResult write_contents_atomic_bytes_sync(const char* path, const uint8_t* bytes, size_t length);
static int remove_directory_recursive_inner(const char* path, FsVoidResult* out);

static int opal_is_path_separator(char c) {
    return c == '/' || c == '\\';
}

typedef enum {
    OPAL_PATH_ROOT_NONE = 0,
    OPAL_PATH_ROOT_POSIX,
    OPAL_PATH_ROOT_DRIVE,
    OPAL_PATH_ROOT_UNC,
} opal_path_root_kind_t;

typedef struct {
    opal_path_root_kind_t kind;
    size_t parse_start;
    size_t server_start;
    size_t server_len;
    size_t share_start;
    size_t share_len;
} opal_path_root_info_t;

static int opal_is_ascii_alpha(char c) {
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
}

static opal_path_root_info_t opal_parse_path_root(const char* path) {
    opal_path_root_info_t info;
    memset(&info, 0, sizeof(info));

    if (!path || path[0] == '\0') {
        return info;
    }

    if (opal_is_ascii_alpha(path[0]) && path[1] == ':' && opal_is_path_separator(path[2])) {
        size_t index = 2;
        while (path[index] && opal_is_path_separator(path[index])) {
            index++;
        }
        info.kind = OPAL_PATH_ROOT_DRIVE;
        info.parse_start = index;
        return info;
    }

    if (opal_is_path_separator(path[0])) {
        if (opal_is_path_separator(path[1])) {
            size_t server_start = 2;
            size_t index = server_start;
            while (path[index] && !opal_is_path_separator(path[index])) {
                index++;
            }

            if (index > server_start) {
                size_t server_len = index - server_start;
                while (path[index] && opal_is_path_separator(path[index])) {
                    index++;
                }

                size_t share_start = index;
                while (path[index] && !opal_is_path_separator(path[index])) {
                    index++;
                }

                if (index > share_start) {
                    info.kind = OPAL_PATH_ROOT_UNC;
                    info.parse_start = index;
                    info.server_start = server_start;
                    info.server_len = server_len;
                    info.share_start = share_start;
                    info.share_len = index - share_start;
                    return info;
                }
            }
        }

        info.kind = OPAL_PATH_ROOT_POSIX;
        info.parse_start = 1;
        while (path[info.parse_start] && opal_is_path_separator(path[info.parse_start])) {
            info.parse_start++;
        }
    }

    return info;
}

static size_t opal_trimmed_path_end(const char* path, const opal_path_root_info_t* root) {
    size_t end = strlen(path);
    while (end > root->parse_start && opal_is_path_separator(path[end - 1])) {
        end--;
    }
    return end;
}

static ssize_t opal_find_last_path_separator(const char* path, size_t end) {
    size_t index = end;
    while (index > 0) {
        index--;
        if (opal_is_path_separator(path[index])) {
            return (ssize_t)index;
        }
    }
    return -1;
}

static char* opal_strdup_slice(const char* start, size_t length) {
    char* copy = (char*)malloc(length + 1);
    if (!copy) {
        return NULL;
    }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
    if (length > 0) {
        memcpy(copy, start, length);
    }
    copy[length] = '\0';
    return copy;
}

static char* opal_strdup_root_preserving_style(const char* path, const opal_path_root_info_t* root) {
    if (!root || root->kind == OPAL_PATH_ROOT_NONE) {
        return safe_strdup("");
    }

    if (root->kind == OPAL_PATH_ROOT_POSIX) {
        char sep[2] = { path[0] ? path[0] : opal_path_separator(), '\0' };
        return safe_strdup(sep);
    }

    if (root->kind == OPAL_PATH_ROOT_DRIVE) {
        char sep[2] = { opal_is_path_separator(path[2]) ? path[2] : opal_path_separator(), '\0' };
        char drive_root[4] = { path[0], ':', sep[0], '\0' };
        return safe_strdup(drive_root);
    }

    if (root->kind == OPAL_PATH_ROOT_UNC) {
        char sep = opal_is_path_separator(path[0]) ? path[0] : opal_path_separator();
        size_t length = 2 + root->server_len + 1 + root->share_len;
        char* value = (char*)malloc(length + 1);
        if (!value) {
            return NULL;
        }
        opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);

        size_t position = 0;
        value[position++] = sep;
        value[position++] = sep;
        memcpy(value + position, path + root->server_start, root->server_len);
        position += root->server_len;
        value[position++] = sep;
        memcpy(value + position, path + root->share_start, root->share_len);
        position += root->share_len;
        value[position] = '\0';
        return value;
    }

    return safe_strdup("");
}

static size_t opal_normalized_root_length(const opal_path_root_info_t* root) {
    if (!root) {
        return 0;
    }

    switch (root->kind) {
        case OPAL_PATH_ROOT_POSIX:
            return 1;
        case OPAL_PATH_ROOT_DRIVE:
            return 2;
        case OPAL_PATH_ROOT_UNC:
            return 2 + root->server_len + 1 + root->share_len;
        case OPAL_PATH_ROOT_NONE:
        default:
            return 0;
    }
}

static int opal_root_needs_separator_before_first_segment(const opal_path_root_info_t* root) {
    if (!root) {
        return 0;
    }

    return root->kind == OPAL_PATH_ROOT_DRIVE || root->kind == OPAL_PATH_ROOT_UNC;
}

static size_t opal_append_normalized_root(char* output, const char* path, const opal_path_root_info_t* root) {
    char sep = opal_path_separator();
    size_t position = 0;

    if (!root) {
        return 0;
    }

    switch (root->kind) {
        case OPAL_PATH_ROOT_POSIX:
            output[position++] = sep;
            break;
        case OPAL_PATH_ROOT_DRIVE:
            output[position++] = path[0];
            output[position++] = ':';
            break;
        case OPAL_PATH_ROOT_UNC:
            output[position++] = sep;
            output[position++] = sep;
            memcpy(output + position, path + root->server_start, root->server_len);
            position += root->server_len;
            output[position++] = sep;
            memcpy(output + position, path + root->share_start, root->share_len);
            position += root->share_len;
            break;
        case OPAL_PATH_ROOT_NONE:
        default:
            break;
    }

    return position;
}

static void free_path_segments(char** segments, int64_t count) {
    if (!segments) return;
    for (int64_t i = 0; i < count; i++) {
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
        free(segments[i]);
    }
}

static void free_string_array_elements(char** values, size_t count) {
    if (!values) return;
    for (size_t i = 0; i < count; i++) {
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
        free(values[i]);
    }
}

static int compare_string_pointers(const void* lhs, const void* rhs) {
    const char* const* left = (const char* const*)lhs;
    const char* const* right = (const char* const*)rhs;
    return strcmp(*left, *right);
}

static char* fs_invalid_path_error(void) {
    char* error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
    if (!error) {
        error = safe_strdup("InvalidPathError: empty path");
    }
    return error;
}

static char* fs_out_of_memory_error(const char* prefix, const char* fallback) {
    char* error = opal_fs_format_err(prefix, "out of memory");
    if (!error) {
        error = safe_strdup(fallback);
    }
    return error;
}

static char* fs_close_error(const char* prefix, const char* fallback) {
    int close_errno = errno ? errno : EIO;
    char detail[256];
    snprintf(detail, sizeof(detail), "close failed: %s", strerror(close_errno));
    char* error = opal_fs_format_err(prefix, detail);
    if (!error) {
        error = safe_strdup(fallback);
    }
    return error;
}

static char* fs_offset_out_of_range_error(const char* detail) {
    char* error = opal_fs_format_err(OPAL_FS_ERR_OUT_OF_BOUNDS, detail);
    if (!error) {
        error = safe_strdup("OffsetOutOfRangeError: offset out of range");
    }
    return error;
}

static int fs_write_via_mode(const char* path, const uint8_t* bytes, size_t length, const char* mode, const char* write_prefix, FsVoidResult* out) {
    FILE* file = opal_fopen(path, mode);
    if (!file) {
        out->error = errno_to_fs_error(errno, write_prefix);
        return -1;
    }

    char* write_error = fwrite_all(file, bytes, length, write_prefix);
    if (write_error) {
        out->error = write_error;
        fclose(file);
        return -1;
    }

    if (fclose(file) != 0) {
        char fallback[96];
        snprintf(fallback, sizeof(fallback), "%s: close failed", write_prefix);
        out->error = fs_close_error(write_prefix, fallback);
        return -1;
    }

    return 0;
}

static FsVoidResult write_contents_atomic_bytes_sync(const char* path, const uint8_t* bytes, size_t length) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = fs_invalid_path_error();
        return r;
    }

    struct opal_stat_result path_stat;
    if (opal_stat(path, &path_stat) == 0 && path_stat.is_directory) {
        r.error = opal_fs_format_err(OPAL_FS_ERR_IS_DIRECTORY, "path is a directory");
        if (!r.error) {
            r.error = safe_strdup("IsADirectoryError: path is a directory");
        }
        return r;
    }

    size_t path_len = strlen(path);
    const char suffix[] = ".tmp.XXXXXX";
    size_t suffix_len = sizeof(suffix) - 1;
    if (path_len > SIZE_MAX - suffix_len - 1) {
        r.error = opal_fs_format_err("WriteFailureError", "temporary path too long");
        if (!r.error) {
            r.error = safe_strdup("WriteFailureError: temporary path too long");
        }
        return r;
    }

    char* tmp_path = (char*)malloc(path_len + suffix_len + 1);
    if (!tmp_path) {
        r.error = fs_out_of_memory_error("WriteFailureError", "WriteFailureError: out of memory");
        return r;
    }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);

    memcpy(tmp_path, path, path_len);
    memcpy(tmp_path + path_len, suffix, suffix_len + 1);

    if (opal_create_temp_file(tmp_path, path_len + suffix_len + 1) != 0) {
        if (strcmp(tmp_path, path) == 0) {
            opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
            free(tmp_path);
            r.error = opal_fs_format_err("WriteFailureError", "temporary path collision");
            if (!r.error) {
                r.error = safe_strdup("WriteFailureError: temporary path collision");
            }
            return r;
        }
        int temp_errno = errno ? errno : EIO;
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
        free(tmp_path);
        r.error = errno_to_fs_error(temp_errno, "WriteFailureError");
        return r;
    }

    if (fs_write_via_mode(tmp_path, bytes, length, "wb", "WriteFailureError", &r) != 0) {
        opal_unlink(tmp_path);
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
        free(tmp_path);
        return r;
    }

    if (opal_replace_path(tmp_path, path) != 0) {
        int rename_errno = errno ? errno : EIO;
        opal_unlink(tmp_path);
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
        free(tmp_path);
        r.error = errno_to_fs_error(rename_errno, "WriteFailureError");
        return r;
    }

    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
    free(tmp_path);
    return r;
}

static int remove_directory_recursive_inner(const char* path, FsVoidResult* out) {
    opal_dir_t* dir = opal_opendir(path);
    if (!dir) {
        int open_errno = errno;
        if (open_errno == ENOENT) {
            out->error = opal_fs_format_err(OPAL_FS_ERR_DIRECTORY_NOT_FOUND, "directory not found");
            if (!out->error) {
                out->error = safe_strdup("DirectoryNotFoundError: directory not found");
            }
        } else {
            out->error = errno_to_fs_error(open_errno, "DeleteFailureError");
        }
        return -1;
    }

    for (;;) {
        errno = 0;
        opal_dirent_t* entry = opal_readdir(dir);
        if (!entry) {
            if (errno != 0) {
                int read_errno = errno;
                opal_closedir(dir);
                out->error = errno_to_fs_error(read_errno, "DeleteFailureError");
                return -1;
            }
            break;
        }

        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
            continue;
        }

        size_t path_len = strlen(path);
        size_t name_len = strlen(entry->d_name);
        int need_separator = path_len > 0 && !opal_is_path_separator(path[path_len - 1]);
        if (path_len > SIZE_MAX - name_len - (need_separator ? 2u : 1u)) {
            opal_closedir(dir);
            out->error = opal_fs_format_err("DeleteFailureError", "path too long");
            if (!out->error) {
                out->error = safe_strdup("DeleteFailureError: path too long");
            }
            return -1;
        }

        char* child_path = (char*)malloc(path_len + name_len + (need_separator ? 2u : 1u));
        if (!child_path) {
            opal_closedir(dir);
            out->error = fs_out_of_memory_error("DeleteFailureError", "DeleteFailureError: out of memory");
            return -1;
        }
        opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);

        memcpy(child_path, path, path_len);
        size_t child_len = path_len;
        if (need_separator) {
            child_path[child_len++] = opal_path_separator();
        }
        memcpy(child_path + child_len, entry->d_name, name_len + 1);

        struct opal_stat_result child_stat;
        if (opal_stat_nofollow(child_path, &child_stat) != 0) {
            int stat_errno = errno;
            opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
            free(child_path);
            opal_closedir(dir);
            out->error = errno_to_fs_error(stat_errno, "DeleteFailureError");
            return -1;
        }

        if (child_stat.is_directory && !child_stat.is_symlink) {
            if (remove_directory_recursive_inner(child_path, out) != 0) {
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
                free(child_path);
                opal_closedir(dir);
                return -1;
            }
        } else if (opal_unlink(child_path) != 0) {
            int unlink_errno = errno;
            opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
            free(child_path);
            opal_closedir(dir);
            out->error = errno_to_fs_error(unlink_errno, "DeleteFailureError");
            return -1;
        }

        free(child_path);
    }

    if (opal_closedir(dir) != 0) {
        int close_errno = errno ? errno : EIO;
        out->error = errno_to_fs_error(close_errno, "DeleteFailureError");
        return -1;
    }

    if (opal_rmdir(path) != 0) {
        int rmdir_errno = errno;
        if (rmdir_errno == ENOENT) {
            out->error = opal_fs_format_err(OPAL_FS_ERR_DIRECTORY_NOT_FOUND, "directory not found");
            if (!out->error) {
                out->error = safe_strdup("DirectoryNotFoundError: directory not found");
            }
        } else if (rmdir_errno == ENOTDIR) {
            out->error = opal_fs_format_err(OPAL_FS_ERR_NOT_A_DIRECTORY, "path is not a directory");
            if (!out->error) {
                out->error = safe_strdup("IsNotADirectoryError: path is not a directory");
            }
        } else {
            out->error = errno_to_fs_error(rmdir_errno, "DeleteFailureError");
        }
        return -1;
    }

    return 0;
}

static char* lex_normalize_path(const char* path) {
    if (!path || path[0] == '\0') {
        return safe_strdup("");
    }

    size_t input_len = strlen(path);
    opal_path_root_info_t root = opal_parse_path_root(path);
    int is_absolute = root.kind != OPAL_PATH_ROOT_NONE;
    int had_trailing_separator = input_len > root.parse_start && opal_is_path_separator(path[input_len - 1]);

    char** segments = (char**)calloc(input_len + 1, sizeof(char*));
    if (!segments) {
        return safe_strdup("");
    }

    int64_t segment_count = 0;
    int escaped_root = 0;
    size_t i = root.parse_start;

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
            char* root_only = opal_strdup_root_preserving_style(path, &root);
            return root_only ? root_only : safe_strdup("");
        }
        return safe_strdup("");
    }

    char sep = opal_path_separator();
    int root_needs_first_separator = opal_root_needs_separator_before_first_segment(&root);
    size_t output_len = opal_normalized_root_length(&root);
    for (int64_t index = 0; index < segment_count; index++) {
        output_len += strlen(segments[index]);
        if (index > 0 || (index == 0 && root_needs_first_separator)) {
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

    size_t position = opal_append_normalized_root(output, path, &root);

    for (int64_t index = 0; index < segment_count; index++) {
        if (index > 0 || (index == 0 && root_needs_first_separator)) {
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

        if (opal_parse_path_root(component).kind != OPAL_PATH_ROOT_NONE) {
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
    if (!path || path[0] == '\0') return safe_strdup(".");

    opal_path_root_info_t root = opal_parse_path_root(path);
    size_t full_len = strlen(path);
    int had_trailing_separator = full_len > root.parse_start && opal_is_path_separator(path[full_len - 1]);
    size_t end = opal_trimmed_path_end(path, &root);

    if (had_trailing_separator) {
        if (end <= root.parse_start) {
            char* root_only = opal_strdup_root_preserving_style(path, &root);
            return root_only ? root_only : safe_strdup("");
        }
        return opal_strdup_slice(path, end);
    }

    ssize_t last_separator = opal_find_last_path_separator(path, end);

    if (last_separator < 0) {
        return safe_strdup(".");
    }

    if ((size_t)last_separator < root.parse_start) {
        char* root_only = opal_strdup_root_preserving_style(path, &root);
        return root_only ? root_only : safe_strdup("");
    }

    if (root.kind == OPAL_PATH_ROOT_NONE && (size_t)last_separator == 0) {
        return safe_strdup(".");
    }

    return opal_strdup_slice(path, (size_t)last_separator);
}

char* path_file_name(const char* path) {
    if (!path || path[0] == '\0') return safe_strdup("");

    opal_path_root_info_t root = opal_parse_path_root(path);
    size_t full_len = strlen(path);
    if (full_len > root.parse_start && opal_is_path_separator(path[full_len - 1])) {
        return safe_strdup("");
    }

    size_t end = opal_trimmed_path_end(path, &root);
    if (end <= root.parse_start) {
        return safe_strdup("");
    }

    ssize_t last_separator = opal_find_last_path_separator(path, end);
    size_t name_start = (last_separator < 0) ? 0u : (size_t)last_separator + 1u;
    if (name_start < root.parse_start) {
        name_start = root.parse_start;
    }

    return opal_strdup_slice(path + name_start, end - name_start);
}

char* path_file_extension(const char* path) {
    if (!path || path[0] == '\0') return safe_strdup("");

    char* name = path_file_name(path);
    if (!name) {
        return safe_strdup("");
    }

    const char* dot = strrchr(name, '.');
    if (!dot || dot == name) {
        free(name);
        return safe_strdup("");
    }

    char* extension = safe_strdup(dot + 1);
    free(name);
    return extension ? extension : safe_strdup("");
}

char* normalize_path(const char* path) {
    return lex_normalize_path(path);
}

char* path_to_string(const char* path) {
    return safe_strdup(path);
}

FsPathResult absolute_path_sync(const char* path) {
    FsPathResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "empty path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: empty path");
        }
        return r;
    }

    char* resolved = opal_realpath_owned(path);
    if (!resolved) {
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_PATH, "could not resolve path");
        if (!r.error) {
            r.error = safe_strdup("InvalidPathError: could not resolve path");
        }
        return r;
    }

    r.value = resolved;
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

    FILE* file = opal_fopen(path, "rb");
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
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);

    size_t length = 0;
    for (;;) {
        if (length == capacity) {
            size_t new_capacity = capacity <= (SIZE_MAX / 2) ? capacity * 2 : SIZE_MAX;
            if (new_capacity <= capacity) {
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
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
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
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
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
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
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
        free(buffer);
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
        return r;
    }

    OpalBytes* bytes = (OpalBytes*)malloc(sizeof(OpalBytes));
    if (!bytes) {
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
        free(buffer);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: out of memory");
        }
        return r;
    }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_BYTES);

    bytes->length = length;
    if (length == 0) {
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
        free(buffer);
        bytes->data = NULL;
    } else {
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS);
        bytes->data = buffer;
    }

    r.value = bytes;
    return r;
}

static char* fwrite_all(FILE* f, const uint8_t* buf, size_t len, const char* write_prefix) {
    const char* prefix = (write_prefix && write_prefix[0] != '\0') ? write_prefix : "WriteFailureError";
    if (!f) {
        return opal_fs_format_err(prefix, "invalid file handle");
    }
    if (!buf && len > 0) {
        return opal_fs_format_err(prefix, "invalid write buffer");
    }

    size_t total_written = 0;
    while (total_written < len) {
        size_t wrote_now = fwrite(buf + total_written, 1, len - total_written, f);
        if (wrote_now == 0) {
            if (ferror(f)) {
                int write_errno = errno ? errno : EIO;
                return errno_to_fs_error(write_errno, prefix);
            }

            char detail[96];
            snprintf(detail, sizeof(detail), "short write (%zu/%zu)", total_written, len);
            return opal_fs_format_err(prefix, detail);
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

    FILE* file = opal_fopen(path, "rb");
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
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);

    size_t length = 0;
    for (;;) {
        if (length == capacity) {
            size_t new_capacity = capacity <= (SIZE_MAX / 2) ? capacity * 2 : SIZE_MAX;
            if (new_capacity <= capacity || new_capacity > SIZE_MAX - 1) {
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
        free(buffer);
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
        return r;
    }

    buffer[length] = '\0';

    size_t invalid_offset = 0;
    if (!opal_validate_utf8((const unsigned char*)buffer, length, &invalid_offset)) {
        char detail[32];
        snprintf(detail, sizeof(detail), "%zu", invalid_offset);
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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

    FILE* file = opal_fopen(path, "rb");
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
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);

    size_t length = 0;
    int saw_newline = 0;
    int first_char = fgetc(file);
    if (first_char == EOF) {
        if (ferror(file)) {
            int read_errno = errno ? errno : EIO;
            opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
            free(buffer);
            fclose(file);
            r.error = errno_to_fs_error(read_errno, OPAL_FS_ERR_IO);
            return r;
        }

        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
        free(buffer);
        fclose(file);
        r.error = errno_to_fs_error(read_errno, OPAL_FS_ERR_IO);
        return r;
    }

    if (fclose(file) != 0) {
        int close_errno = errno ? errno : EIO;
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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

    FILE* file = opal_fopen(path, "rb");
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
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);

    size_t length = 0;
    for (;;) {
        if (length == capacity) {
            size_t new_capacity = capacity <= (SIZE_MAX / 2) ? capacity * 2 : SIZE_MAX;
            if (new_capacity <= capacity || new_capacity > SIZE_MAX - 1) {
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
        free(buffer);
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
        return r;
    }

    buffer[length] = '\0';

    size_t invalid_offset = 0;
    if (!opal_validate_utf8((const unsigned char*)buffer, length, &invalid_offset)) {
        char detail[32];
        snprintf(detail, sizeof(detail), "%zu", invalid_offset);
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
        free(buffer);
        r.error = opal_fs_format_err(OPAL_FS_ERR_INVALID_UTF8, detail);
        if (!r.error) {
            r.error = safe_strdup("InvalidUtf8Error: failed to allocate error message");
        }
        return r;
    }

    char* normalized = (char*)malloc(length + 1);
    if (!normalized) {
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
    free(buffer);
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);

    if (normalized_len == 0) {
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
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
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
        free(normalized);
        return r;
    }

    if (line_count > (size_t)INT64_MAX) {
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
        free(normalized);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "too many lines");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: too many lines");
        }
        return r;
    }

    char** lines = (char**)calloc(line_count, sizeof(char*));
    if (!lines) {
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
        free(normalized);
        r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
        if (!r.error) {
            r.error = safe_strdup("ReadFailureError: out of memory");
        }
        return r;
    }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);

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
            opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);
            free(lines);
            opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
            free(normalized);
            r.error = opal_fs_format_err(OPAL_FS_ERR_IO, "out of memory");
            if (!r.error) {
                r.error = safe_strdup("ReadFailureError: out of memory");
            }
            return r;
        }
        opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_STRINGS);

        if (line_len > 0) {
            memcpy(line, normalized + start, line_len);
        }
        line[line_len] = '\0';

        lines[line_index++] = line;
        start = i + 1;
    }

    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_STRINGS);
    free(normalized);
    r.value = lines;
    r.count = (int64_t)line_index;
    return r;
}

FsBytesResult read_bytes_at_offset_sync(const char* path, int64_t offset, int64_t length) {
    FsBytesResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = fs_invalid_path_error();
        return r;
    }
    if (offset < 0) {
        r.error = fs_offset_out_of_range_error("offset must be non-negative");
        return r;
    }
    if (length < 0) {
        r.error = fs_offset_out_of_range_error("length must be non-negative");
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
    if (offset > stat_result.size) {
        r.error = fs_offset_out_of_range_error("offset out of range");
        return r;
    }
    if (length > stat_result.size - offset) {
        r.error = fs_offset_out_of_range_error("requested range exceeds file length");
        return r;
    }

    FILE* file = opal_fopen(path, "rb");
    if (!file) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    if (opal_seek_file(file, offset) != 0) {
        int seek_errno = errno ? errno : EIO;
        fclose(file);
        if (seek_errno == EINVAL) {
            r.error = fs_offset_out_of_range_error("offset out of range");
        } else {
            r.error = errno_to_fs_error(seek_errno, OPAL_FS_ERR_IO);
        }
        return r;
    }

    OpalBytes* bytes = (OpalBytes*)malloc(sizeof(OpalBytes));
    if (!bytes) {
        fclose(file);
        r.error = fs_out_of_memory_error(OPAL_FS_ERR_IO, "ReadFailureError: out of memory");
        return r;
    }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_BYTES);

    bytes->length = (size_t)length;
    bytes->data = NULL;
    if (length > 0) {
        bytes->data = (uint8_t*)malloc((size_t)length);
        if (!bytes->data) {
            opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_BYTES);
            free(bytes);
            fclose(file);
            r.error = fs_out_of_memory_error(OPAL_FS_ERR_IO, "ReadFailureError: out of memory");
            return r;
        }

        size_t total_read = 0;
        while (total_read < (size_t)length) {
            size_t read_now = fread(bytes->data + total_read, 1, (size_t)length - total_read, file);
            if (read_now == 0) {
                if (ferror(file)) {
                    int read_errno = errno ? errno : EIO;
                    free(bytes->data);
                    opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_BYTES);
                    free(bytes);
                    fclose(file);
                    r.error = errno_to_fs_error(read_errno, OPAL_FS_ERR_IO);
                    return r;
                }
                free(bytes->data);
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_BYTES);
                free(bytes);
                fclose(file);
                r.error = fs_offset_out_of_range_error("requested range exceeds file length");
                return r;
            }
            total_read += read_now;
        }
    }

    if (fclose(file) != 0) {
        int close_errno = errno ? errno : EIO;
        free(bytes->data);
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_BYTES);
        free(bytes);
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
        return r;
    }

    r.value = bytes;
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

    FILE* file = opal_fopen(path, "wb");
    if (!file) {
        r.error = errno_to_fs_error(errno, "Io");
        return r;
    }

    char* write_error = fwrite_all(file, write_bytes, write_len, "Io");
    if (write_error) {
        r.error = write_error;
        fclose(file);
        return r;
    }

    if (fclose(file) != 0) {
        r.error = fs_close_error("Io", "Io: close failed");
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

    FILE* file = opal_fopen(path, "wb");
    if (!file) {
        r.error = errno_to_fs_error(errno, "Io");
        return r;
    }

    char* write_error = fwrite_all(file, (const uint8_t*)write_text, write_len, "Io");
    if (write_error) {
        r.error = write_error;
        fclose(file);
        return r;
    }

    if (fclose(file) != 0) {
        r.error = fs_close_error("Io", "Io: close failed");
        return r;
    }

    return r;
}

FsVoidResult write_contents_atomic_sync(const char* path, OpalBytes* data) {
    const uint8_t* write_bytes = NULL;
    size_t write_len = 0;
    if (data) {
        write_bytes = data->data;
        write_len = data->length;
    }
    return write_contents_atomic_bytes_sync(path, write_bytes, write_len);
}

FsVoidResult write_text_atomic_sync(const char* path, const char* text) {
    const char* write_text = text ? text : "";
    return write_contents_atomic_bytes_sync(path, (const uint8_t*)write_text, strlen(write_text));
}

FsVoidResult append_contents_sync(const char* path, OpalBytes* data) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = fs_invalid_path_error();
        return r;
    }

    const uint8_t* write_bytes = NULL;
    size_t write_len = 0;
    if (data) {
        write_bytes = data->data;
        write_len = data->length;
    }

    fs_write_via_mode(path, write_bytes, write_len, "ab", "WriteFailureError", &r);
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

    FILE* file = opal_fopen(path, "ab");
    if (!file) {
        r.error = errno_to_fs_error(errno, "Io");
        return r;
    }

    char* write_error = fwrite_all(file, (const uint8_t*)write_text, write_len, "WriteFailureError");
    if (write_error) {
        r.error = write_error;
        fclose(file);
        return r;
    }

    if (fclose(file) != 0) {
        r.error = fs_close_error("WriteFailureError", "WriteFailureError: close failed");
        return r;
    }

    return r;
}

FsVoidResult write_bytes_at_offset_sync(const char* path, int64_t offset, OpalBytes* data) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = fs_invalid_path_error();
        return r;
    }
    if (offset < 0) {
        r.error = fs_offset_out_of_range_error("offset must be non-negative");
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat(path, &stat_result) != 0) {
        r.error = errno_to_fs_error(errno, "WriteFailureError");
        return r;
    }
    if (stat_result.is_directory) {
        r.error = opal_fs_format_err(OPAL_FS_ERR_IS_DIRECTORY, "path is a directory");
        if (!r.error) {
            r.error = safe_strdup("IsADirectoryError: path is a directory");
        }
        return r;
    }
    if (offset > stat_result.size) {
        r.error = fs_offset_out_of_range_error("offset out of range");
        return r;
    }

    FILE* file = opal_fopen(path, "r+b");
    if (!file) {
        r.error = errno_to_fs_error(errno, "WriteFailureError");
        return r;
    }

    if (opal_seek_file(file, offset) != 0) {
        int seek_errno = errno ? errno : EIO;
        fclose(file);
        if (seek_errno == EINVAL) {
            r.error = fs_offset_out_of_range_error("offset out of range");
        } else {
            r.error = errno_to_fs_error(seek_errno, "WriteFailureError");
        }
        return r;
    }

    const uint8_t* write_bytes = NULL;
    size_t write_len = 0;
    if (data) {
        write_bytes = data->data;
        write_len = data->length;
    }

    char* write_error = fwrite_all(file, write_bytes, write_len, "WriteFailureError");
    if (write_error) {
        r.error = write_error;
        fclose(file);
        return r;
    }

    if (fclose(file) != 0) {
        r.error = fs_close_error("WriteFailureError", "WriteFailureError: close failed");
        return r;
    }

    return r;
}

FsVoidResult create_file_sync(const char* path) {
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = fs_invalid_path_error();
        return r;
    }

    if (opal_create_file_exclusive(path) != 0) {
        int create_errno = errno;
        if (create_errno == EEXIST || create_errno == EISDIR) {
            r.error = opal_fs_format_err(OPAL_FS_ERR_ALREADY_EXISTS, "file exists");
            if (!r.error) {
                r.error = safe_strdup("FileAlreadyExistsError: file exists");
            }
        } else {
            r.error = errno_to_fs_error(create_errno, "CreateFailureError");
        }
        return r;
    }

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

    int same_file = 0;
    if (opal_paths_refer_to_same_file(source, destination, &same_file) == 0 && same_file) {
        return r;
    }

    FILE* source_file = opal_fopen(source, "rb");
    if (!source_file) {
        r.error = errno_to_fs_error(errno, OPAL_FS_ERR_IO);
        return r;
    }

    FILE* destination_file = opal_fopen(destination, "wb");
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

    if (opal_replace_path(source, destination) != 0) {
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
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_METADATA_PERMISSIONS);

    metadata->size_bytes = stat_result.size;
    metadata->modified_unix_seconds = stat_result.modified_time;
    metadata->is_directory = stat_result.is_directory ? 1 : 0;
    metadata->is_symlink = 0;

    r.value = metadata;
    return r;
}

FsMetadataResult read_metadata_nofollow_sync(const char* path) {
    FsMetadataResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = fs_invalid_path_error();
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat_nofollow(path, &stat_result) != 0) {
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
        r.error = fs_out_of_memory_error(OPAL_FS_ERR_METADATA_UNAVAILABLE, "MetadataUnavailableError: out of memory");
        return r;
    }
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_METADATA_PERMISSIONS);

    metadata->size_bytes = stat_result.size;
    metadata->modified_unix_seconds = stat_result.modified_time;
    metadata->is_directory = stat_result.is_directory ? 1 : 0;
    metadata->is_symlink = stat_result.is_symlink ? 1 : 0;

    r.value = metadata;
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
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = fs_invalid_path_error();
        return r;
    }

    char* normalized = lex_normalize_path(path);
    if (!normalized) {
        r.error = fs_out_of_memory_error("CreateFailureError", "CreateFailureError: out of memory");
        return r;
    }
    if (normalized[0] == '\0') {
        free(normalized);
        r.error = opal_fs_format_err("CreateFailureError", "normalized path is empty");
        if (!r.error) {
            r.error = safe_strdup("CreateFailureError: normalized path is empty");
        }
        return r;
    }

    size_t len = strlen(normalized);
    char* current = (char*)malloc(len + 2);
    if (!current) {
        free(normalized);
        r.error = fs_out_of_memory_error("CreateFailureError", "CreateFailureError: out of memory");
        return r;
    }

    size_t current_len = 0;
    if (normalized[0] == opal_path_separator()) {
        current[current_len++] = opal_path_separator();
        current[current_len] = '\0';
    }

    size_t index = (normalized[0] == opal_path_separator()) ? 1u : 0u;
    while (index <= len) {
        size_t start = index;
        while (index < len && !opal_is_path_separator(normalized[index])) {
            index++;
        }
        size_t segment_len = index - start;
        if (segment_len > 0) {
            if (current_len > 0 && !opal_is_path_separator(current[current_len - 1])) {
                current[current_len++] = opal_path_separator();
            }
            memcpy(current + current_len, normalized + start, segment_len);
            current_len += segment_len;
            current[current_len] = '\0';

            if (opal_mkdir(current) != 0) {
                int mkdir_errno = errno;
                if (mkdir_errno != EEXIST) {
                    free(current);
                    free(normalized);
                    r.error = errno_to_fs_error(mkdir_errno, "CreateFailureError");
                    return r;
                }

                struct opal_stat_result existing_stat;
                if (opal_stat(current, &existing_stat) != 0 || !existing_stat.is_directory) {
                    free(current);
                    free(normalized);
                    if (errno == ENOENT) {
                        r.error = errno_to_fs_error(EEXIST, "CreateFailureError");
                    } else {
                        r.error = errno_to_fs_error(errno ? errno : ENOTDIR, "CreateFailureError");
                    }
                    return r;
                }
            }
        }

        index++;
    }

    free(current);
    free(normalized);
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
    FsVoidResult r;
    r.value = NULL;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = fs_invalid_path_error();
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat_nofollow(path, &stat_result) != 0) {
        int stat_errno = errno;
        if (stat_errno == ENOENT) {
            r.error = opal_fs_format_err(OPAL_FS_ERR_DIRECTORY_NOT_FOUND, "directory not found");
            if (!r.error) {
                r.error = safe_strdup("DirectoryNotFoundError: directory not found");
            }
        } else {
            r.error = errno_to_fs_error(stat_errno, "DeleteFailureError");
        }
        return r;
    }

    if (!stat_result.is_directory || stat_result.is_symlink) {
        r.error = opal_fs_format_err(OPAL_FS_ERR_NOT_A_DIRECTORY, "path is not a directory");
        if (!r.error) {
            r.error = safe_strdup("IsNotADirectoryError: path is not a directory");
        }
        return r;
    }

    remove_directory_recursive_inner(path, &r);
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
    opal_rc_debug_note_alloc(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);

    for (;;) {
        errno = 0;
        opal_dirent_t* entry = opal_readdir(dir);
        if (!entry) {
            if (errno != 0) {
                int read_errno = errno;
                free_string_array_elements(entries, count);
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);
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
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);
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
                opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);
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
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);
        free(entries);
        r.error = errno_to_fs_error(close_errno, OPAL_FS_ERR_IO);
        return r;
    }

    if (count > 1) {
        qsort(entries, count, sizeof(char*), compare_string_pointers);
    }

    if (count > (size_t)INT64_MAX) {
        free_string_array_elements(entries, count);
        opal_rc_debug_note_free(OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS);
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
    FsBooleanResult r;
    r.value = 0;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = fs_invalid_path_error();
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat_nofollow(path, &stat_result) != 0) {
        int stat_errno = errno;
        if (stat_errno == ENOENT) {
            r.value = 0;
            return r;
        }

        r.error = errno_to_fs_error(stat_errno, OPAL_FS_ERR_IO);
        return r;
    }

    r.value = (stat_result.is_directory || stat_result.is_symlink) ? 0 : 1;
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
    FsBooleanResult r;
    r.value = 0;
    r.error = NULL;

    if (!path || path[0] == '\0') {
        r.error = fs_invalid_path_error();
        return r;
    }

    struct opal_stat_result stat_result;
    if (opal_stat_nofollow(path, &stat_result) != 0) {
        int stat_errno = errno;
        if (stat_errno == ENOENT) {
            r.value = 0;
            return r;
        }

        r.error = errno_to_fs_error(stat_errno, OPAL_FS_ERR_IO);
        return r;
    }

    r.value = (stat_result.is_directory && !stat_result.is_symlink) ? 1 : 0;
    return r;
}
