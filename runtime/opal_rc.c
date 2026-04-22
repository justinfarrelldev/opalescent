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

#include "opal_portability.h"
#include <stddef.h>

/* RC object header — precedes every heap-allocated RC object in memory */
typedef struct OpalRcHeader {
    size_t refcount;
    size_t weak_count;
    void (*drop_children_fn)(void *obj, void **stack, size_t *stack_top, size_t *stack_cap);
} OpalRcHeader;

/* Weak reference — holds a pointer to the header (not the payload) */
typedef struct OpalWeakRef {
    OpalRcHeader *header;
} OpalWeakRef;

/* RC header field offsets (bytes from header start) — for codegen use */
#define OPAL_RC_REFCOUNT_OFFSET    0
#define OPAL_RC_WEAK_COUNT_OFFSET  8
#define OPAL_RC_DROP_FN_OFFSET     16
#define OPAL_RC_HEADER_SIZE        24

/* Forward declarations of all public functions */
void *opal_rc_alloc(size_t payload_size, void (*drop_children_fn)(void *, void **, size_t *, size_t *));
void opal_rc_reuse(void *obj, void (*new_drop_fn)(void *, void **, size_t *, size_t *), size_t payload_size);
void opal_rc_inc(void *obj);
void opal_rc_dec(void *obj);
void opal_rc_drop_iterative(void *obj);
OpalWeakRef *opal_weak_alloc(void *strong_obj);
void *opal_weak_upgrade(OpalWeakRef *weak);
void opal_weak_dec(OpalWeakRef *weak);
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

/* -------------------------------------------------------------------------
 * Public API
 * ---------------------------------------------------------------------- */

void *opal_rc_alloc(size_t payload_size,
                    void (*drop_children_fn)(void *, void **, size_t *, size_t *)) {
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
                   void (*new_drop_fn)(void *, void **, size_t *, size_t *),
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

        /* Enqueue children onto the work-list before freeing this object.
         * drop_children_fn is responsible for calling opal_rc_dec on each
         * child, which will push them onto the stack if their refcount hits 0.
         * However, since we're doing iterative drop, we use a different
         * approach: drop_children_fn pushes child payload pointers directly
         * onto our stack (without calling opal_rc_dec recursively). */
        if (header->drop_children_fn) {
            /* Ensure enough capacity for children before calling */
            /* We pass stack by pointer so drop_children_fn can grow it */
            /* But our stack is a local variable — we need to handle realloc.
             * Solution: pass &stack (void***) — but the fn signature uses void**.
             * Compromise: pre-grow the stack to a safe size, or use a wrapper. */
            /* For the iterative drop, drop_children_fn receives the stack array
             * pointer by value. If it needs to grow, it must realloc and update
             * stack_top/stack_cap. Since it can't update our local `stack` pointer,
             * we use a context struct approach via a thread-local or pass by ref.
             *
             * Practical solution: pass a pointer to our local stack variable
             * by casting. The fn signature is:
             *   void fn(void* obj, void** stack, size_t* top, size_t* cap)
             * where `stack` is the array itself (not a pointer to the array).
             * This means drop_children_fn cannot realloc the stack.
             *
             * To handle this safely: pre-grow the stack before calling fn.
             * We don't know how many children there are, so we grow by a fixed
             * amount (OPAL_DROP_STACK_INIT) as a heuristic. */
            if (stack_top + OPAL_DROP_STACK_INIT > stack_cap) {
                size_t new_cap = stack_cap + OPAL_DROP_STACK_INIT;
                void **new_stack = (void **)realloc(stack, new_cap * sizeof(void *));
                if (new_stack) {
                    stack = new_stack;
                    stack_cap = new_cap;
                }
                /* If realloc fails, proceed with current capacity — may miss some children */
            }
            header->drop_children_fn(obj, stack, &stack_top, &stack_cap);
        }

        /* Free the object: either free the whole block (header+payload) if no
         * weak refs remain, or just mark refcount=0 and leave header alive
         * for weak ref upgrade checks. */
        if (header->weak_count == 0) {
            free(header); /* frees header + payload together */
        }
        /* If weak_count > 0, the header stays alive until opal_weak_dec
         * brings weak_count to 0. The payload is logically dropped (refcount=0)
         * but the header memory persists for weak upgrade checks. */
    }

    free(stack);
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

    /* Object is alive if refcount > 0 */
    if (weak->header->refcount > 0) {
        /* Return payload pointer (header + 1) */
        return (void *)(weak->header + 1);
    }

    return NULL; /* Object has been dropped */
}

void opal_weak_dec(OpalWeakRef *weak) {
    if (!weak) return;

    OpalRcHeader *header = weak->header;
    if (header) {
        if (header->weak_count > 0) {
            header->weak_count--;
        }
        /* If both counts are 0, the header was kept alive for weak refs — free it now */
        if (header->refcount == 0 && header->weak_count == 0) {
            free(header);
        }
    }

    free(weak);
}
