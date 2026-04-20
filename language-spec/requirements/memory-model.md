# Memory Model: Perceus + Second-Class References

## 1. Motivation
Opalescent employs a hybrid memory management strategy combining **Perceus-style Reference Counting** with **Second-Class References** to provide high performance, safety, and predictability.

- **Deterministic Deallocation**: Unlike Garbage Collection (GC), Reference Counting (RC) ensures that memory and resources are reclaimed as soon as they are no longer needed, avoiding unpredictable stop-the-world pauses.
- **Predictable Latency**: Deterministic deallocation is critical for real-time systems and hot-reloading scenarios where consistent performance is required.
- **No Lifetime Annotations**: Second-class references provide memory safety without the complexity of a borrow checker or explicit lifetime annotations, keeping the language simple yet robust.
- **Deep Structure Safety**: The iterative drop algorithm prevents stack overflows when deallocating deeply nested structures (e.g., long linked lists).
- **Explicit Cycle Breaking**: Weak references (`Weak<T>`) allow for explicit cycle breaking without the overhead and complexity of a background cycle collector.

## 2. Second-Class References

Second-class references in Opalescent provide a way to borrow data without taking ownership, while ensuring that the references never outlive the data they point to.

### 2.1 Syntax
- `ref x: T` — A read-only (immutable) borrow of a value.
- `mutable ref x: T` — A mutable borrow of a value.

Second-class references are restricted to function parameters. They **cannot** appear in:
- `let` bindings (locals)
- Return types
- Struct fields
- Collection elements

### 2.2 Escape Rules (FORMAL)
The type checker enforces the following rules to prevent dangling references (see `src/type_system/checker/ref_rules.rs`):
1. **No Return**: A `ref` or `mutable ref` parameter cannot be returned from a function.
2. **No Storage**: A `ref` or `mutable ref` parameter cannot be stored in a variable or structure that could outlive the function call.
3. **No Closure Capture**: A `ref` or `mutable ref` parameter cannot be captured by a closure.
4. **No Unsafe Aliasing**: A `mutable ref` parameter cannot be aliased at the same call site (i.e., you cannot pass two mutable references to the same object if they might overlap).

### 2.3 Lowering
- `ref x: T` is lowered to a raw pointer in LLVM IR.
- The callee receives this pointer and performs reads via double-loads (loading the pointer, then the value).
- No RC operations (`inc`/`dec`) are emitted for `ref` parameters as they do not own the object.

## 3. Reference Counting

### 3.1 RC-Managed Types
Reference counting is used for heap-allocated types:
- `string`: UTF-8 encoded, heap-allocated strings.
- `T[]` (Arrays): Dynamic heap-allocated arrays.
- User-defined heap types (e.g., `class` instances).

Primitives (e.g., `int32`, `float64`, `boolean`) are stored inline and are **not** RC-managed.

### 3.2 RC Object Header Layout (ABI-stable)
Every RC-managed object is preceded by a 24-byte header. The user pointer points directly to the payload, with the header residing at `pointer - 24`.

| Offset | Size | Field | Description |
| :--- | :--- | :--- | :--- |
| 0 | 8 | `refcount` | Strong reference count (`size_t`) |
| 8 | 8 | `weak_count` | Weak reference count (`size_t`) |
| 16 | 8 | `drop_children_fn` | Function pointer for enqueuing children |

Total Header Size: **24 bytes**.

**ABI Stability**: This layout is guaranteed to remain stable across compiler versions to facilitate binary compatibility and hot-module reloading.

### 3.3 RC Operations
The runtime provides the following C-compatible API (see `runtime/opal_rc.h`):
- `void *opal_rc_alloc(size_t size, void (*drop_fn)(...))`: Allocates an object with `refcount=1`.
- `void opal_rc_inc(void *obj)`: Increments the `refcount`.
- `void opal_rc_dec(void *obj)`: Decrements the `refcount`. If it reaches 0, triggers `opal_rc_drop_iterative`.
- `void opal_rc_drop_iterative(void *obj)`: Deallocates the object using a work-list to avoid recursion.

### 3.4 RC Insertion Points
The compiler's `RcAnalysis` pass (see `src/type_system/rc_analysis.rs`) plans where to insert operations:
- **Function Entry**: `opal_rc_inc` is called for each owned RC parameter.
- **Return/Exit**: `opal_rc_dec` is called for each owned local or parameter that is still alive.
- **Assignment**: `opal_rc_inc` on the new value, followed by `opal_rc_dec` on the old value of the target.
- **Perceus Optimization**: If a value's last use is an assignment, the `dec` may be optimized into a `reuse`.

