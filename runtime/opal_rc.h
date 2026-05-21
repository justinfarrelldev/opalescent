/**
 * opal_rc.h — Perceus Reference Counting Runtime for Opalescent
 *
 * Memory Layout (ABI-stable for future module imports):
 *
 *   [ OpalRcHeader (24 bytes) | payload ... ]
 *   ^                          ^
 *   header ptr                 obj ptr (returned to user)
 *
 * Header fields:
 *   refcount         (size_t, offset 0)  — strong reference count
 *   weak_count       (size_t, offset 8)  — weak reference count
 *   drop_children_fn (fn ptr, offset 16) — enqueues child RC objects onto work-list
 *
 * The header immediately precedes the payload in memory. The user-visible
 * pointer points to the payload (header + 1). This keeps the ABI stable:
 * adding fields to the header does not change the payload pointer.
 *
 * Weak references:
 *   OpalWeakRef holds a pointer to the OpalRcHeader (not the payload).
 *   opal_weak_upgrade() returns the payload pointer if refcount > 0, else NULL.
 *   The header is freed only when BOTH refcount == 0 AND weak_count == 0.
 *
 * Iterative drop:
 *   opal_rc_drop_iterative() uses a work-list (stack) to avoid recursion.
 *   Each RC type provides a drop_children_fn that enqueues its child RC
 *   objects onto the work-list. The outer loop processes them iteratively.
 *   This prevents stack overflow on deeply nested data structures.
 */

#ifndef OPAL_RC_H
#define OPAL_RC_H

#include <stddef.h>
#include "opal_portability.h"

/* RC object header — precedes every heap-allocated RC object in memory */
typedef struct OpalRcHeader {
    size_t refcount;
    size_t weak_count;
    /**
     * drop_children_fn — called during iterative drop to enqueue child RC
     * objects onto the work-list. Signature:
     *   obj:       pointer to the payload (header + 1)
     *   stack:     pointer to the work-list stack array pointer (may be reallocated)
     *   stack_top: current top index (in/out)
     *   stack_cap: current capacity (in/out)
     * The function should push each child RC payload pointer onto the stack.
     * Pass NULL if the object has no RC children.
     */
    void (*drop_children_fn)(void *obj, void ***stack, size_t *stack_top, size_t *stack_cap);
} OpalRcHeader;

/* RC-backed array payload header — lives inside the payload region */
typedef struct OpalArrayPayloadHeader {
    size_t len;
    size_t cap;
} OpalArrayPayloadHeader;

/* Weak reference — holds a pointer to the header (not the payload) */
typedef struct OpalWeakRef {
    OpalRcHeader *header;
} OpalWeakRef;

typedef enum OpalRcDebugCounterKind {
    OPAL_RC_DEBUG_COUNTER_NONE = 0,
    OPAL_RC_DEBUG_COUNTER_STRINGS,
    OPAL_RC_DEBUG_COUNTER_ARRAYS,
    OPAL_RC_DEBUG_COUNTER_BYTES,
    OPAL_RC_DEBUG_COUNTER_BUILDERS,
    OPAL_RC_DEBUG_COUNTER_FILESYSTEM_OBJECTS,
    OPAL_RC_DEBUG_COUNTER_METADATA_PERMISSIONS,
    OPAL_RC_DEBUG_COUNTER_ERROR_PAYLOADS,
    OPAL_RC_DEBUG_COUNTER_RC_CHILD_ARRAYS,
    OPAL_RC_DEBUG_COUNTER_KIND_COUNT
} OpalRcDebugCounterKind;

/* RC header field offsets (bytes from header start) — for codegen use */
#define OPAL_RC_REFCOUNT_OFFSET    offsetof(OpalRcHeader, refcount)
#define OPAL_RC_WEAK_COUNT_OFFSET  offsetof(OpalRcHeader, weak_count)
#define OPAL_RC_DROP_FN_OFFSET     offsetof(OpalRcHeader, drop_children_fn)
#define OPAL_RC_HEADER_SIZE        sizeof(OpalRcHeader)

/* Array payload field offsets (bytes from payload start) — for codegen use */
#define OPAL_ARRAY_LEN_OFFSET      offsetof(OpalArrayPayloadHeader, len)
#define OPAL_ARRAY_CAP_OFFSET      offsetof(OpalArrayPayloadHeader, cap)
#define OPAL_ARRAY_HEADER_SIZE     sizeof(OpalArrayPayloadHeader)

