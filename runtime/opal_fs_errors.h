/*
 * opal_fs_errors.h - Single Source of Truth (SSOT) for filesystem error discriminants.
 *
 * All fs error strings emitted by runtime MUST start with one of these discriminants,
 * followed by ': ' and a detail string. Opalescent code matches on the prefix.
 *
 * Allocation contract (LOCKED):
 * - Error strings in Fs*Result.error are EITHER (a) NULL on success, OR (b) a
 *   heap-allocated char* produced by opal_fs_format_err or strdup.
 * - Static string literals are FORBIDDEN in T15+ replacements — they break the
 *   uniform-free contract on the consumer side.
 */

#ifndef OPAL_FS_ERRORS_H
#define OPAL_FS_ERRORS_H

#include <stdlib.h>
#include <stdio.h>
#include <string.h>

/* 14 filesystem error discriminants (SSOT) */
#define OPAL_FS_ERR_NOT_FOUND "FileNotFoundError"
#define OPAL_FS_ERR_PERMISSION_DENIED "PermissionDeniedError"
#define OPAL_FS_ERR_IS_DIRECTORY "IsADirectoryError"
#define OPAL_FS_ERR_NOT_A_DIRECTORY "IsNotADirectoryError"
#define OPAL_FS_ERR_INVALID_UTF8 "InvalidUtf8Error"
#define OPAL_FS_ERR_ALREADY_EXISTS "FileAlreadyExistsError"
#define OPAL_FS_ERR_INVALID_PATH "InvalidPathError"
/* OPAL_FS_ERR_OUT_OF_BOUNDS call-site alternates: "OffsetOutOfRangeError" or "LineOutOfRangeError" */
/* T18 lock: read_first_line_sync empty-file maps to OPAL_FS_ERR_OUT_OF_BOUNDS + detail "file is empty". */
#define OPAL_FS_ERR_OUT_OF_BOUNDS "OffsetOutOfRangeError"
/* OPAL_FS_ERR_IO call-site alternates: "ReadFailureError", "WriteFailureError", "CopyFailureError", "MoveFailureError", "DeleteFailureError", "CreateFailureError" */
#define OPAL_FS_ERR_IO "ReadFailureError"
#define OPAL_FS_ERR_FILESYSTEM_FULL "FilesystemFullError"
#define OPAL_FS_ERR_DIRECTORY_NOT_EMPTY "DirectoryNotEmptyError"
#define OPAL_FS_ERR_DIRECTORY_NOT_FOUND "DirectoryNotFoundError"
#define OPAL_FS_ERR_METADATA_UNAVAILABLE "MetadataUnavailableError"
#define OPAL_FS_ERR_SET_PERMISSIONS "SetPermissionsError"

/*
 * Helper function: allocate and format an error string.
 *
 * Returns a heap-allocated string "<prefix>: <detail>" via malloc.
 * Caller becomes owner; freed by Opal runtime's error consumer.
 *
 * Defensive: if prefix or detail is NULL, uses safe fallback text.
 */
static inline char* opal_fs_format_err(const char* prefix, const char* detail) {
    if (!prefix) prefix = "UnknownError";
    if (!detail) detail = "unknown details";
    
    size_t prefix_len = strlen(prefix);
    size_t detail_len = strlen(detail);
    size_t total_len = prefix_len + 2 + detail_len + 1; /* ": " + null terminator */
    
    char* result = (char*)malloc(total_len);
    if (!result) return NULL;
    
    snprintf(result, total_len, "%s: %s", prefix, detail);
    return result;
}

#endif /* OPAL_FS_ERRORS_H */
