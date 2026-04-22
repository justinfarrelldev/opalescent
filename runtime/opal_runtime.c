/*
 * opal_runtime.c - Aggregator for the Opalescent C runtime.
 *
 * This file serves as a thin aggregator that includes all runtime headers.
 * Each .c file in the runtime compiles independently and is linked together
 * by the Rust build system (see src/compiler.rs).
 *
 * The Rust side concatenates all .c files into a single temporary file for
 * compilation, so this aggregator is primarily for documentation and to
 * ensure the runtime can be compiled as a single unit if needed.
 */

#include "opal_portability.h"
#include "opal_runtime.h"
#include "opal_rc.h"

#if OPAL_WINDOWS
#  include <windows.h>
#endif

void opal_runtime_init(void) {
#if OPAL_WINDOWS
    SetConsoleOutputCP(65001);  /* CP_UTF8 */
    SetConsoleCP(65001);
#endif
}
