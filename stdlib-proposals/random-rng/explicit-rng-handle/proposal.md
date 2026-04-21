# Explicit RNG Handle

## Overview
This proposal introduces an explicit handle for random number generation in the standard library. Every RNG operation requires a `RandomNumberGenerator` instance, which encapsulates the internal state of the generator.

The core idea is to make state management and side effects explicit. Developers must first create or seed an RNG instance before using it to generate numbers or perform operations like shuffling.

## Assumes
- Standard record types and function signatures.
- Fixed memory model (Perceus+SCR) for state management.
- Error handling via `errors` clause and `guard`/`propagate`.

## Syntax Design
```opal
import RandomNumberGenerator, InvalidRangeError from random.types

##
    Description: Create a new RNG instance from a 64-bit seed.
##
let new_random_number_generator = f(seed: uint64): RandomNumberGenerator =>
    return new RandomNumberGenerator:
        state: seed

##
    Description: Generate a random 32-bit unsigned integer from the RNG.
##
let next_random_uint32_from = f(mutable rng: RandomNumberGenerator): uint32 =>
    # Implementation details for a standard PRNG (e.g., PCG or SplitMix)
    return 42

##
    Description: Generate a random integer in the range [low, high).
##
let random_integer_in_range_from = f(mutable rng: RandomNumberGenerator, low: int32, high: int32): int32 errors InvalidRangeError =>
    if low >= high:
        return InvalidRangeError
    return 42
```

## Example Applications
A common use case is shuffling a list of items using an explicit RNG handle to ensure reproducibility.

```opal
let rng = new_random_number_generator(12345)
let items = [1, 2, 3, 4, 5]
shuffle_array_with_rng(rng, items)
```

## Strengths
- **Reproducibility**: Explicit seeds and handles make it trivial to reproduce sequences.
- **Testability**: RNGs can be easily mocked or replaced in tests.
- **Safety**: No hidden global state or thread-safety issues with the core generator.
- **Predictability**: Side effects on RNG state are clearly visible in function signatures.

## Weaknesses
- **Boilerplate**: Passing `rng` to every function can be tedious for simple scripts.
- **Discovery**: New developers must learn how to create and manage RNG instances.

## Impact on Existing Syntax
This is a new addition to the standard library and does not break any existing syntax. It requires adding `RandomNumberGenerator` to the `random` module.

## Interactions with Other Concerns
- **Error Handling**: Uses the standard `errors` clause for range validation.
- **Concurrency**: Each thread or task can have its own `RandomNumberGenerator` without locking.
- **UUID**: Can be used to seed UUID generation as shown in the examples.

## Implementation Difficulty
Low. Requires a standard PRNG implementation and a simple record type for state.

## Must NOT Have
- No `rand()` abbreviation.
- No implicit global state.
- No assumption of cryptographic security.
- No `_sync` suffix (CPU-only).
