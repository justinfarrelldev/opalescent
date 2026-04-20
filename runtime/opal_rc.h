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
 *   refcount        (size_t, offset 0)  — strong reference count
 *   weak_count      (size_t, offset 8)  — weak reference count
 *   drop_children_fn (fn ptr, offset 16) — enqueues child RC objects onto work-list
 *
 * The header immediately precedes the payload in memory. The user-visible
 * pointer points to the payload (header - 1). This keeps the ABI stable:
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

/* RC object header — precedes every heap-allocated RC object in memory */
typedef struct OpalRcHeader {
    size_t refcount;
    size_t weak_count;
    /**
     * drop_children_fn — called during iterative drop to enqueue child RC
     * objects onto the work-list. Signature:
     *   obj:       pointer to the payload (header + 1)
     *   stack:     pointer to the work-list stack array (may be reallocated)
     *   stack_top: current top index (in/out)
     *   stack_cap: current capacity (in/out)
     * The function should push each child RC payload pointer onto the stack.
     * Pass NULL if the object has no RC children.
     */
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

/**
 * opal_rc_alloc — allocate a new RC object with refcount=1, weak_count=0.
 *
 * @param payload_size      size of the payload in bytes
 * @param drop_children_fn  function to enqueue child RC objects (may be NULL)
 * @return pointer to the payload (NOT the header); NULL on allocation failure
 */
void *opal_rc_alloc(size_t payload_size,
                    void (*drop_children_fn)(void *, void **, size_t *, size_t *));

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
                   void (*new_drop_fn)(void *, void **, size_t *, size_t *),
                   size_t payload_size);

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

/**
 * opal_weak_dec — release a weak reference.
 * Decrements weak_count. If both refcount == 0 and weak_count == 0,
 * frees the header memory.
 *
 * @param weak  weak reference to release (freed after this call)
 */
void opal_weak_dec(OpalWeakRef *weak);

#endif /* OPAL_RC_H */
