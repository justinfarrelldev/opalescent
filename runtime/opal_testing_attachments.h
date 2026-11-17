#ifndef OPAL_TESTING_ATTACHMENTS_H
#define OPAL_TESTING_ATTACHMENTS_H
#include "opal_testing_runtime.h"
typedef struct { const char *name; const char *text; } OpalTestAttachment;
typedef void (*OpalTestAttachmentObserver)(const OpalTestAttachment *attachment, void *context);
void opal_test_set_attachment_observer(OpalTestAttachmentObserver observer, void *context);
/* Copies diagnostic values only. Never retains context/borrowed source addresses. */
uint64_t opal_test_attach_text(const void *context, const char *name, const char *text);
size_t opal_test_attachment_count(void);
const OpalTestAttachment *opal_test_attachment_at(size_t index);
void opal_test_attachments_end(void);
#endif
