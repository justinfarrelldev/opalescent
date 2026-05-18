/*
 * opal_portability.h — Cross-platform portability shims for the Opalescent runtime.
 *
 * Include-order contract:
 * - This is the SINGLE SOURCE OF TRUTH for platform detection and portability macros.
 * - All runtime .c files MUST include this header FIRST.
 * - Raw platform macros (_WIN32, _MSC_VER, __MINGW32__) are ONLY permitted inside this file.
 */
#ifndef OPAL_PORTABILITY_H
#define OPAL_PORTABILITY_H

/* ── POSIX feature test macro (must come before any system headers) ──────── */

#if !defined(_WIN32)
#  define _POSIX_C_SOURCE 200809L
#endif

/* ── Platform detection ─────────────────────────────────────────────────── */

#if defined(_WIN32)
#  define OPAL_WINDOWS 1
#else
#  define OPAL_WINDOWS 0
#endif

#if defined(_MSC_VER)
#  define OPAL_MSVC 1
#else
#  define OPAL_MSVC 0
#endif

#if defined(__MINGW32__) || defined(__MINGW64__)
#  define OPAL_MINGW 1
#else
#  define OPAL_MINGW 0
#endif

/* ── Export / path-size portability ─────────────────────────────────────── */

#if OPAL_WINDOWS && defined(OPAL_BUILD_DLL)
#  define OPAL_API __declspec(dllexport)
#else
#  define OPAL_API
#endif

#include <limits.h>
#define OPAL_PATH_BUFFER_CAP ((size_t)4096)

/* ── Static assert ──────────────────────────────────────────────────────── */

#if OPAL_MSVC
#  define OPAL_STATIC_ASSERT(cond, msg) static_assert((cond), msg)
#elif defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
#  define OPAL_STATIC_ASSERT(cond, msg) _Static_assert((cond), msg)
#else
#  define OPAL_STATIC_ASSERT(cond, msg) \
     typedef char opal_static_assert_##__LINE__[(cond) ? 1 : -1]
#endif

/* ── Common includes for portability shims ──────────────────────────────── */

#include <errno.h>
#include <inttypes.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>

/* ── ssize_t ────────────────────────────────────────────────────────────── */

#if OPAL_MSVC
   typedef intptr_t ssize_t;
#else
#  include <sys/types.h>
#endif

/* ── PRId64 / PRIu64 fallback ───────────────────────────────────────────── */

#ifndef PRId64
#  if OPAL_MSVC
#    define PRId64 "I64d"
#    define PRIu64 "I64u"
#  else
#    error "PRId64 not defined and compiler is not MSVC"
#  endif
#endif

/* ── Thread-local storage ───────────────────────────────────────────────── */

#if OPAL_MSVC
#  define OPAL_THREAD_LOCAL __declspec(thread)
#elif defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
#  define OPAL_THREAD_LOCAL _Thread_local
#else
#  define OPAL_THREAD_LOCAL __thread
#endif

/* ── strdup shim ────────────────────────────────────────────────────────── */

#if OPAL_MSVC
#  define opal_strdup _strdup
#else
#  define opal_strdup strdup
#endif

/* ── getline shim ───────────────────────────────────────────────────────── */

#if OPAL_MSVC
static inline ssize_t opal_getline(char **lineptr, size_t *n, FILE *stream) {
    size_t pos = 0;
    int c;
    if (!lineptr || !n || !stream) return -1;
    if (*lineptr == NULL || *n == 0) {
        *n = 128;
        *lineptr = (char*)malloc(*n);
        if (!*lineptr) return -1;
    }
    while ((c = fgetc(stream)) != EOF) {
        if (pos + 1 >= *n) {
            size_t new_n = *n * 2;
            char *new_ptr = (char*)realloc(*lineptr, new_n);
            if (!new_ptr) return -1;
            *lineptr = new_ptr;
            *n = new_n;
        }
        (*lineptr)[pos++] = (char)c;
        if (c == '\n') break;
    }
    if (pos == 0 && c == EOF) return -1;
    (*lineptr)[pos] = '\0';
    return (ssize_t)pos;
}
#else
#  define opal_getline getline
#endif

