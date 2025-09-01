Math is straightforward, barring a couple things. 

It follows the same model as math in C# as far as evaluation order.

# Basic Algebraic Operators

Addition:
a = 2 + 2

Subtraction:
a = 2 - 2

Multiplication: 
a = 2 * 2

Division:
a = 2 / 2

Exponents (this is two to the power of four):
a = 2 ^ 4

NOTE: bitwise XOR is handled with bit_xor

Square root, etc. will be handled in the standard library.

Cross-type numeric comparisons are forbidden - if comparisons are done to ints and floats (or int64 and int32, for example) then those cause compiler errors. The compiler should hint that the type should be adjusted for one of them.

a = b = c is disallowed.

Binary numeric ops require operands of the same type after inference. An explicit cast is required to convert the types.

## Boolean Operations

AND

something and something2

OR

something or something2

NOT

something and not something2

XOR

something xor something2

Bitwise XOR

something bit_xor something2

## Precedence & associativity (mostly matches C#)

integer / truncates toward zero, and % has the sign of the dividend

exponent binds tighter than unary (so -2 ^ 2 → -4) and exponent stays right-associative (2 ^ 3 ^ 2 → 2 ^ (3 ^ 2)).

Binary operator operands and function arguments are evaluated left-to-right.

Parentheses
( … ) — force grouping.

Unary (right-associative)
+a, -a, not a (boolean NOT), bnot a (bitwise NOT).
No ++/--.

Exponent (right-associative; your addition)
a ^ b (power).

Multiplicative (left-associative, div_euclid and mod_euclid are operators)
*, /, %, div_euclid, mod_euclid.

Additive (left-associative)
+, -.

Shift (left-associative)
bshl, bshr (arithmetic), bushr (logical/unsigned).

Relational (non-associative)
<, <=, >, >=.

Equality (non-associative, and not type-testing)
is, is not.

type_of() function can be used to determine type equivalence... for example "if type_of(user_number) is type_of(quiz_num)"

type_of is const-eval when all inputs are compile-time (e.g., generic T, or a literal/const). Runtime otherwise.

Bitwise AND (left-associative)
band.

Bitwise XOR (left-associative)
bxor.

Bitwise OR (left-associative)
bor.

Logical AND (left-associative; C# “conditional AND”)
and.

Logical XOR (left-associative; logical XOR)
xor.

Logical OR (left-associative; C# “conditional OR”)
or.

Null-coalescing
Not supported.

Conditional operator
Not used (using Rust-style if expressions instead).

Assignment (right-associative)
=.
(No compound assignments)

# Bitwise notes

## Semantics (prioritize safety)

Logical AND and OR both short-circuit - logical xor does not.

No negative counts: using a negative shift count is a compile error if known at compile time; otherwise a runtime trap.

bit-width must be ≥ 1.

No silent masking: if n >= bitwidth(lhs), that’s a compile error (when n is a constant) or a runtime trap (when n is non-constant).

Rationale: masking (n % bitwidth) is convenient but hides bugs; make it explicit via the variants below.

Right shifts:

bshr: arithmetic (propagates sign bit).

bushr: logical (fills with zero), result type is the same as the left operand’s type.

Well-typed only: shifts require an integer left operand and an integer shift count; mixed signed/unsigned ints follow normal numeric conversion rules (keep it consistent with + - *).

## Checked vs. “I know what I’m doing” variants

Logical AND and OR both short-circuit - logical xor does not.

Provide explicit stdlib intrinsics so users can choose behavior:

Checked (default operators above): trap on bad counts.

Masked variants (wrap count by bit-width—what C#/JS effectively do):

masked_bshl(a, n), masked_bshr(a, n), masked_bushr(a, n)

Wrapping-value variants (rarely needed; mostly for DSP/crypto bit-twiddling):

wrapping_bshl(a, n), wrapping_bshr(a, n), wrapping_bushr(a, n)
(Masks the count; value wraps naturally by type width.)

Rotates & helpers (stdlib bits module):

rotl(a, n), rotr(a, n), popcount(a), clz(a), ctz(a), bit_test(a, i), bit_set(a, i), bit_clear(a, i), bit_toggle(a, i)

Policy: the operators are the safe, checked form. Any “do it anyway” behavior must be spelled out by calling the explicit variant.

Precedence (slots that matter here)

bnot sits with other unary ops.

Shifts bshl/bshr/bushr sit below multiplicative and above relational (same spot as C#’s << >> >>>).

Bitwise: band → bxor → bor (from high to low), mirroring C#’s & ^ |.

Logical: and → xor → or; and/or short-circuit, xor does not.

Bitwise ops are integer-only: Booleans use and/or/not/xor (logical), integers use band/bor/bxor/bnot + shifts.

## Division by Zero

a / 0 or a % 0 → runtime trap (compile error if compile-time known).

# Arithmetic Overflow

Integer + - * and bshl trap on overflow in Debug; in Release, use explicit variants: checked_*, wrapping_*, saturating_* (and wrapping_bshl).”

# IEEE for Floats

Floats follow IEEE-754 by default, but ship:

a strict_floats build flag (treat Infinity/NaN as traps at function boundaries).

Integer / and % by 0 → trap (compile-time error if constant).
Float / by 0:
  - Option A (IEEE default): produce ±Inf/NaN.
  - Option B (strict): trap.  // If chosen, expose checked_div and a NonNaN<T> newtype.
Pick one and state it here.