OPAL_STATIC_ASSERT(OPAL_RC_REFCOUNT_OFFSET == 0, "refcount offset must be 0");
OPAL_STATIC_ASSERT(OPAL_RC_WEAK_COUNT_OFFSET == 8, "weak_count offset must be 8");
OPAL_STATIC_ASSERT(OPAL_RC_DROP_FN_OFFSET == 16, "drop_children_fn offset must be 16");
OPAL_STATIC_ASSERT(OPAL_RC_HEADER_SIZE == 24, "OpalRcHeader size must be 24");
OPAL_STATIC_ASSERT(OPAL_ARRAY_LEN_OFFSET == 0, "array len offset must be 0");
OPAL_STATIC_ASSERT(OPAL_ARRAY_CAP_OFFSET == 8, "array cap offset must be 8");
OPAL_STATIC_ASSERT(OPAL_ARRAY_HEADER_SIZE == 16, "OpalArrayPayloadHeader size must be 16");

/**
 * opal_rc_alloc — allocate a new RC object with refcount=1, weak_count=0.
 *
 * @param payload_size      size of the payload in bytes
 * @param drop_children_fn  function to enqueue child RC objects (may be NULL)
 * @return pointer to the payload (NOT the header); NULL on allocation failure
 */
void *opal_rc_alloc(size_t payload_size,
                    void (*drop_children_fn)(void *, void ***, size_t *, size_t *));

/**
 * opal_rc_alloc_tracked — allocate an RC object and tag it for debug counters.
 *
 * Runtime internals use this for surface-specific accounting while preserving
 * the stable payload ABI.
 */
void *opal_rc_alloc_tracked(size_t payload_size,
                            void (*drop_children_fn)(void *, void ***, size_t *, size_t *),
                            OpalRcDebugCounterKind counter_kind);

/**
 * opal_rc_reuse — reset an RC object for reuse by a new allocation.
 *
 * This is a Perceus optimization hook used when the caller has proven unique
 * ownership (`refcount == 1`) and wants to reuse the existing allocation
 * instead of performing free + malloc.
 *
 * The function resets strong/weak counters, installs the new drop callback,
 * and zeroes the payload.
 *
 * @param obj              payload pointer to the reusable allocation
 * @param new_drop_fn      drop callback for the new logical object
 * @param payload_size     payload size in bytes to clear
 */
void opal_rc_reuse(void *obj,
                   void (*new_drop_fn)(void *, void ***, size_t *, size_t *),
                   size_t payload_size);

/**
 * opal_rc_is_unique — report whether an RC object has exactly one strong owner.
 *
 * This predicate ignores weak references. It is suitable for semantic
 * uniqueness checks, but not for storage/header reuse decisions.
 *
 * @param obj  payload pointer (as returned by opal_rc_alloc)
 * @return non-zero when refcount == 1, otherwise 0
 */
int opal_rc_is_unique(const void *obj);

/**
 * opal_rc_is_reuse_eligible — report whether an RC object may be safely reused.
 *
 * Reuse eligibility is stricter than strong uniqueness: callers must not reuse
 * or move an allocation while weak references still observe its header.
 *
 * @param obj  payload pointer (as returned by opal_rc_alloc)
 * @return non-zero when refcount == 1 && weak_count == 0, otherwise 0
 */
int opal_rc_is_reuse_eligible(const void *obj);

/**
 * opal_rc_inc — increment the strong reference count of an RC object.
 *
 * @param obj  payload pointer (as returned by opal_rc_alloc)
 */
void opal_rc_inc(void *obj);

/**
 * opal_rc_dec — decrement the strong reference count.
 * When refcount reaches 0, triggers iterative drop.
 *
 * @param obj  payload pointer
 */
void opal_rc_dec(void *obj);

/**
 * opal_rc_drop_iterative — iteratively drop an RC object and all its children.
 * Uses a work-list (stack) to avoid recursion. Called by opal_rc_dec when
 * refcount reaches 0.
 *
 * @param obj  payload pointer of the object to drop
 */
void opal_rc_drop_iterative(void *obj);

/**
 * opal_runtime_reset_heap_accounting — reset runtime heap allocation counters.
 *
 * This measurement tracks only Opalescent RC/array heap bytes allocated via
 * `opal_rc_alloc`, excluding process RSS and internal bookkeeping overhead.
 */
void opal_runtime_reset_heap_accounting(void);

/**
 * opal_runtime_live_heap_bytes — current live Opalescent RC/array heap bytes.
 */
size_t opal_runtime_live_heap_bytes(void);

/**
 * opal_runtime_peak_heap_bytes — peak live Opalescent RC/array heap bytes.
 */
size_t opal_runtime_peak_heap_bytes(void);