/* ── UTF-8 ↔ UTF-16 conversion (Windows only) ──────────────────────────── */

#if OPAL_WINDOWS
#  include <windows.h>
#  include <direct.h>
#  include <io.h>
#  include <share.h>
#  include <sys/stat.h>
#  include <wchar.h>
#else
#  include <dirent.h>
#  include <sys/stat.h>
#  include <sys/types.h>
#  include <time.h>
#  include <unistd.h>
#endif

#if OPAL_WINDOWS

typedef wchar_t opal_wchar_t;

static inline void opal_set_errno_from_win32(DWORD error) {
    switch (error) {
        case ERROR_FILE_NOT_FOUND:
        case ERROR_PATH_NOT_FOUND:
            errno = ENOENT;
            break;
        case ERROR_ACCESS_DENIED:
        case ERROR_SHARING_VIOLATION:
        case ERROR_LOCK_VIOLATION:
            errno = EACCES;
            break;
        case ERROR_FILE_EXISTS:
        case ERROR_ALREADY_EXISTS:
            errno = EEXIST;
            break;
        case ERROR_DIR_NOT_EMPTY:
            errno = ENOTEMPTY;
            break;
        case ERROR_DIRECTORY:
            errno = ENOTDIR;
            break;
        case ERROR_BUFFER_OVERFLOW:
        case ERROR_FILENAME_EXCED_RANGE:
            errno = ENAMETOOLONG;
            break;
        case ERROR_INVALID_NAME:
        case ERROR_INVALID_PARAMETER:
            errno = EINVAL;
            break;
        case ERROR_NOT_SAME_DEVICE:
            errno = EXDEV;
            break;
        default:
            errno = EIO;
            break;
    }
}

/*
 * opal_utf8_to_wide — Convert UTF-8 string to UTF-16 (wide char).
 *
 * Allocates a new wide-char buffer via malloc(). Caller owns the result
 * and must free() it. Returns NULL on conversion error (invalid UTF-8).
 * Uses CP_UTF8 with MB_ERR_INVALID_CHARS for strict validation.
 */
static inline wchar_t* opal_utf8_to_wide(const char* utf8) {
    if (!utf8) return NULL;

    /* First pass: determine required buffer size (including null terminator) */
    int wide_len = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, NULL, 0);
    if (wide_len <= 0) return NULL;

    /* Allocate buffer */
    wchar_t* wide = (wchar_t*)malloc((size_t)wide_len * sizeof(wchar_t));
    if (!wide) return NULL;

    /* Second pass: perform actual conversion */
    int result = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, wide, wide_len);
    if (result <= 0) {
        free(wide);
        return NULL;
    }

    return wide;
}

/*
 * opal_wide_to_utf8 — Convert UTF-16 (wide char) string to UTF-8.
 *
 * Allocates a new UTF-8 buffer via malloc(). Caller owns the result
 * and must free() it. Returns NULL on conversion error (invalid UTF-16).
 * Uses CP_UTF8 with WC_ERR_INVALID_CHARS for strict validation.
 */
static inline char* opal_wide_to_utf8(const wchar_t* wide) {
    if (!wide) return NULL;

    /* First pass: determine required buffer size (including null terminator) */
    int utf8_len = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, wide, -1, NULL, 0, NULL, NULL);
    if (utf8_len <= 0) return NULL;

    /* Allocate buffer */
    char* utf8 = (char*)malloc((size_t)utf8_len);
    if (!utf8) return NULL;

    /* Second pass: perform actual conversion */
    int result = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, wide, -1, utf8, utf8_len, NULL, NULL);
    if (result <= 0) {
        free(utf8);
        return NULL;
    }

    return utf8;
}

