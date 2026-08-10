#ifndef OPAL_TEST_ALLOC_H
#define OPAL_TEST_ALLOC_H

#if defined(OPAL_ENABLE_INTERNAL_TESTING)
#include "opal_rc.h"

#define malloc(size) opal_test_malloc_for_test(size)
#define calloc(count, size) opal_test_calloc_for_test(count, size)
#define realloc(ptr, size) opal_test_realloc_for_test(ptr, size)
#endif

#endif
