/*
 * opal_portability.h — Cross-platform portability shims for the Opalescent runtime.
 *
 * This is the SINGLE SOURCE OF TRUTH for platform detection macros.
 * All runtime .c files MUST include this header FIRST.
 * Raw platform macros (_WIN32, _MSC_VER, __MINGW32__) are ONLY permitted inside this file.
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

/* ── Static assert ──────────────────────────────────────────────────────── */

#if OPAL_MSVC
#  define OPAL_STATIC_ASSERT(cond, msg) static_assert((cond), msg)
#elif defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
#  define OPAL_STATIC_ASSERT(cond, msg) _Static_assert((cond), msg)
#else
#  define OPAL_STATIC_ASSERT(cond, msg) \
     typedef char opal_static_assert_##__LINE__[(cond) ? 1 : -1]
#endif

/* ── ssize_t ────────────────────────────────────────────────────────────── */

#if OPAL_MSVC
#  include <stddef.h>
#  include <stdint.h>
   typedef intptr_t ssize_t;
#else
#  include <sys/types.h>
#endif

/* ── PRId64 / PRIu64 fallback ───────────────────────────────────────────── */

#include <inttypes.h>
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
#  include <string.h>
#  define opal_strdup strdup
#endif

/* ── getline shim ───────────────────────────────────────────────────────── */

#if OPAL_MSVC
#  include <stdio.h>
#  include <stdlib.h>
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
#  include <stdio.h>
#  define opal_getline getline
#endif

/* ── UTF-8 ↔ UTF-16 conversion (Windows only) ──────────────────────────── */

#if OPAL_WINDOWS
#  include <windows.h>

typedef wchar_t opal_wchar_t;

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

#define OPAL_HAS_DIRENT 0

#else /* POSIX branch */

typedef char opal_wchar_t;

#define OPAL_HAS_DIRENT 1

#endif /* OPAL_WINDOWS */

#endif /* OPAL_PORTABILITY_H */