static inline wchar_t* opal_windows_apply_long_path_prefix(const wchar_t* wide_path) {
    if (!wide_path) {
        errno = EINVAL;
        return NULL;
    }

    if (wcsncmp(wide_path, L"\\\\?\\", 4) == 0) {
        size_t existing_len = wcslen(wide_path) + 1;
        wchar_t* copy = (wchar_t*)malloc(existing_len * sizeof(wchar_t));
        if (!copy) {
            errno = ENOMEM;
            return NULL;
        }
        memcpy(copy, wide_path, existing_len * sizeof(wchar_t));
        return copy;
    }

    if (wcsncmp(wide_path, L"\\\\", 2) == 0) {
        const wchar_t* unc_body = wide_path + 2;
        size_t unc_len = wcslen(unc_body);
        size_t total_len = 8 + unc_len + 1; /* \\?\UNC\ + body + nul */
        wchar_t* prefixed = (wchar_t*)malloc(total_len * sizeof(wchar_t));
        if (!prefixed) {
            errno = ENOMEM;
            return NULL;
        }
        memcpy(prefixed, L"\\\\?\\UNC\\", 8 * sizeof(wchar_t));
        memcpy(prefixed + 8, unc_body, (unc_len + 1) * sizeof(wchar_t));
        return prefixed;
    }

    size_t path_len = wcslen(wide_path);
    size_t total_len = 4 + path_len + 1; /* \\?\ + path + nul */
    wchar_t* prefixed = (wchar_t*)malloc(total_len * sizeof(wchar_t));
    if (!prefixed) {
        errno = ENOMEM;
        return NULL;
    }
    memcpy(prefixed, L"\\\\?\\", 4 * sizeof(wchar_t));
    memcpy(prefixed + 4, wide_path, (path_len + 1) * sizeof(wchar_t));
    return prefixed;
}

static inline wchar_t* opal_utf8_to_wide_path(const char* utf8) {
    if (!utf8) {
        errno = EINVAL;
        return NULL;
    }

    int wide_len = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, NULL, 0);
    if (wide_len <= 0) {
        errno = EINVAL;
        return NULL;
    }

    wchar_t* wide = (wchar_t*)malloc((size_t)wide_len * sizeof(wchar_t));
    if (!wide) {
        errno = ENOMEM;
        return NULL;
    }

    if (MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, wide, wide_len) <= 0) {
        free(wide);
        errno = EINVAL;
        return NULL;
    }

    DWORD absolute_len = GetFullPathNameW(wide, 0, NULL, NULL);
    if (absolute_len == 0) {
        DWORD error = GetLastError();
        free(wide);
        opal_set_errno_from_win32(error);
        return NULL;
    }

    wchar_t* absolute = (wchar_t*)malloc((size_t)absolute_len * sizeof(wchar_t));
    if (!absolute) {
        free(wide);
        errno = ENOMEM;
        return NULL;
    }

    DWORD absolute_result = GetFullPathNameW(wide, absolute_len, absolute, NULL);
    free(wide);
    if (absolute_result == 0) {
        DWORD error = GetLastError();
        free(absolute);
        opal_set_errno_from_win32(error);
        return NULL;
    }
    if (absolute_result >= absolute_len) {
        free(absolute);
        errno = ENAMETOOLONG;
        return NULL;
    }

    wchar_t* prefixed = opal_windows_apply_long_path_prefix(absolute);
    free(absolute);
    return prefixed;
}

static inline FILE* opal_fopen(const char* path, const char* mode) {
    if (!path || !mode) {
        errno = EINVAL;
        return NULL;
    }

    wchar_t* wide_path = opal_utf8_to_wide_path(path);
    if (!wide_path) {
        return NULL;
    }

    wchar_t* wide_mode = opal_utf8_to_wide(mode);
    if (!wide_mode) {
        free(wide_path);
        return NULL;
    }

    FILE* file = _wfopen(wide_path, wide_mode);
    free(wide_mode);
    free(wide_path);
    return file;
}

typedef struct opal_dir_s opal_dir_t;
typedef struct {
    char* d_name;
} opal_dirent_t;
#define OPAL_DIRENT_T_DEFINED 1

struct opal_dir_s {
    HANDLE handle;
    WIN32_FIND_DATAW find_data;
    int has_first;
    opal_dirent_t current;
};

