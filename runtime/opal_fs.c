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
#include "opal_runtime.h"
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

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

char* path_from(const char* raw) {
    if (!raw) return strdup("");
    return strdup(raw);
}

char* join_path_components(const char* base, const char** components, int64_t count) {
    size_t len = strlen(base);
    for (int64_t i = 0; i < count; i++) {
        len += 1 + strlen(components[i]);
    }
    char* result = (char*)malloc(len + 1);
    if (!result) return strdup(base);
    strcpy(result, base);
    for (int64_t i = 0; i < count; i++) {
        strcat(result, "/");
        strcat(result, components[i]);
    }
    return result;
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
    if (!path) return strdup(".");
    return strdup(path);
}

FsPathResult absolute_path_sync(const char* path) {
    FsPathResult r;
    if (!path || path[0] == '\0') {
        r.value = NULL;
        r.error = "InvalidPathError: empty path";
        return r;
    }
    char* resolved = realpath(path, NULL);
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
    (void)path;
    FsBytesResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsStringResult read_text_sync(const char* path) {
    (void)path;
    FsStringResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsStringArrayResult read_lines_sync(const char* path) {
    (void)path;
    FsStringArrayResult r;
    r.value = NULL;
    r.count = 0;
    r.error = "not implemented";
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
    (void)path;
    (void)data;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsVoidResult write_text_sync(const char* path, const char* text) {
    (void)path;
    (void)text;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
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
    (void)path;
    (void)text;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
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
    (void)path;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsVoidResult copy_file_sync(const char* source, const char* destination) {
    (void)source;
    (void)destination;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsVoidResult move_path_sync(const char* source, const char* destination) {
    (void)source;
    (void)destination;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
    return r;
}

FsBooleanResult path_exists_sync(const char* path) {
    (void)path;
    FsBooleanResult r;
    r.value = 0;
    r.error = "not implemented";
    return r;
}

FsMetadataResult read_metadata_sync(const char* path) {
    (void)path;
    FsMetadataResult r;
    r.value = NULL;
    r.error = "not implemented";
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
    (void)path;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
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
    (void)path;
    FsVoidResult r;
    r.value = NULL;
    r.error = "not implemented";
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
    (void)path;
    FsPathArrayResult r;
    r.value = NULL;
    r.count = 0;
    r.error = "not implemented";
    return r;
}

FsBooleanResult is_file_sync(const char* path) {
    (void)path;
    FsBooleanResult r;
    r.value = 0;
    r.error = "not implemented";
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
    (void)path;
    FsBooleanResult r;
    r.value = 0;
    r.error = "not implemented";
    return r;
}

FsBooleanResult is_directory_nofollow_sync(const char* path) {
    (void)path;
    FsBooleanResult r;
    r.value = 0;
    r.error = "not implemented";
    return r;
}
