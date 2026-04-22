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

#endif /* OPAL_PORTABILITY_H */
