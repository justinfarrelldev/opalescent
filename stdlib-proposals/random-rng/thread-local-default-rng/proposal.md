# Thread-Local Default RNG

## Overview
This proposal introduces a thread-local default random number generator to the standard library. While providing the convenience of a global generator, it ensures thread safety by providing each thread its own state.

A `RandomNumberGenerator` type is still available for cases where an explicit handle is needed, but for common tasks, the library provides a set of top-level functions that use the default RNG.

## Assumes
- Support for thread-local storage in the Opalescent runtime.
- Fixed memory model for managing the default generator instance.
- Error handling via `errors` clause and `guard`/`propagate`.

## Syntax Design
```opal
import next_random_uint32, random_integer_in_range from random

##
    Description: Generate a random 32-bit unsigned integer using the default RNG.
##
let next_random_uint32 = f(): uint32 =>
    # Implementation using thread-local state
    return 42

##
    Description: Generate a random integer in the range [low, high) using the default RNG.
##
let random_integer_in_range = f(low: int32, high: int32): int32 errors InvalidRangeError =>
    if low >= high:
        return InvalidRangeError
    return 42
```

## Example Applications
Convenience is the main goal. Simple random number generation can be done without any setup.

```opal
let r = next_random_uint32()
let num = random_integer_in_range(1, 10)
```

## Strengths
- **Ergonomics**: Minimal boilerplate for common tasks.
- **Convenience**: No need to manage RNG state manually.
- **Thread Safety**: Each thread has its own default generator, avoiding contention.
- **Discovery**: Intuitive API for beginners.

## Weaknesses
- **Implicit State**: Some developers may prefer explicit side effects.
- **Reproducibility**: Global default state can make it harder to reproduce specific sequences.
- **Runtime Complexity**: Requires thread-local storage support in the runtime.

## Impact on Existing Syntax
New standard library addition. Does not break any existing syntax but requires runtime support for thread-local storage.

## Interactions with Other Concerns
- **Error Handling**: Uses the standard `errors` clause for range validation.
- **Concurrency**: Leverages thread-local storage for safety without locks.
- **Async**: Needs careful integration with green-thread or fiber-based deferred models.

## Implementation Difficulty
Medium. PRNG logic is simple, but thread-local storage management adds complexity.

## Must NOT Have
- No `rand()` abbreviation.
- No global shared state with locking.
- No assumption of cryptographic security.
- No `_sync` suffix (CPU-only).
