# Opalescent Language Implementation Plan

This document outlines the comprehensive plan for implementing the Opalescent programming language, a compiled, statically and strongly typed language with hot reloading capabilities.

**Current Project Status**: Phase 1 (Foundation & Core Infrastructure) - Type System Core implementation in progress. The lexer and parser are complete with 213 tests passing. Type System Core has most infrastructure complete but **11 critical Phase 2 blockers must be addressed** before Phase 2 can begin. These blockers include fundamental language features: error handling (guard/propagate/errors), multiple return values, standard library built-ins, generic constraints, if expression semantics, member access, overflow detection, and warning infrastructure.

## Phase 1: Foundation & Core Infrastructure

### ✅ Project Setup

- [x] Initialize Rust project structure
- [x] Set up cargo-make configuration
- [x] Configure linting and testing infrastructure

### ✅ Lexical Analysis (Name: lexer-plan.md)

- [x] Implement tokenization for keywords, identifiers, literals
- [x] Handle operators and punctuation
- [x] String literal tokenization (interpolation parsed at parser level)
- [x] Whitespace consistency checking (spaces vs tabs)
- [x] Comment handling (single # and multi-line ##)
- [x] Doc comment parsing and preservation
- [x] Error recovery and multi-error collection
- [x] Comprehensive test coverage

### ✅ Parser Foundation (Name: parser-foundation-plan.md)

#### AST Node Definitions

- [x] Core AST Infrastructure (base trait, source location, visitor pattern)
- [x] Expression AST Nodes (literals, identifiers, binary/unary ops, calls, casts)
- [x] Statement AST Nodes (let bindings, assignments, returns, blocks)
- [x] Declaration AST Nodes (functions, types, imports, public/entry declarations)
- [x] Control Flow AST Nodes (if, for, while, loop, break/continue)
- [x] Type AST Nodes (generics, function types, custom types)
- [x] String interpolation expressions
- [x] Type checking expressions (type_of)

#### Parser Implementation

- [x] Core Parser Structure (token stream, lookahead, error collection)
- [x] Expression Parsing (Pratt parser, operator precedence)
- [x] Statement Parsing (declarations, assignments, blocks)
- [x] Declaration Parsing (functions, types, imports)
- [x] Type Parsing (basic, generic, array, function types)
- [x] Import statement parsing
- [x] Type declaration parsing

#### Error Handling

- [x] Parse Errors (unexpected/missing tokens, invalid syntax)
- [x] Error Recovery (panic mode, synchronization points)
- [x] Error Reporting (miette integration, source context, suggestions)

#### Advanced Features

- [x] String Interpolation parsing
- [x] Comments and Documentation preservation (doc comments)
- [x] Operator Precedence (Pratt parser with precedence table)

#### Testing & Validation

- [x] Unit Tests (AST nodes, expression/statement/declaration parsing)
- [x] Integration Tests (comprehensive test coverage)
- [x] Property-based tests for parser robustness
- [x] Error handling validation

### ⏳ Type System Core (Name: type-system-core-plan.md)

#### Foundation Infrastructure (✅ Complete)

- [x] Basic type representation (all int32/64, uint32/64, int8/16, uint8/16, float32/64, string, boolean, unit)
- [x] Extended integer and floating point type support
- [x] Basic type environment and context management
- [x] Type checking framework foundation
- [x] Error reporting framework with miette integration
- [x] AST Type to CoreType conversion
- [x] Type variable system for inference
- [x] Substitution system for type variables
- [x] Complete unification algorithm with occurs check
- [x] Array, Function, and Generic type infrastructure
- [x] Comprehensive test suite (213 tests passing)

#### Enhanced Error Handling (✅ Complete)

- [x] SourceSpan integration in all TypeError variants
- [x] Diagnostic codes and help text (miette #[diagnostic] attributes)
- [x] Span propagation through all type checking methods
- [x] Helper methods for span creation (span_from_span, unknown_span)
- [x] Compliance with ERROR_HANDLING_STANDARDS.md

#### Phase 2 Preparation Infrastructure (✅ Complete)

- [x] **Symbol Table System** (SymbolTable, SymbolInfo, SymbolType, Visibility)
  - [x] register_symbol() method for adding symbols to scope
  - [x] get_symbol() method for symbol lookup
  - [x] enter_scope() / exit_scope() for scope management
  - [x] Shadowing support and scope hierarchy
- [x] **Constraint Collection Infrastructure** (TypeConstraint enum)
  - [x] Equality constraints for type unification
  - [x] HasField constraints for struct field checking (deferred to Phase 3)
  - [x] Callable constraints for function type checking
  - [x] add_constraint() method for constraint tracking
  - [x] solve_constraints() implementation (equality and callable constraints working)
- [x] **Test maintainability improvements**
  - [x] Replace magic numbers with semantic constants
  - [x] TEST_VAR_ID, ANOTHER_TEST_VAR_ID, THIRD_TEST_VAR_ID constants

#### Phase 6 Hot Reload Preparation (✅ Complete)

- [x] **ABI Layout Infrastructure** (MemoryLayout struct)
  - [x] memory_layout() const method on CoreType
  - [x] Size and alignment calculation for all types
  - [x] Support for Phase 6 ABI compatibility checking
- [x] **Symbol Table for ABI Signature Generation**
  - [x] SymbolInfo struct with type information
  - [x] Preparation for cross-module state preservation

#### Comprehensive Documentation (✅ Complete)

- [x] Module-level documentation
- [x] Phase integration documentation (Phase 2, 6 dependencies)
- [x] Architecture overview (Hindley-Milner type inference)
- [x] Future enhancements section
- [x] Error handling patterns documentation
- [x] Code examples for common patterns

#### Type Checking Implementation (✅ Partially Complete)

- [x] **Expression Type Checking** (expressions.rs)
  - [x] Literal expressions
  - [x] Identifier resolution
  - [x] Binary operations (arithmetic, comparison, logical, bitwise)
  - [x] Unary operations (negation, not, bitwise not)
  - [x] Function calls with arity and parameter checking
  - [x] Array access and array literals
  - [x] Cast expressions with validation
  - [x] String interpolation
  - [x] Lambda expressions (parameters, return types, body checking)
  - [x] Type inference for expression trees
- [x] **Statement Type Checking** (statements.rs)
  - [x] Let bindings with type inference
  - [x] Assignment statements
  - [x] Return statements with expected type validation
  - [x] Expression statements
  - [x] Block statements with scope management
  - [x] If statements with boolean condition enforcement
  - [x] For loops with array iteration
  - [x] While loops with boolean condition
  - [x] Loop statements
  - [x] Break/continue statements
- [x] **Declaration Type Checking** (declarations.rs)
  - [x] Function declarations with parameter and return type validation
  - [x] Function body type checking in new scope
  - [x] Let declarations (module-level)
  - [x] Type declarations (ADT validation)
  - [x] Forward reference handling (two-pass: signature registration, then body checking)
  - [x] Program-level type checking with error collection
  - [ ] Import declarations (deferred to Phase 4 - currently acknowledged but not validated)

#### Cast Validation and Safety (✅ Complete)

- [x] Safe cast checking (widening within same numeric family)
- [x] is_safe_cast() helper function
- [x] is_valid_cast() helper function
- [x] Identity casts (same type)
- [x] Widening integer casts (int8 -> int32, uint16 -> uint64)
- [x] Integer to float conversion (safe but may lose precision)
- [x] Float widening (float32 -> float64)
- [x] Arithmetic overflow handling strategy documented (debug trap, release wrap per math.md)
- [x] Cast validation integrated into expression type checker
- [x] Helper function documentation with cross-references
- [x] Edge case test coverage (Unit, String, Boolean, TypeVar, Generic, Function types)
- [ ] Warning collection system for unsafe casts (deferred to Phase 2 warning infrastructure)

#### Phase 2 Blockers (Name: phase-2-blockers-plan.md)

**CRITICAL: The following items MUST be completed before ANY Phase 2 work can begin. These are fundamental language features required by the specification, not optional enhancements.**

##### 1. Error Handling Language Features (⚠️ HIGHEST PRIORITY - BLOCKS ALL PHASE 2)

- [ ] **Error Type Declarations in Function Signatures**
  - [x] Parse `errors ErrorType1, ErrorType2` clause in function declarations
  - [x] Parse `errors` clause in lambda expressions
  - [x] Store error types in `Decl::Function` AST node
  - [x] Store error types in `Expr::Lambda` AST node
  - [x] Validate error type names exist in type system
  - [ ] Add error types to function signature documentation
- [ ] **Guard Expression Implementation**
  - [x] Add `Expr::Guard` variant to AST with: `expr`, `binding_name`, `else_branch`
  - [x] Parse `guard expr into name else handler` syntax
  - [x] Support optional type/mutability modifiers on guard binding
  - [x] Type check guard expression (guarded expr returns Result/error type)
  - [x] Type check binding name against success type
  - [x] Type check else branch handler for error type compatibility
  - [x] Register guard binding in symbol table for subsequent statements
  - [x] Ensure else branch handles error type correctly
- [ ] **Propagate Keyword Implementation**
  - [x] Add `Expr::Propagate` variant to AST (or extend `Expr::Call`)
  - [x] Parse `propagate function_call()` syntax
  - [x] Validate propagate only used in functions that declare errors
  - [x] Type check propagate ensures error types match function signature
- [ ] **Type System Error Type Support**
  - [x] Add `error_types: Vec<String>` to `CoreType::Function`
  - [x] Implement error type compatibility checking
  - [x] Validate propagate statements match function error signature
  - [x] Check guard expressions handle appropriate error types
  - [x] Update unification algorithm to handle error types
- [ ] **Error Handling Test Coverage**
  - [ ] Create test files with guard/propagate patterns
  - [x] Test multiple error type scenarios
  - [x] Test error type mismatch detection
  - [x] Test propagate in non-error-returning functions (should error)
  - [x] Test guard with incompatible else branch types

##### 2. Multiple Return Values (⚠️ CRITICAL - BLOCKS FUNCTION SYSTEM)

- [ ] **Multiple Return Type Support in AST**
  - [ ] Modify `Type::Function` to support `return_types: Vec<Type>` (replace single return_type)
  - [ ] Modify `Decl::Function` to use `return_types: Vec<Type>`
  - [ ] Modify `Expr::Lambda` to use `return_types: Vec<Type>`
  - [ ] Maintain backward compatibility with single return (Vec of 1 element)
- [ ] **Labeled Return Value Support**
  - [ ] Modify `Stmt::Return` to support `values: Vec<LabeledValue>`
  - [ ] Parse `return label1: expr1, label2: expr2` syntax
  - [ ] Validate label names are unique in return statement
  - [ ] Support mixing labeled and unlabeled returns (unlabeled gets auto-label?)
- [ ] **Parser Updates for Multiple Returns**
  - [ ] Parse `f(...): Type1, Type2, Type3` function return signatures
  - [ ] Parse comma-separated return types in function declarations
  - [ ] Parse comma-separated return types in lambda expressions
  - [ ] Error on label duplication in return statements
  - [ ] Support single return as special case of multiple returns
- [ ] **Type System Multiple Return Support**
  - [ ] Update `CoreType::Function` to handle `return_types: Vec<CoreType>`
  - [ ] Type check multiple return values match signature
  - [ ] Validate labeled return values against function signature labels
  - [ ] Update constraint solving for multi-return functions
  - [ ] Ensure all return statements in function match signature
- [ ] **Multiple Return Test Coverage**
  - [ ] Test functions with multiple return types
  - [ ] Test labeled return statements
  - [ ] Test return value count mismatch errors
  - [ ] Test return label name mismatches
  - [ ] Test single return backward compatibility

##### 3. Standard Library Built-ins (⚠️ CRITICAL - ENABLES TESTING)

- [ ] **Core Built-in Function Signatures**
  - [ ] Define `print<T>(value: T): unit` signature
  - [ ] Define `take_input(): string` signature
  - [ ] Define `string_to_int32(s: string): int32 errors ParseError` signature
  - [ ] Define `random_int32(min: int32, max: int32): int32` signature
  - [ ] Document all built-in function semantics and behavior
- [ ] **Type System Built-in Integration**
  - [ ] Add `TypeEnvironment::register_builtin()` method
  - [ ] Pre-register all built-in functions on `TypeChecker::new()`
  - [ ] Ensure built-ins available in all type checking contexts
  - [ ] Support generic built-in functions (like `print<T>`)
- [ ] **Standard Library Prelude Module**
  - [ ] Create `stdlib/prelude.op` with built-in signatures
  - [ ] Ensure prelude is implicitly imported in all modules
  - [ ] Document standard library module organization
  - [ ] Add prelude to language specification documentation
- [ ] **Built-in Function Testing**
  - [ ] Test type checking with `print()` calls
  - [ ] Test type checking with `take_input()` calls
  - [ ] Test type checking with `string_to_int32()` (error handling)
  - [ ] Test generic built-in instantiation
  - [ ] Validate example files in `language-spec/` can be type checked

##### 4. Generic Type Parameter Constraints (⚠️ HIGH PRIORITY - BLOCKS GENERICS)

- [ ] **Generic Parameter Constraint AST Support**
  - [ ] Extend AST to support type parameter constraints/bounds
  - [ ] Parse constraint syntax in generic type parameters
  - [ ] Store constraints in `Decl::Function` for generic functions
  - [ ] Store constraints in `Expr::Lambda` for generic lambdas
  - [ ] Support multiple constraints per type parameter
- [ ] **Constraint Checking in Type System**
  - [ ] Validate generic parameters satisfy declared constraints
  - [ ] Check constraint satisfaction at generic instantiation sites
  - [ ] Implement constraint solving algorithm for type inference
  - [ ] Add constraint violation error messages with helpful diagnostics
  - [ ] Support constraint propagation through type inference
- [ ] **Generic Type Inference Enhancements**
  - [ ] Infer generic parameters from function call arguments
  - [ ] Validate inferred types satisfy all constraints
  - [ ] Support explicit generic parameter syntax: `map<int32, string>(...)`
  - [ ] Handle constraint conflicts and report clear errors
- [ ] **Generic Constraint Testing**
  - [ ] Test generic functions with constraints
  - [ ] Test constraint violation detection
  - [ ] Test constraint inference from call sites
  - [ ] Test multiple constraints on single type parameter
  - [ ] Test generic ADT with constraints (preparation for Phase 3)

##### 5. If Expression Semantics Clarification (⚠️ HIGH PRIORITY - AFFECTS TYPE CHECKING)

- [ ] **If Expression vs Statement Resolution**
  - [ ] Review language spec for if expression semantics
  - [ ] Determine if `Stmt::If` should become `Expr::If` (Rust-style)
  - [ ] Document value-returning if expressions in specification
  - [ ] Update parser to support if as expression (if needed)
  - [ ] Decide on else-less if semantics (returns unit type?)
- [ ] **If Expression Type Checking**
  - [ ] Ensure both if/else branches return compatible types
  - [ ] Infer if expression result type from branch types
  - [ ] Type check else-less if expressions (must return unit?)
  - [ ] Update constraint collection for if expressions
  - [ ] Add tests for if expression type inference

##### 6. Member Access Type Checking (HIGH PRIORITY - REQUIRED FOR COMPLETENESS)

- [ ] **Member Access Implementation**
  - [ ] Implement `type_check_expr` for `Expr::Member` (currently NotImplementedYet)
  - [ ] Type check object/receiver expression
  - [ ] Validate member exists on object type
  - [ ] Handle module member access (e.g., `math.sqrt`)
  - [ ] Handle struct field access (requires Phase 3 ADT support)
  - [ ] Return member type for subsequent type analysis
- [ ] **Member Access Testing**
  - [ ] Test module member access type checking
  - [ ] Test field access on product types (Phase 3 integration)
  - [ ] Test member access errors (member not found)
  - [ ] Test chained member access (e.g., `obj.field.method`)

##### 7. Arithmetic Overflow Detection (MEDIUM PRIORITY - SPEC COMPLIANCE)

- [ ] **Compile-time Overflow Checking**
  - [ ] Detect overflow in constant arithmetic expressions
  - [ ] Emit errors for overflowing constant additions
  - [ ] Emit errors for overflowing constant multiplications
  - [ ] Emit errors for overflowing bitwise shifts
  - [ ] Document runtime trap behavior for debug mode (per math.md)
- [ ] **Checked Arithmetic Variant Validation**
  - [ ] Parse `checked_add`, `wrapping_add`, `saturating_add` variants
  - [ ] Validate use of explicit overflow-handling variants
  - [ ] Type check checked arithmetic operations
  - [ ] Add tests for all checked/wrapping/saturating variants

##### 8. Division by Zero Detection (MEDIUM PRIORITY - SPEC COMPLIANCE)

- [ ] **Compile-time Division by Zero Checking**
  - [ ] Detect division by zero in constant expressions
  - [ ] Detect modulo by zero in constant expressions
  - [ ] Emit compile-time errors for constant division by zero
  - [ ] Document runtime trap behavior for non-constant division
- [ ] **Division by Zero Testing**
  - [ ] Test constant division by zero detection
  - [ ] Test constant modulo by zero detection
  - [ ] Ensure runtime division preserves error handling

##### 9. Warning System Infrastructure (MEDIUM PRIORITY - ERROR HANDLING ENHANCEMENT)

- [ ] **Warning Collection System**
  - [ ] Add `Warning` type parallel to `TypeError`
  - [ ] Add warning collection to `TypeChecker`
  - [ ] Convert `UnsafeCast` from error to warning
  - [ ] Implement warning display with miette
  - [ ] Support warning suppression annotations
- [ ] **Warning Categories**
  - [ ] Unsafe cast warnings
  - [ ] Unused variable warnings (from Variable System)
  - [ ] Unreachable code warnings (from Control Flow)
  - [ ] Exhaustiveness warnings (from Pattern Matching)

##### 10. Type System Core Plan Synchronization (DOCUMENTATION - DO IN PARALLEL)

- [ ] **Update type-system-core-plan.md**
  - [ ] Mark error handling as critical Phase 2 blocker
  - [ ] Add multiple return value support requirements
  - [ ] Add standard library built-ins section
  - [ ] Mark if expression semantics as needing resolution
  - [ ] Update constraint solver status with new constraints
  - [ ] Document HasField constraint deferral to Phase 3

##### 11. PLAN.md Integration (DOCUMENTATION - DO IN PARALLEL)

- [ ] **Update PLAN.md Phase 2 Structure**
  - [ ] Add "Error Handling System" as standalone phase item
  - [ ] Document guard/propagate/errors syntax requirements
  - [ ] Add dependency notes: error handling blocks function system
  - [ ] Update Function System dependencies on error handling
  - [ ] Cross-reference blocker items in relevant phase sections

#### Remaining Tasks for Phase 1 Type System Core

- [ ] **Generic type runtime instantiation**
  - [x] Generic type representation (CoreType::Generic)
  - [x] Type parameter storage
  - [ ] Concrete type argument inference at call sites (moved to blocker #4)
  - [ ] Generic constraint satisfaction checking (moved to blocker #4)
  - [ ] Monomorphization preparation (deferred to Phase 5)
- [ ] **Type inference engine enhancements**
  - [x] Constraint collection during AST traversal
  - [x] Equality constraint solving
  - [x] Callable constraint solving
  - [ ] HasField constraint handling (deferred to Phase 3 - requires ADT Product types)
  - [ ] Principal type inference refinement
- [ ] **Import declaration type checking** (deferred to Phase 4)
- [ ] **Warning system for unsafe casts** (moved to blocker #9)
- [ ] **Integration Tests**
  - [ ] Parser + type checker integration tests
  - [ ] Error message quality tests (miette formatting, span accuracy)
  - [ ] Multi-error reporting tests
- [ ] **Test organization** (low priority, nice-to-have)
  - [ ] Organize type system tests into separate modules by category

## Phase 2: Language Features

**NOTE: Phase 2 CANNOT BEGIN until all Phase 2 Blockers (listed above in Type System Core section) are complete. These are fundamental language features from the specification including error handling (guard/propagate/errors), multiple return values, standard library built-ins, generic constraints, and member access. All items below require these foundational features to be functional.**

### ⏳ Function System (Name: function-system-plan.md)

**NOTE: This phase requires Type System Core completion AND Phase 2 Blockers #1 (Error Handling), #2 (Multiple Returns), #3 (Standard Library), and #4 (Generic Constraints) before proceeding.**

- [x] Function declaration and definition parsing
- [x] Parameter and return type handling
- [x] Lambda expressions (f(): type => ...)
- [x] Lambda body normalization (expression vs block)
- [x] Type checking for lambda bodies
- [ ] Error type declarations in function signatures (see Phase 2 Blocker #1)
- [ ] Multiple return type support (see Phase 2 Blocker #2)
- [ ] Function call resolution (basic call type checking complete; advanced resolution pending)
- [ ] Entry point validation (single entry keyword)
- [ ] Scope management for parameters and local variables (basic implementation complete)
- [ ] Integration with type system for generic functions (requires Blocker #4)
- [ ] Hot-reload metadata propagation for functions
- [x] Comprehensive unit tests for parsing
- [x] Type checking tests for lambdas
- [ ] Integration tests with full type inference
- [ ] Integration tests with error handling and multiple returns
- [ ] Documentation for all function system code
- [ ] Lint and test compliance before commit

### ☐ Variable System (Name: variable-system-plan.md)

**NOTE: This phase requires Type System Core completion before proceeding.**

- [x] Let bindings parsing (immutable by default)
- [x] Mutable variable parsing
- [x] Type annotation parsing
- [x] Let statement type checking
- [x] Let declaration type checking (module-level)
- [x] Scope management for variables
- [x] Variable shadowing in nested scopes
- [x] Type inference for let bindings
- [ ] Assignment to mutable variables validation
- [ ] Mutation of immutable variables error detection
- [ ] Unused variable warnings

### ☐ Control Flow (Name: control-flow-plan.md)

**NOTE: This phase requires Type System Core completion AND Phase 2 Blocker #5 (If Expression Semantics) before proceeding.**

- [x] If expressions parsing (Rust-style)
- [x] For loops parsing with iterators
- [x] While loops parsing
- [x] Loop statements parsing
- [x] Break/continue statement parsing
- [x] Break/continue with labeled values parsing (break label: value1, label2: value2)
- [x] Return statement parsing
- [x] Type checking for if statements (boolean condition enforcement)
- [x] Type checking for for loops (array iteration)
- [x] Type checking for while loops (boolean condition)
- [x] Type checking for loop statements
- [x] Return statement validation with expected return type
- [ ] If expression value-returning semantics (see Phase 2 Blocker #5)
- [ ] Exhaustiveness checking for if expressions
- [ ] Control flow analysis for unreachable code
- [ ] Type narrowing in conditional branches

### ☐ Arithmetic & Logic (Name: arithmetic-logic-plan.md)

**NOTE: This phase requires Type System Core completion AND Phase 2 Blockers #7 (Overflow Detection) and #8 (Division by Zero) before proceeding.**

- [x] Basic operators parsing (+, -, *, /, ^, %)
- [x] Comparison operators parsing (<, <=, >, >=, is, is not)
- [x] Boolean operators parsing (and, or, not, xor)
- [x] Bitwise operators parsing (band, bor, bxor, bnot, bshl, bshr, bushr)
- [x] Type checking for arithmetic operators (same-type requirement)
- [x] Type checking for comparison operators (numeric types only)
- [x] Type checking for boolean operators (boolean types only)
- [x] Type checking for bitwise operators (integer types only)
- [x] Cross-type comparison prohibition enforcement
- [ ] Compile-time overflow detection for constants (see Phase 2 Blocker #7)
- [ ] Division by zero detection (compile-time for constants) (see Phase 2 Blocker #8)
- [ ] Arithmetic overflow handling in code generation (debug trap, release wrap)
- [ ] Checked/wrapping/saturating arithmetic variants (see Phase 2 Blocker #7)
- [ ] Bitwise shift bounds checking (negative and out-of-range counts)
- [ ] Masked/wrapping bitwise shift variants

## Phase 3: Advanced Type Features

### ☐ ADT Implementation (Name: adt-implementation-plan.md)

**NOTE: This phase requires Type System Core to be complete AND Phase 2 Blocker #6 (Member Access) for field access support.**

- [x] Sum type parsing (enum-like with variants)
- [x] Product type parsing (struct-like)
- [x] Type declaration parsing (type keyword)
- [x] ADT validation (basic structure validation)
- [ ] Pattern matching parsing
- [ ] Pattern matching type checking
- [ ] Pattern exhaustiveness checking
- [ ] Generic ADT support (instantiation) (requires Phase 2 Blocker #4)
- [ ] ADT constructor type checking
- [ ] Field access type checking (requires Phase 2 Blocker #6 - member access)
- [ ] HasField constraint implementation (currently deferred)

### ☐ Array & Collection Support (Name: collections-plan.md)

**NOTE: This phase requires Type System Core and ADT Implementation to be complete.**

- [x] Array type parsing ([T])
- [x] Array literal parsing ([1, 2, 3])
- [x] Array index access parsing (arr[0])
- [x] Type checking for array literals (element type consistency)
- [x] Type checking for array access (index must be integer)
- [x] String type support
- [x] String literal parsing
- [x] String interpolation parsing
- [x] String interpolation type checking
- [ ] Array methods and operations
- [ ] String manipulation methods
- [ ] Iterator trait/interface
- [ ] Collection iteration in for loops (currently only arrays)
- [ ] Memory management for collections

### ☐ Generic System (Name: generics-plan.md)

**NOTE: This phase requires Type System Core completion AND Phase 2 Blocker #4 (Generic Constraints) before proceeding.**

- [x] Generic type parameter parsing (Type\<T\>)
- [x] Generic type with multiple parameters parsing (Map\<K, V\>)
- [x] Generic function type parsing
- [x] Generic type representation in CoreType
- [x] Generic lambda expressions parsing
- [x] Type variable infrastructure (TypeVar)
- [ ] Generic function definitions with constraints (see Phase 2 Blocker #4)
- [ ] Generic type parameter bounds/constraints (see Phase 2 Blocker #4)
- [ ] Type parameter inference at call sites (see Phase 2 Blocker #4)
- [ ] Concrete type argument validation (see Phase 2 Blocker #4)
- [ ] Monomorphization for code generation (Phase 5)
- [ ] Generic ADT instantiation (requires Phase 3 ADT completion)

## Phase 4: Module System

### ☐ Import/Export System (Name: module-system-plan.md)

**NOTE: This phase requires Type System Core to be complete.**

- [x] Public keyword parsing for exports
- [x] Import statement parsing (single and multiple items)
- [x] Local file imports parsing (./path)
- [x] Type imports parsing (.types files)
- [x] Import aliasing parsing (as keyword)
- [ ] Standard library imports resolution
- [ ] Package imports resolution (@scope/name)
- [ ] Import path validation
- [ ] Export validation (no duplicate exports)
- [ ] Type checking for imported symbols
- [ ] Dependency resolution
- [ ] Circular dependency detection
- [ ] Module interface generation

### ☐ Module Validation (Name: module-validation-plan.md)

- [ ] Circular dependency detection
- [ ] Name clash resolution
- [ ] Symbol visibility rules
- [ ] Module interface generation
- [ ] Cross-module type checking

## Phase 5: Code Generation

### ☐ LLVM Backend Setup (Name: llvm-backend-plan.md)

- [ ] LLVM integration
- [ ] Target platform support
- [ ] Code generation for basic expressions
- [ ] Function compilation
- [ ] Memory management

### ☐ Runtime System (Name: runtime-system-plan.md)

- [ ] Runtime library foundation
- [ ] Memory allocator
- [ ] Garbage collection (if needed)
- [ ] Standard library implementation
- [ ] Error handling runtime

### ☐ Optimization (Name: optimization-plan.md)

- [ ] Basic optimizations
- [ ] Dead code elimination
- [ ] Constant folding
- [ ] Inline expansion
- [ ] Loop optimizations

## Phase 6: Hot Reloading System

### ☐ Hot Reload Infrastructure (Name: hot-reload-infrastructure-plan.md)

- [ ] Dynamic library compilation
- [ ] ABI signature generation
- [ ] Version management system
- [ ] Host process framework
- [ ] Module hot-swap mechanism

### ☐ Change Detection (Name: change-detection-plan.md)

- [ ] File watching system
- [ ] Build graph analysis
- [ ] ABI compatibility checking
- [ ] Hot vs restart classification
- [ ] Incremental compilation

### ☐ Hot Reload Safety (Name: hot-reload-safety-plan.md)

- [ ] ABI guard implementation
- [ ] Automatic fallback restart
- [ ] State preservation
- [ ] Error recovery
- [ ] Testing framework for hot reload

## Phase 7: Developer Experience

### ☐ Error Reporting (Name: error-reporting-plan.md)

- [ ] Miette integration for beautiful errors
- [ ] Source location tracking
- [ ] Helpful error messages
- [ ] Suggestion system
- [ ] Multi-error reporting

### ☐ Documentation System (Name: documentation-plan.md)

- [x] Doc comment parsing (## Description: ... ##)
- [x] Doc comment preservation in AST
- [x] Documentation attribute parsing (@description, @param, @returns, @example)
- [ ] Documentation generation from code
- [ ] API documentation generation
- [ ] Examples and tutorials
- [ ] Language reference generation

### ☐ Build System (Name: build-system-plan.md)

- [ ] Project configuration
- [ ] Dependency management
- [ ] Build caching
- [ ] Incremental builds
- [ ] Cross-compilation support

## Phase 8: Standard Library

### ☐ Core Library (Name: core-library-plan.md)

- [ ] Basic data types
- [ ] String operations
- [ ] Math functions
- [ ] I/O operations
- [ ] File system access

### ☐ Collections Library (Name: collections-library-plan.md)

- [ ] Array operations
- [ ] Hash maps
- [ ] Sets
- [ ] Lists
- [ ] Iterators

### ☐ System Library (Name: system-library-plan.md)

- [ ] Operating system interfaces
- [ ] Network operations
- [ ] Threading support
- [ ] Process management
- [ ] Environment access

## Phase 9: Testing & Quality

### ☐ Test Framework (Name: test-framework-plan.md)

- [ ] Unit testing support
- [ ] Integration testing
- [ ] Property-based testing
- [ ] Benchmark testing
- [ ] Coverage reporting

### ☐ Language Server (Name: language-server-plan.md)

- [ ] LSP implementation
- [ ] Syntax highlighting
- [ ] Auto-completion
- [ ] Error reporting
- [ ] Refactoring support

### ☐ Formatter (Name: formatter-plan.md)

- [ ] Code formatting rules
- [ ] Whitespace enforcement
- [ ] Style consistency
- [ ] Editor integration
- [ ] Configuration options

## Phase 10: Production Readiness

### ☐ Performance Optimization (Name: performance-plan.md)

- [ ] Compile time optimization
- [ ] Runtime performance
- [ ] Memory usage optimization
- [ ] Hot reload performance
- [ ] Benchmark suite

### ☐ Platform Support (Name: platform-support-plan.md)

- [ ] Windows support
- [ ] macOS support
- [ ] Linux support
- [ ] Cross-compilation
- [ ] Package distribution

### ☐ Ecosystem (Name: ecosystem-plan.md)

- [ ] Package manager
- [ ] Registry system
- [ ] Community tools
- [ ] IDE plugins
- [ ] Documentation hosting
- [ ] Compiler "Help" commands work to get command clarifications

---

## Architectural Decisions & Dependencies

### Type System Dependencies

- **Phase 2 Blocker**: All language features require functional type checking
- **Hot Reload Planning**: AST and type system must preserve metadata for ABI signature generation
- **LLVM Backend Planning**: Type system design must support code generation requirements
- **Error Handling**: Consistent miette integration across all modules (lexer, parser, type system)

### Hot Reload Architecture Requirements

All phases must consider hot reload compatibility:

- **Symbol Tables**: Must support ABI signature generation and change detection
- **AST Metadata**: Must preserve all information needed for incremental compilation
- **Memory Layout**: Type system must plan for cross-module state preservation
- **Change Detection**: Build graph analysis requires dependency tracking from Phase 1

### Code Generation Preparation

- **Type Information Preservation**: Must survive through compilation pipeline
- **Memory Management Strategy**: Plan for LLVM backend memory allocation
- **Cross-compilation Support**: no_std compatibility maintained in core modules

## Quality & Testing Standards

### Error Handling Standards

- **Consistent Patterns**: All modules use miette for beautiful error reporting
- **Source Location Preservation**: All AST nodes maintain span information
- **Multiple Error Collection**: Support for reporting multiple errors simultaneously
- **Recovery Strategies**: Graceful degradation and continued processing after errors

### Test Coverage Requirements

- **Test-Driven Development**: Red-green-refactor for all new features
- **Integration Testing**: Cross-module compatibility validation
- **Hot Reload Testing**: Framework for testing hot reload scenarios (Phase 6)
- **Performance Benchmarks**: Establish baselines for optimization tracking

### Development Workflow

- **Linting First**: All code must pass strict linting before commits
- **No Shortcuts**: `--no-verify` is never allowed in git commits
- **Documentation**: Comprehensive inline documentation for future maintainers
- **Architectural Decisions**: Document rationale for infrastructure choices
- **Phase Dependencies**: Each phase builds upon the previous ones - no exceptions
- **Type System Priority**: Type System Core completion is required before Phase 2 can begin
