/* Attempt-local owned diagnostics, not a registry of acquired user resources. */
#include "opal_testing_attachments.h"
#include <stdlib.h>
#include <string.h>
typedef struct AttachmentNode {
    OpalTestAttachment value;
    struct AttachmentNode *next;
} AttachmentNode;
static AttachmentNode *head, *tail;
static size_t count;
static OpalTestAttachmentObserver observer;
static void *observer_context;
void opal_test_set_attachment_observer(OpalTestAttachmentObserver value, void *context) {observer = value; observer_context = context;}

uint64_t opal_test_attach_text(const void *context, const char *name, const char *text) {
    if (!context || !name || !*name || !text || !opal_test_attempt_number())
        return opal_test_record_failure(OPAL_TEST_FAILURE_INFRASTRUCTURE,"invalid diagnostic attachment", "", 0, 0);
    AttachmentNode *node = calloc(1, sizeof(*node));
    char *owned_name = malloc(strlen(name) + 1);
    char *owned_text = malloc(strlen(text) + 1);
    if (!node || !owned_name || !owned_text) {
        free(node); free(owned_name); free(owned_text);
        return opal_test_record_failure(OPAL_TEST_FAILURE_INFRASTRUCTURE,"could not allocate owned diagnostic attachment", "", 0, 0);
    }
    memcpy(owned_name, name, strlen(name) + 1);
    memcpy(owned_text, text, strlen(text) + 1);
    node->value.name = owned_name; node->value.text = owned_text;
    if (tail) tail->next = node; else head = node;
    tail = node; ++count;
    if (observer) observer(&node->value, observer_context);
    return 0;
}
size_t opal_test_attachment_count(void) {return count;}
const OpalTestAttachment *opal_test_attachment_at(size_t index) {
    AttachmentNode *node = head;
    while (node && index) {node = node->next; --index;}
    return node ? &node->value : NULL;
}
void opal_test_attachments_end(void) {
    while (head) {
        AttachmentNode *next = head->next;
        free((void *)head->value.name); free((void *)head->value.text); free(head); head = next;
    }
    tail = NULL; count = 0;
}
