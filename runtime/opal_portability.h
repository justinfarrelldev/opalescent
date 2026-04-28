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

#if OPAL_WINDOWS
#  include <limits.h>
#  define OPAL_PATH_BUFFER_CAP ((size_t)260)
#else
#  define OPAL_PATH_BUFFER_CAP ((size_t)4096)
#endif

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

typedef struct opal_dir_s opal_dir_t;
typedef struct {
    char d_name[260];
} opal_dirent_t;
#define OPAL_DIRENT_T_DEFINED 1

struct opal_dir_s {
    HANDLE handle;
    WIN32_FIND_DATAA find_data;
    int has_first;
    opal_dirent_t current;
};

static inline opal_dir_t* opal_opendir(const char* path) {
    if (!path || path[0] == '\0') {
        errno = EINVAL;
        return NULL;
    }

    char pattern[OPAL_PATH_BUFFER_CAP];
    size_t path_len = strlen(path);
    if (path_len + 3 > sizeof(pattern)) {
        errno = ENAMETOOLONG;
        return NULL;
    }

    memcpy(pattern, path, path_len + 1);
    if (path_len > 0 && pattern[path_len - 1] != '/' && pattern[path_len - 1] != '\\') {
        pattern[path_len++] = '\\';
    }
    pattern[path_len++] = '*';
    pattern[path_len] = '\0';

    opal_dir_t* dir = (opal_dir_t*)malloc(sizeof(*dir));
    if (!dir) {
        errno = ENOMEM;
        return NULL;
    }

    dir->handle = FindFirstFileA(pattern, &dir->find_data);
    if (dir->handle == INVALID_HANDLE_VALUE) {
        free(dir);
        return NULL;
    }

    dir->has_first = 1;
    dir->current.d_name[0] = '\0';
    return dir;
}

static inline opal_dirent_t* opal_readdir(opal_dir_t* dir) {
    if (!dir) {
        errno = EINVAL;
        return NULL;
    }

    for (;;) {
        WIN32_FIND_DATAA* src = NULL;
        if (dir->has_first) {
            dir->has_first = 0;
            src = &dir->find_data;
        } else {
            if (!FindNextFileA(dir->handle, &dir->find_data)) {
                return NULL;
            }
            src = &dir->find_data;
        }

        if (strcmp(src->cFileName, ".") == 0 || strcmp(src->cFileName, "..") == 0) {
            continue;
        }

        strncpy(dir->current.d_name, src->cFileName, sizeof(dir->current.d_name) - 1);
        dir->current.d_name[sizeof(dir->current.d_name) - 1] = '\0';
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
        result = FindClose(dir->handle) ? 0 : -1;
    }
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
 * opal_realpath
 *
 * Normalized cross-platform behavior:
 * - Windows: uses _fullpath(), which can resolve lexical absolute paths even
 *   for non-existent filesystem entries.
 * - POSIX: first attempts realpath(); if that fails with ENOENT, falls back to
 *   lexical absolute resolution (cwd + input path with '.'/'..' collapsed and
 *   separators normalized) so behavior matches Windows for non-existent paths.
 */
static inline char* opal_realpath(const char* path, char* resolved_buf, size_t buf_size) {
    if (!path || !resolved_buf || buf_size == 0) {
        errno = EINVAL;
        return NULL;
    }

#if OPAL_WINDOWS
    return _fullpath(resolved_buf, path, buf_size);
#else
    char* resolved = realpath(path, resolved_buf);
    if (resolved) {
        return resolved;
    }

    if (errno != ENOENT) {
        return NULL;
    }

    char absolute[OPAL_PATH_BUFFER_CAP];
    const char* source = path;

    if (path[0] == '/') {
        size_t path_len = strlen(path);
        if (path_len + 1 > sizeof(absolute)) {
            errno = ENAMETOOLONG;
            return NULL;
        }
        memcpy(absolute, path, path_len + 1);
    } else {
        char cwd[OPAL_PATH_BUFFER_CAP];
        if (!getcwd(cwd, sizeof(cwd))) {
            return NULL;
        }

        int wrote = snprintf(absolute, sizeof(absolute), "%s/%s", cwd, path);
        if (wrote < 0 || (size_t)wrote >= sizeof(absolute)) {
            errno = ENAMETOOLONG;
            return NULL;
        }
    }

    source = absolute;
    size_t out_len = 0;
    resolved_buf[out_len++] = '/';
    resolved_buf[out_len] = '\0';

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
                while (out_len > 1 && resolved_buf[out_len - 1] != '/') {
                    out_len--;
                }
                resolved_buf[out_len] = '\0';
            }
            continue;
        }

        if (out_len > 1) {
            if (out_len + 1 >= buf_size) {
                errno = ENAMETOOLONG;
                return NULL;
            }
            resolved_buf[out_len++] = '/';
        }

        if (out_len + seg_len >= buf_size) {
            errno = ENAMETOOLONG;
            return NULL;
        }

        memcpy(resolved_buf + out_len, seg_start, seg_len);
        out_len += seg_len;
        resolved_buf[out_len] = '\0';
    }

    if (out_len == 0) {
        if (buf_size < 2) {
            errno = ENAMETOOLONG;
            return NULL;
        }
        resolved_buf[0] = '/';
        resolved_buf[1] = '\0';
    }

    return resolved_buf;
#endif
}

static inline int opal_stat(const char* path, struct opal_stat_result* out) {
    if (!path || !out) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    struct _stat64 st;
    if (_stat64(path, &st) != 0) {
        return -1;
    }
    out->is_directory = ((st.st_mode & _S_IFDIR) != 0) ? 1 : 0;
    out->is_symlink = 0;
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
    DWORD attrs = GetFileAttributesA(path);
    if (attrs == INVALID_FILE_ATTRIBUTES) {
        struct _stat64 st;
        if (_stat64(path, &st) != 0) {
            return -1;
        }
        out->is_directory = ((st.st_mode & _S_IFDIR) != 0) ? 1 : 0;
        out->is_symlink = 0;
        out->size = (int64_t)st.st_size;
        out->modified_time = (int64_t)st.st_mtime;
        return 0;
    }

    struct _stat64 st;
    if (_stat64(path, &st) != 0) {
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

static inline int opal_mkdir(const char* path) {
    if (!path) {
        errno = EINVAL;
        return -1;
    }

#if OPAL_WINDOWS
    return _mkdir(path);
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
    return _rmdir(path);
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
    return _unlink(path);
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
    if (MoveFileExA(source, destination, MOVEFILE_REPLACE_EXISTING) != 0) {
        return 0;
    }
    opal_set_errno_from_win32(GetLastError());
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
    int fd = _open(path, _O_CREAT | _O_EXCL | _O_WRONLY | _O_BINARY, _S_IREAD | _S_IWRITE);
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

        HANDLE handle = CreateFileA(
            path_buf,
            GENERIC_READ | GENERIC_WRITE,
            0,
            NULL,
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL,
            NULL
        );
        if (handle != INVALID_HANDLE_VALUE) {
            CloseHandle(handle);
            return 0;
        }

        DWORD error = GetLastError();
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

static inline char opal_path_separator(void) {
#if OPAL_WINDOWS
    return '\\';
#else
    return '/';
#endif
}

#endif /* OPAL_PORTABILITY_H */
