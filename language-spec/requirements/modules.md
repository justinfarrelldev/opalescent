The "public" keyword is used to export any functions, variables or types. .types.(ending) files can, indeed, have the public keyword in them.

# nums.(ending)
public let is_prime = f(n: int32): boolean => ...
public let gcd      = f(a: int32, b: int32): int32 => ...
public let pi = 3.14
let internal_helper = f(x: int32): int32 => ...   # not exported

Within a module, at most one public symbol may use a given identifier.

public let name = "Timmy"
public let name = f(): void => return "Timmy" # not allowed; name is already exported

The name of the file (with relative path) is the way to import it:

import is_prime from ./nums

In the case of a name clash, the compiler will not compile. It will tell you to alias one:

import is_prime as is_prime_new from nums # now you can use is_prime_new for the one from this module
import is_prime from math

import gcd from ./math      # explicitly point at local file
import sqrt from math   # explicitly point at standard library

Packages are always the repository creator and name after an @:

import leftpad from @leftpaddev/leftpad

Importing types will always have .types at the end:

import PrimeFactorization from ./nums.types
import gcd, is_prime from ./nums

Bare specifiers are stdlib only; .//, ../, etc. are local only; @scope/name are packages only.

Multiple imports: 

import is_prime, gcd, pi from ./nums
import is_prime as is_prime_new, gcd as greatest_cd from ./nums
import type User, Address from ./models.types

Allow member tracing when member names are statically referenced, else fall back to whole-module include. This applies below with m.sqrt: 

import math as m
let x = m.sqrt(9)

Imports are case-sensitive. Since the language requires snake_case for file names, this means import specifiers will never be capitalized
(and only types will be capitalized when being imported).