static inline opal_dir_t* opal_opendir(const char* path) {
    if (!path || path[0] == '\0') {
        errno = EINVAL;
        return NULL;
    }

    size_t path_len = strlen(path);
    char* pattern = (char*)malloc(path_len + 3);
    if (!pattern) {
        errno = ENOMEM;
        return NULL;
    }

    memcpy(pattern, path, path_len + 1);
    if (path_len > 0 && pattern[path_len - 1] != '/' && pattern[path_len - 1] != '\\') {
        pattern[path_len++] = '\\';
    }
    pattern[path_len++] = '*';
    pattern[path_len] = '\0';

    wchar_t* wide_pattern = opal_utf8_to_wide_path(pattern);
    free(pattern);
    if (!wide_pattern) {
        return NULL;
    }

    opal_dir_t* dir = (opal_dir_t*)malloc(sizeof(*dir));
    if (!dir) {
        free(wide_pattern);
        errno = ENOMEM;
        return NULL;
    }

    dir->handle = FindFirstFileW(wide_pattern, &dir->find_data);
    free(wide_pattern);
    if (dir->handle == INVALID_HANDLE_VALUE) {
        DWORD error = GetLastError();
        free(dir);
        opal_set_errno_from_win32(error);
        return NULL;
    }

    dir->has_first = 1;
    dir->current.d_name = NULL;
    return dir;
}

static inline opal_dirent_t* opal_readdir(opal_dir_t* dir) {
    if (!dir) {
        errno = EINVAL;
        return NULL;
    }

    for (;;) {
        WIN32_FIND_DATAW* src = NULL;
        if (dir->has_first) {
            dir->has_first = 0;
            src = &dir->find_data;
        } else {
            if (!FindNextFileW(dir->handle, &dir->find_data)) {
                DWORD error = GetLastError();
                if (error == ERROR_NO_MORE_FILES) {
                    errno = 0;
                } else {
                    opal_set_errno_from_win32(error);
                }
                return NULL;
            }
            src = &dir->find_data;
        }

        if (wcscmp(src->cFileName, L".") == 0 || wcscmp(src->cFileName, L"..") == 0) {
            continue;
        }

        char* utf8_name = opal_wide_to_utf8(src->cFileName);
        if (!utf8_name) {
            errno = EINVAL;
            return NULL;
        }

        free(dir->current.d_name);
        dir->current.d_name = utf8_name;
        return &dir->current;
    }
}

static inline int opal_closedir(opal_dir_t* dir) {
    if (!dir) {
        errno = EINVAL;
        return -1;
    }

    int result = 0;
    if (dir->handle != INVALID_HANDLE_VALUE) {
        if (!FindClose(dir->handle)) {
            DWORD error = GetLastError();
            opal_set_errno_from_win32(error);
            result = -1;
        }
    }
    free(dir->current.d_name);
    free(dir);
    return result;
}

#define OPAL_HAS_DIRENT 0

#else /* POSIX branch */

#  include <dirent.h>
#  include <limits.h>
#  include <sys/stat.h>
#  include <unistd.h>

/* realpath prototype may be hidden under strict feature-test macros. */
extern char* realpath(const char* path, char* resolved_path);

typedef char opal_wchar_t;

typedef DIR opal_dir_t;
typedef struct dirent opal_dirent_t;

static inline FILE* opal_fopen(const char* path, const char* mode) {
    return fopen(path, mode);
}

static inline opal_dir_t* opal_opendir(const char* path) {
    return opendir(path);
}

static inline opal_dirent_t* opal_readdir(opal_dir_t* dir) {
    return readdir(dir);
}

static inline int opal_closedir(opal_dir_t* dir) {
    return closedir(dir);
}

#define OPAL_HAS_DIRENT 1

#endif /* OPAL_WINDOWS */

typedef struct opal_stat_result {
    int is_directory;
    int is_symlink;
    int64_t size;
    int64_t modified_time;
} opal_stat_result;

/*
 * opal_realpath_owned
 *
 * Normalized cross-platform behavior:
 * - Windows: uses GetFullPathNameW(), which can resolve lexical absolute paths even
 *   for non-existent filesystem entries.
 * - POSIX: first attempts realpath(); if that fails with ENOENT, falls back to
 *   lexical absolute resolution (cwd + input path with '.'/'..' collapsed and
 *   separators normalized) so behavior matches Windows for non-existent paths.
 *
 * Returns a heap-owned UTF-8 buffer that the caller must free().
 */
