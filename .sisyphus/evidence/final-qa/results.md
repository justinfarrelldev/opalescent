# Final QA Evidence — Opalescent Compiler
Date: Sun Apr 12 2026

## Step 1: Build
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
Exit: 0
```
**PASS**

## Step 2: Program Execution

### hello-world
```
target/program
Hello world
Exit: 0
```
Expected: "Hello world" — **PASS**

### fib-recursive
```
target/program
fib(10) = 55
Exit: 0
```
Expected: "fib(10) = 55" — **PASS**

### fib-iterative
```
target/program
fib(10) = 55
Exit: 0
```
Expected: "fib(10) = 55" — **PASS**

### simple-quiz (input: "TestUser\n3")
```
target/program
What is your name?
Hello, TestUser! Guess a number between 1 and 5
3, huh? Let's see how close you are, TestUser...
Oh no, too low, you lose!
Exit: 0
```
Expected: starts with "What is your name?" — **PASS**

## Step 3: Spec File Identity
All 4 test-project .op files are IDENTICAL to language-spec counterparts.
**4/4 IDENTICAL**

## Step 4: No-Runtime Portability
Renamed runtime/ → runtime_backup/, compiled and ran hello-world:
```
target/program
Hello world
Exit: 0
```
Runtime folder NOT needed at runtime — embedded in binary. **PASS**

## Step 5: Cross-Directory Compilation
Compiled from /tmp with absolute paths:
```
target/program
Hello world
Exit: 0
```
**PASS**

## Step 6: Test Suite
Unit tests: 0 passed; 0 failed; 9 ignored (doc-tests, all ignored as expected)
Integration tests (--features integration): 7 passed; 0 failed
**7/7 integration PASS**

