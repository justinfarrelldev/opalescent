#ifndef OPAL_TESTING_POLLING_H
#define OPAL_TESTING_POLLING_H
#include "opal_testing_runtime.h"
/* Zero means completed; nonzero is an independent infrastructure abort identity. */
uint64_t opal_test_poll_begin(double seconds, uint64_t *deadline);
uint64_t opal_test_poll_retry(uint64_t deadline, int *retry);
#endif