## 4. Iterative Drop Algorithm

### 4.1 Motivation
Recursive deallocation of nested structures (like a linked list) can lead to stack exhaustion. Opalescent uses an iterative approach to ensure safety regardless of data depth.

### 4.2 Algorithm
```
function drop_iterative(obj):
    stack = [obj]
    while stack is not empty:
        current = stack.pop()
        header = current - HEADER_SIZE
        
        # Decrement and check
        if header.refcount > 0: continue
        
        # Enqueue children if any
        if header.drop_children_fn != NULL:
            header.drop_children_fn(current, &stack)
            
        free(header)
```

### 4.3 drop_children_fn Contract
Each RC-managed type generates a `drop_children_fn` which:
1. Takes the payload pointer and the work-list stack.
2. Pushes the payload pointers of all child RC objects onto the stack.
3. Does **not** decrement child refcounts or free them; that is handled by the main loop.

## 5. Weak References

### 5.1 Semantics
A `Weak<T>` reference allows observing an object without owning it or preventing its destruction.
- Holds a pointer to the **header** (not the payload).
- Increments `weak_count`, not `refcount`.
- Does **not** prevent the payload from being dropped when `refcount` hits 0.

### 5.2 Lifecycle
1. When `refcount == 0`: Payload is dropped/freed. Header remains if `weak_count > 0`.
2. When `refcount == 0` AND `weak_count == 0`: Header memory is finally freed.

### 5.3 Operations
- `opal_weak_alloc(obj)`: Creates a weak reference, increments `weak_count`.
- `opal_weak_upgrade(weak)`: Checks `refcount`. If `> 0`, returns payload pointer. Else returns `NULL`.
- `opal_weak_dec(weak)`: Decrements `weak_count` and frees header if necessary.

In Opalescent, the `guard` statement is the primary way to safely upgrade and use weak references:
```opalescent
guard weak_ref into strong_ref else {
    # handle dead reference
    return
}
# strong_ref is now a valid owned reference
```

## 6. Perceus Reuse Optimization

Perceus allows the compiler to reuse an object's memory instead of freeing and reallocating it, significantly reducing allocator pressure.

### 6.1 Conditions for Reuse
Reuse is safe and performed when:
1. **Uniqueness**: The source variable has a `refcount` of exactly 1 (proven statically or checked at runtime).
2. **Temporal Proximity**: The source variable's last use is immediately followed by an allocation of the same layout.
3. **Compatibility**: The source and target types have identical size and alignment.
4. **Ownership**: The source is an owned variable (not a `ref` parameter).

### 6.2 Analysis Pass
The `ReuseAnalysis` pass (see `src/type_system/rc_analysis.rs`) identifies these patterns and replaces `dec` + `alloc` sequences with a single `opal_rc_reuse` call.

### 6.3 Runtime Operation
`opal_rc_reuse(obj, new_drop_fn, size)`:
- Resets `refcount` to 1.
- Resets `weak_count` to 0.
- Installs the new `drop_children_fn`.
- Zeroes the payload memory.

## 7. ABI Stability and Module Imports

### 7.1 Stability Guarantee
The 24-byte RC header is considered a permanent part of the Opalescent ABI. This allows modules compiled with different versions of the compiler (or even different languages) to exchange RC-managed objects safely.

### 7.2 Module Boundaries
When objects cross module boundaries:
- The `drop_children_fn` pointer must remain valid.
- Module hot-swapping must ensure that pending drop operations in the work-list don't call into unloaded code (handled by the host-orchestrated swap).

## 8. Invariants and Safety Guarantees

The Opalescent memory model maintains the following invariants:
- **Reference Integrity**: `refcount >= 1` for any object accessible via a strong reference.
- **Memory Safety**: No use-after-free or dangling references due to strict second-class reference escape rules.
- **Leak Prevention**: RC ensures memory is reclaimed; `Weak<T>` provides tools to break cycles manually.
- **Stack Safety**: Iterative drop prevents deep recursion during deallocation.
- **Concurrency Ready**: RC operations are designed to be atomic (though the current implementation is single-threaded).