/**
 * Debug-only surface counter registry hooks for runtime internals.
 */
void opal_rc_debug_note_alloc(OpalRcDebugCounterKind kind);
void opal_rc_debug_note_free(OpalRcDebugCounterKind kind);

/**
 * opal_array_data_offset — compute the aligned byte offset of array element
 * storage from the beginning of the payload.
 *
 * The offset depends on the payload address because the enclosing RC header is
 * 24 bytes wide; callers must use the runtime helper rather than assuming a
 * compile-time constant for wide-alignment element types.
 *
 * @param array       pointer to array payload header
 * @param elem_align  required element alignment in bytes (0 treated as 1)
 * @return aligned byte offset from array payload pointer to first element
 */
size_t opal_array_data_offset(const void *array, size_t elem_align);

/**
 * opal_array_alloc — allocate an RC-backed array payload using opal_rc_alloc.
 *
 * The returned pointer is the start of the array payload header. The payload
 * bytes begin with `len` and `cap`, followed by aligned element storage.
 *
 * @param elem_size         element size in bytes
 * @param elem_align        required element alignment in bytes (0 treated as 1)
 * @param len               initial logical length
 * @param cap               initial capacity
 * @param drop_children_fn  array-specific RC child drop callback (may be NULL)
 * @return pointer to array payload header; NULL on allocation failure/overflow
 */
void *opal_array_alloc(size_t elem_size,
                       size_t elem_align,
                       size_t len,
                       size_t cap,
                       void (*drop_children_fn)(void *, void ***, size_t *, size_t *));

/**
 * opal_array_len — read the logical length from an array payload.
 */
size_t opal_array_len(const void *array);

/**
 * opal_array_cap — read the capacity from an array payload.
 */
size_t opal_array_cap(const void *array);

/**
 * opal_array_set_len — write the logical length field of an array payload.
 */
void opal_array_set_len(void *array, size_t len);

/**
 * opal_array_set_cap — write the capacity field of an array payload.
 */
void opal_array_set_cap(void *array, size_t cap);

/**
 * opal_array_data — get a pointer to the first array element.
 *
 * @param array       pointer to array payload header
 * @param elem_align  required element alignment in bytes (0 treated as 1)
 * @return pointer to first element storage byte
 */
void *opal_array_data(void *array, size_t elem_align);

/**
 * opal_array_data_const — const-qualified variant of opal_array_data.
 */
const void *opal_array_data_const(const void *array, size_t elem_align);

/**
 * opal_weak_alloc — create a weak reference to an existing RC object.
 * Increments the weak_count of the target object's header.
 *
 * @param strong_obj  payload pointer of the target RC object
 * @return new OpalWeakRef; NULL on allocation failure
 */
OpalWeakRef *opal_weak_alloc(void *strong_obj);

/**
 * opal_weak_upgrade — attempt to upgrade a weak reference to a strong one.
 * Returns the payload pointer if the object is still alive (refcount > 0),
 * or NULL if the object has been dropped. Does NOT increment refcount —
 * caller must call opal_rc_inc if they want to hold the reference.
 *
 * @param weak  weak reference to upgrade
 * @return payload pointer if alive, NULL if dead
 */
void *opal_weak_upgrade(OpalWeakRef *weak);

#if defined(OPAL_ENABLE_INTERNAL_TESTING)
/**
 * Test-only helpers exposing raw strong/weak counts and debug counters for
 * runtime ABI checks.
 */
size_t opal_rc_strong_count_for_test(const void *obj);
size_t opal_rc_weak_count_for_test(const void *obj);
void opal_rc_debug_reset_counters_for_test(void);
size_t opal_rc_debug_live_count_for_test(OpalRcDebugCounterKind kind);
size_t opal_rc_debug_alloc_count_for_test(OpalRcDebugCounterKind kind);
size_t opal_rc_debug_free_count_for_test(OpalRcDebugCounterKind kind);
#endif

/**
 * opal_rc_drop_child — decrement a child reference during iterative drop.
 *
 * When the child strong count reaches zero, its payload is pushed onto the
 * caller's iterative-drop work-list instead of recursing.
 */
void opal_rc_drop_child(void *obj,
                        void ***stack,
                        size_t *stack_top,
                        size_t *stack_cap);

/**
 * opal_weak_dec — release a weak reference.
 * Decrements weak_count. If both refcount == 0 and weak_count == 0,
 * frees the header memory.
 *
 * @param weak  weak reference to release (freed after this call)
 */
void opal_weak_dec(OpalWeakRef *weak);

#endif /* OPAL_RC_H */
