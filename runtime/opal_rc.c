/**
 * opal_rc.c — Perceus Reference Counting Runtime for Opalescent
 *
 * See opal_rc.h for the full memory layout and API documentation.
 *
 * Implementation notes:
 * - All RC objects are allocated as: malloc(sizeof(OpalRcHeader) + payload_size)
 *   The user-visible pointer is (header + 1), i.e., the payload starts
 *   immediately after the header.
 * - opal_rc_drop_iterative uses a dynamically-growing stack (initial capacity
 *   OPAL_DROP_STACK_INIT) to process children without recursion.
 * - Weak references hold a pointer to the OpalRcHeader. The header is freed
 *   only when both refcount == 0 AND weak_count == 0.
 */

#include "opal_rc.h"
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* Initial capacity of the iterative drop work-list stack */
#define OPAL_DROP_STACK_INIT 64

/* -------------------------------------------------------------------------
 * Internal helpers
 * ---------------------------------------------------------------------- */

/**
 * obj_to_header — get the OpalRcHeader* from a payload pointer.
 * The header immediately precedes the payload in memory.
 */
static OpalRcHeader *obj_to_header(void *obj) {
    return ((OpalRcHeader *)obj) - 1;
}

static size_t opal_array_normalize_align(size_t elem_align) {
    return elem_align == 0 ? (size_t)1 : elem_align;
}

static uintptr_t opal_array_align_address_up(uintptr_t value, size_t alignment) {
    size_t align = opal_array_normalize_align(alignment);
    uintptr_t remainder = value % (uintptr_t)align;
    if (remainder == 0) {
        return value;
    }
    return value + ((uintptr_t)align - remainder);
}

static int opal_size_add_overflow(size_t left, size_t right, size_t *out) {
    if (left > (SIZE_MAX - right)) {
        return 1;
    }
    *out = left + right;
    return 0;
}

static int opal_size_mul_overflow(size_t left, size_t right, size_t *out) {
    if (left != 0 && right > (SIZE_MAX / left)) {
        return 1;
    }
    *out = left * right;
    return 0;
}

static OpalArrayPayloadHeader *opal_array_header(void *array) {
    return (OpalArrayPayloadHeader *)array;
}

static const OpalArrayPayloadHeader *opal_array_header_const(const void *array) {
    return (const OpalArrayPayloadHeader *)array;
}

/* -------------------------------------------------------------------------
 * Public API
 * ---------------------------------------------------------------------- */

void *opal_rc_alloc(size_t payload_size,
                    void (*drop_children_fn)(void *, void ***, size_t *, size_t *)) {
    /* Allocate header + payload in one contiguous block */
    OpalRcHeader *header = (OpalRcHeader *)malloc(sizeof(OpalRcHeader) + payload_size);
    if (!header) return NULL;

    header->refcount = 1;
    header->weak_count = 0;
    header->drop_children_fn = drop_children_fn;

    /* Zero-initialize the payload */
    void *payload = (void *)(header + 1);
    memset(payload, 0, payload_size);

    return payload;
}

void opal_rc_reuse(void *obj,
                   void (*new_drop_fn)(void *, void ***, size_t *, size_t *),
                   size_t payload_size) {
    if (!obj) return;

    OpalRcHeader *header = obj_to_header(obj);
    header->refcount = 1;
    header->weak_count = 0;
    header->drop_children_fn = new_drop_fn;

    memset(obj, 0, payload_size);
}

void opal_rc_inc(void *obj) {
    if (!obj) return;
    OpalRcHeader *header = obj_to_header(obj);
    header->refcount++;
}

void opal_rc_dec(void *obj) {
    if (!obj) return;
    OpalRcHeader *header = obj_to_header(obj);
    if (header->refcount == 0) return; /* already dropped */
    header->refcount--;
    if (header->refcount == 0) {
        opal_rc_drop_iterative(obj);
    }
}

void opal_rc_drop_child(void *obj,
                        void ***stack,
                        size_t *stack_top,
                        size_t *stack_cap) {
    if (!obj) return;

    OpalRcHeader *header = obj_to_header(obj);
    if (header->refcount == 0) return;

    header->refcount--;
    if (header->refcount != 0) {
        return;
    }

    if (*stack_top == *stack_cap) {
        size_t new_cap = *stack_cap + OPAL_DROP_STACK_INIT;
        void **new_stack = (void **)realloc(*stack, new_cap * sizeof(void *));
        if (!new_stack) {
            return;
        }
        *stack = new_stack;
        *stack_cap = new_cap;
    }

    (*stack)[(*stack_top)++] = obj;
}