static inline char* opal_realpath_owned(const char* path) {
    if (!path) {
        errno = EINVAL;
        return NULL;
    }

#if OPAL_WINDOWS
    wchar_t* wide_path = opal_utf8_to_wide(path);
    if (!wide_path) {
        errno = EINVAL;
        return NULL;
    }

    DWORD required = GetFullPathNameW(wide_path, 0, NULL, NULL);
    if (required == 0) {
        DWORD error = GetLastError();
        free(wide_path);
        opal_set_errno_from_win32(error);
        return NULL;
    }

    wchar_t* wide_resolved = (wchar_t*)malloc((size_t)required * sizeof(wchar_t));
    if (!wide_resolved) {
        free(wide_path);
        errno = ENOMEM;
        return NULL;
    }

    DWORD result = GetFullPathNameW(wide_path, required, wide_resolved, NULL);
    free(wide_path);
    if (result == 0) {
        DWORD error = GetLastError();
        free(wide_resolved);
        opal_set_errno_from_win32(error);
        return NULL;
    }
    if (result >= required) {
        free(wide_resolved);
        errno = ENAMETOOLONG;
        return NULL;
    }

    char* utf8_resolved = opal_wide_to_utf8(wide_resolved);
    free(wide_resolved);
    if (!utf8_resolved) {
        errno = EINVAL;
        return NULL;
    }

    return utf8_resolved;
#else
    char* resolved = realpath(path, NULL);
    if (resolved) {
        return resolved;
    }

    if (errno != ENOENT) {
        return NULL;
    }

    char* absolute = NULL;
    const char* source = path;

    if (path[0] == '/') {
        absolute = opal_strdup(path);
        if (!absolute) {
            errno = ENOMEM;
            return NULL;
        }
    } else {
        char* cwd = getcwd(NULL, 0);
        if (!cwd) {
            return NULL;
        }

        size_t cwd_len = strlen(cwd);
        size_t path_len = strlen(path);
        absolute = (char*)malloc(cwd_len + 1 + path_len + 1);
        if (!absolute) {
            free(cwd);
            errno = ENOMEM;
            return NULL;
        }

        memcpy(absolute, cwd, cwd_len);
        absolute[cwd_len] = '/';
        memcpy(absolute + cwd_len + 1, path, path_len + 1);
        free(cwd);
    }

    source = absolute;
    size_t source_len = strlen(source);
    size_t output_cap = source_len + 2;
    char* normalized = (char*)malloc(output_cap);
    if (!normalized) {
        free(absolute);
        errno = ENOMEM;
        return NULL;
    }

    size_t out_len = 0;
    normalized[out_len++] = '/';
    normalized[out_len] = '\0';

    const char* p = source;
    while (*p) {
        while (*p == '/' || *p == '\\') {
            p++;
        }
        if (*p == '\0') {
            break;
        }

        const char* seg_start = p;
        while (*p != '\0' && *p != '/' && *p != '\\') {
            p++;
        }
        size_t seg_len = (size_t)(p - seg_start);

        if (seg_len == 1 && seg_start[0] == '.') {
            continue;
        }

        if (seg_len == 2 && seg_start[0] == '.' && seg_start[1] == '.') {
            if (out_len > 1) {
                out_len--;
                while (out_len > 1 && normalized[out_len - 1] != '/') {
                    out_len--;
                }
                normalized[out_len] = '\0';
            }
            continue;
        }

        if (out_len > 1) {
            normalized[out_len++] = '/';
        }

        memcpy(normalized + out_len, seg_start, seg_len);
        out_len += seg_len;
        normalized[out_len] = '\0';
    }

    if (out_len == 0) {
        normalized[0] = '/';
        normalized[1] = '\0';
    }

    free(absolute);
    return normalized;
#endif
}

static inline int opal_stat(const char* path, struct opal_stat_result* out) {
    if (!path || !out) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    wchar_t* wide_path = opal_utf8_to_wide_path(path);
    if (!wide_path) {
        return -1;
    }

    DWORD attrs = GetFileAttributesW(wide_path);
    DWORD attrs_error = (attrs == INVALID_FILE_ATTRIBUTES) ? GetLastError() : 0;
    struct _stat64 st;
    int result = _wstat64(wide_path, &st);
    free(wide_path);
    if (result != 0) {
        if (attrs == INVALID_FILE_ATTRIBUTES) {
            opal_set_errno_from_win32(attrs_error);
            return -1;
        }
        out->is_directory = (attrs & FILE_ATTRIBUTE_DIRECTORY) ? 1 : 0;
        out->is_symlink = (attrs & FILE_ATTRIBUTE_REPARSE_POINT) ? 1 : 0;
        out->size = 0;
        out->modified_time = 0;
        return 0;
    }
    out->is_directory = ((st.st_mode & _S_IFDIR) != 0) ? 1 : 0;
    out->is_symlink = (attrs != INVALID_FILE_ATTRIBUTES && (attrs & FILE_ATTRIBUTE_REPARSE_POINT)) ? 1 : 0;
    out->size = (int64_t)st.st_size;
    out->modified_time = (int64_t)st.st_mtime;
#else
    struct stat st;
    if (stat(path, &st) != 0) {
        return -1;
    }
    out->is_directory = S_ISDIR(st.st_mode) ? 1 : 0;
    out->is_symlink = S_ISLNK(st.st_mode) ? 1 : 0;
    out->size = (int64_t)st.st_size;
    out->modified_time = (int64_t)st.st_mtime;
#endif

    return 0;
}

static inline int opal_stat_nofollow(const char* path, struct opal_stat_result* out) {
    if (!path || !out) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    wchar_t* wide_path = opal_utf8_to_wide_path(path);
    if (!wide_path) {
        return -1;
    }

    DWORD attrs = GetFileAttributesW(wide_path);
    if (attrs == INVALID_FILE_ATTRIBUTES) {
        DWORD attrs_error = GetLastError();
        struct _stat64 st;
        int stat_result = _wstat64(wide_path, &st);
        free(wide_path);
        if (stat_result != 0) {
            opal_set_errno_from_win32(attrs_error);
            return -1;
        }
        out->is_directory = ((st.st_mode & _S_IFDIR) != 0) ? 1 : 0;
        out->is_symlink = 0;
        out->size = (int64_t)st.st_size;
        out->modified_time = (int64_t)st.st_mtime;
        return 0;
    }

    struct _stat64 st;
    int stat_result = _wstat64(wide_path, &st);
    free(wide_path);
    if (stat_result != 0) {
        if ((attrs & FILE_ATTRIBUTE_REPARSE_POINT) == 0) {
            return -1;
        }
        memset(&st, 0, sizeof(st));
    }

    out->is_directory = (attrs & FILE_ATTRIBUTE_DIRECTORY) ? 1 : 0;
    out->is_symlink = (attrs & FILE_ATTRIBUTE_REPARSE_POINT) ? 1 : 0;
    out->size = (int64_t)st.st_size;
    out->modified_time = (int64_t)st.st_mtime;
#else
    struct stat st;
    if (lstat(path, &st) != 0) {
        return -1;
    }
    out->is_directory = S_ISDIR(st.st_mode) ? 1 : 0;
    out->is_symlink = S_ISLNK(st.st_mode) ? 1 : 0;
    out->size = (int64_t)st.st_size;
    out->modified_time = (int64_t)st.st_mtime;
#endif

    return 0;
}