void opal_rc_drop_iterative(void *root_obj) {
    if (!root_obj) return;

    /* Work-list stack: stores payload pointers of objects to drop */
    size_t stack_cap = OPAL_DROP_STACK_INIT;
    size_t stack_top = 0;
    void **stack = (void **)malloc(stack_cap * sizeof(void *));
    if (!stack) {
        /* Allocation failure: best-effort free of root only */
        OpalRcHeader *h = obj_to_header(root_obj);
        if (h->weak_count == 0) {
            free(h);
        }
        return;
    }

    /* Push the root object */
    stack[stack_top++] = root_obj;

    while (stack_top > 0) {
        void *obj = stack[--stack_top];
        OpalRcHeader *header = obj_to_header(obj);

        if (header->drop_children_fn) {
            if (stack_top + OPAL_DROP_STACK_INIT > stack_cap) {
                size_t new_cap = stack_cap + OPAL_DROP_STACK_INIT;
                void **new_stack = (void **)realloc(stack, new_cap * sizeof(void *));
                if (new_stack) {
                    stack = new_stack;
                    stack_cap = new_cap;
                }
            }
            header->drop_children_fn(obj, &stack, &stack_top, &stack_cap);
        }

        if (header->weak_count == 0) {
            free(header);
        }
    }

    free(stack);
}

size_t opal_array_data_offset(const void *array, size_t elem_align) {
    uintptr_t payload_addr = (uintptr_t)array;
    uintptr_t payload_end = payload_addr + (uintptr_t)sizeof(OpalArrayPayloadHeader);
    uintptr_t aligned_data = opal_array_align_address_up(payload_end, elem_align);
    return (size_t)(aligned_data - payload_addr);
}

void *opal_array_alloc(size_t elem_size,
                       size_t elem_align,
                       size_t len,
                       size_t cap,
                       void (*drop_children_fn)(void *, void ***, size_t *, size_t *)) {
    size_t normalized_align = opal_array_normalize_align(elem_align);
    size_t max_padding = normalized_align - (size_t)1;
    size_t header_bytes = 0;
    size_t capacity = cap < len ? len : cap;
    size_t element_bytes = 0;
    size_t payload_size = 0;
    void *array = NULL;
    OpalArrayPayloadHeader *header = NULL;

    if (opal_size_add_overflow(sizeof(OpalArrayPayloadHeader), max_padding, &header_bytes)) {
        return NULL;
    }
    if (opal_size_mul_overflow(elem_size, capacity, &element_bytes)) {
        return NULL;
    }
    if (opal_size_add_overflow(header_bytes, element_bytes, &payload_size)) {
        return NULL;
    }

    array = opal_rc_alloc(payload_size, drop_children_fn);
    if (!array) {
        return NULL;
    }

    header = opal_array_header(array);
    header->len = len;
    header->cap = capacity;

    return array;
}

size_t opal_array_len(const void *array) {
    if (!array) return 0;
    return opal_array_header_const(array)->len;
}

size_t opal_array_cap(const void *array) {
    if (!array) return 0;
    return opal_array_header_const(array)->cap;
}

void opal_array_set_len(void *array, size_t len) {
    if (!array) return;
    opal_array_header(array)->len = len;
}

void opal_array_set_cap(void *array, size_t cap) {
    if (!array) return;
    opal_array_header(array)->cap = cap;
}

void *opal_array_data(void *array, size_t elem_align) {
    if (!array) return NULL;
    return (void *)((unsigned char *)array + opal_array_data_offset(array, elem_align));
}

const void *opal_array_data_const(const void *array, size_t elem_align) {
    if (!array) return NULL;
    return (const void *)((const unsigned char *)array + opal_array_data_offset(array, elem_align));
}

OpalWeakRef *opal_weak_alloc(void *strong_obj) {
    if (!strong_obj) return NULL;

    OpalWeakRef *weak = (OpalWeakRef *)malloc(sizeof(OpalWeakRef));
    if (!weak) return NULL;

    OpalRcHeader *header = obj_to_header(strong_obj);
    header->weak_count++;
    weak->header = header;

    return weak;
}

void *opal_weak_upgrade(OpalWeakRef *weak) {
    if (!weak || !weak->header) return NULL;

    if (weak->header->refcount > 0) {
        return (void *)(weak->header + 1);
    }

    return NULL;
}

void opal_weak_dec(OpalWeakRef *weak) {
    if (!weak) return;

    OpalRcHeader *header = weak->header;
    if (header) {
        if (header->weak_count > 0) {
            header->weak_count--;
        }
        if (header->refcount == 0 && header->weak_count == 0) {
            free(header);
        }
    }

    free(weak);
}