static inline int opal_paths_refer_to_same_file(const char* first, const char* second, int* out_same) {
    if (!first || !second || !out_same) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    wchar_t* wide_first = opal_utf8_to_wide_path(first);
    if (!wide_first) {
        return -1;
    }

    wchar_t* wide_second = opal_utf8_to_wide_path(second);
    if (!wide_second) {
        free(wide_first);
        return -1;
    }

    HANDLE first_handle = CreateFileW(
        wide_first,
        FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        NULL,
        OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS,
        NULL
    );
    if (first_handle == INVALID_HANDLE_VALUE) {
        DWORD error = GetLastError();
        free(wide_second);
        free(wide_first);
        opal_set_errno_from_win32(error);
        return -1;
    }

    HANDLE second_handle = CreateFileW(
        wide_second,
        FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        NULL,
        OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS,
        NULL
    );
    free(wide_second);
    free(wide_first);
    if (second_handle == INVALID_HANDLE_VALUE) {
        DWORD error = GetLastError();
        CloseHandle(first_handle);
        opal_set_errno_from_win32(error);
        return -1;
    }

    BY_HANDLE_FILE_INFORMATION first_info;
    if (!GetFileInformationByHandle(first_handle, &first_info)) {
        DWORD error = GetLastError();
        CloseHandle(second_handle);
        CloseHandle(first_handle);
        opal_set_errno_from_win32(error);
        return -1;
    }

    BY_HANDLE_FILE_INFORMATION second_info;
    if (!GetFileInformationByHandle(second_handle, &second_info)) {
        DWORD error = GetLastError();
        CloseHandle(second_handle);
        CloseHandle(first_handle);
        opal_set_errno_from_win32(error);
        return -1;
    }

    if (!CloseHandle(second_handle)) {
        DWORD error = GetLastError();
        CloseHandle(first_handle);
        opal_set_errno_from_win32(error);
        return -1;
    }
    if (!CloseHandle(first_handle)) {
        DWORD error = GetLastError();
        opal_set_errno_from_win32(error);
        return -1;
    }

    *out_same = (
        first_info.dwVolumeSerialNumber == second_info.dwVolumeSerialNumber &&
        first_info.nFileIndexHigh == second_info.nFileIndexHigh &&
        first_info.nFileIndexLow == second_info.nFileIndexLow
    ) ? 1 : 0;
    return 0;
#else
    struct stat first_stat;
    struct stat second_stat;
    if (stat(first, &first_stat) != 0 || stat(second, &second_stat) != 0) {
        return -1;
    }

    *out_same = (first_stat.st_dev == second_stat.st_dev && first_stat.st_ino == second_stat.st_ino) ? 1 : 0;
    return 0;
#endif
}

static inline int opal_mkdir(const char* path) {
    if (!path) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    wchar_t* wide_path = opal_utf8_to_wide_path(path);
    if (!wide_path) {
        return -1;
    }

    int result = _wmkdir(wide_path);
    free(wide_path);
    return result;
#else
    return mkdir(path, 0755);
#endif
}

static inline int opal_rmdir(const char* path) {
    if (!path) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    wchar_t* wide_path = opal_utf8_to_wide_path(path);
    if (!wide_path) {
        return -1;
    }

    int result = _wrmdir(wide_path);
    free(wide_path);
    return result;
#else
    return rmdir(path);
#endif
}

static inline int opal_unlink(const char* path) {
    if (!path) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    wchar_t* wide_path = opal_utf8_to_wide_path(path);
    if (!wide_path) {
        return -1;
    }

    int result = _wunlink(wide_path);
    free(wide_path);
    return result;
#else
    return unlink(path);
#endif
}

static inline int opal_seek_file(FILE* file, int64_t offset) {
    if (!file || offset < 0) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    return _fseeki64(file, offset, SEEK_SET);
#else
    return fseeko(file, (off_t)offset, SEEK_SET);
#endif
}

static inline int opal_replace_path(const char* source, const char* destination) {
    if (!source || !destination) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    wchar_t* wide_source = opal_utf8_to_wide_path(source);
    if (!wide_source) {
        return -1;
    }

    wchar_t* wide_destination = opal_utf8_to_wide_path(destination);
    if (!wide_destination) {
        free(wide_source);
        return -1;
    }

    if (MoveFileExW(wide_source, wide_destination, MOVEFILE_REPLACE_EXISTING) != 0) {
        free(wide_destination);
        free(wide_source);
        return 0;
    }

    DWORD error = GetLastError();
    free(wide_destination);
    free(wide_source);
    opal_set_errno_from_win32(error);
    return -1;
#else
    return rename(source, destination);
#endif
}

static inline int opal_create_file_exclusive(const char* path) {
    if (!path || path[0] == '\0') {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    wchar_t* wide_path = opal_utf8_to_wide_path(path);
    if (!wide_path) {
        return -1;
    }

    int fd = _wopen(wide_path, _O_CREAT | _O_EXCL | _O_WRONLY | _O_BINARY, _S_IREAD | _S_IWRITE);
    free(wide_path);
#else
    int fd = open(path, O_CREAT | O_EXCL | O_WRONLY, 0666);
#endif
    if (fd < 0) {
        return -1;
    }

#if OPAL_WINDOWS
    if (_close(fd) != 0) {
        return -1;
    }
#else
    if (close(fd) != 0) {
        return -1;
    }
#endif

    return 0;
}

static inline int opal_create_temp_file(char* path_buf, size_t path_buf_size) {
    if (!path_buf || path_buf_size == 0) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    const char alphabet[] = "0123456789abcdef";
    size_t original_len = strlen(path_buf);
    if (original_len + 1 > path_buf_size) {
        errno = ENAMETOOLONG;
        return -1;
    }

    char* placeholder = strstr(path_buf, "XXXXXX");
    if (!placeholder) {
        errno = EINVAL;
        return -1;
    }

    DWORD pid = GetCurrentProcessId();
    DWORD tick = GetTickCount();
    for (unsigned attempt = 0; attempt < 256; attempt++) {
        unsigned value = (unsigned)(pid ^ tick ^ (attempt * 2654435761u));
        for (size_t i = 0; i < 6; i++) {
            placeholder[i] = alphabet[value & 0x0Fu];
            value >>= 4;
        }

        wchar_t* wide_path = opal_utf8_to_wide_path(path_buf);
        if (!wide_path) {
            return -1;
        }

        HANDLE handle = CreateFileW(
            wide_path,
            GENERIC_READ | GENERIC_WRITE,
            0,
            NULL,
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL,
            NULL
        );
        if (handle != INVALID_HANDLE_VALUE) {
            free(wide_path);
            CloseHandle(handle);
            return 0;
        }

        DWORD error = GetLastError();
        free(wide_path);
        if (error != ERROR_FILE_EXISTS && error != ERROR_ALREADY_EXISTS) {
            opal_set_errno_from_win32(error);
            return -1;
        }
    }

    errno = EEXIST;
    return -1;
#else
    int fd = mkstemp(path_buf);
    if (fd < 0) {
        return -1;
    }
    if (close(fd) != 0) {
        return -1;
    }
    return 0;
#endif
}

static inline int opal_monotonic_time_ms(int64_t* out_milliseconds) {
    if (!out_milliseconds) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    static LARGE_INTEGER frequency;
    static int frequency_initialized = 0;
    LARGE_INTEGER counter;

    if (!frequency_initialized) {
        if (!QueryPerformanceFrequency(&frequency) || frequency.QuadPart <= 0) {
            errno = EINVAL;
            return -1;
        }
        frequency_initialized = 1;
    }

    if (!QueryPerformanceCounter(&counter)) {
        errno = EINVAL;
        return -1;
    }

    *out_milliseconds = (int64_t)((counter.QuadPart * 1000LL) / frequency.QuadPart);
    return 0;
#else
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) {
        return -1;
    }

    *out_milliseconds = (int64_t)ts.tv_sec * 1000LL + (int64_t)(ts.tv_nsec / 1000000L);
    return 0;
#endif
}

static inline int opal_sleep_ms(int32_t milliseconds) {
    if (milliseconds < 0) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    Sleep((DWORD)milliseconds);
    return 0;
#else
    struct timespec request;
    request.tv_sec = milliseconds / 1000;
    request.tv_nsec = (long)((milliseconds % 1000) * 1000000L);

    while (nanosleep(&request, &request) != 0) {
        if (errno != EINTR) {
            return -1;
        }
    }

    return 0;
#endif
}

static inline char opal_path_separator(void) {
#if OPAL_WINDOWS
    return '\\';
#else
    return '/';
#endif
}

#endif /* OPAL_PORTABILITY_H */
