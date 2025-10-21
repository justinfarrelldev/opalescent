# Type System Core Implementation Plan

This document outlines the detailed implementation plan for the Opalescent language type system core.

## Overview

The type system provides static type checking, type inference, and type safety guarantees. It must handle basic types, generic types, algebraic data types (ADTs), function types, and ensure type compatibility for all operations including casts, arithmetic, and function calls.

## Basic Type Representation

### ✅ Core Type Infrastructure

- [x] Base Type enum/trait for all type representations (CoreType enum)
- [x] Type context/environment for managing type definitions (TypeEnvironment)
- [x] Basic type validation and compatibility checking
- [x] Source location preservation preparation (TypeError with structured error info)
- [x] Type unification algorithms for inference
- [x] Type substitution for generics
- [x] Complete source location integration

#### Complete Source Location Integration (✅ Complete)

- [x] Extend `TypeConstraint` variants to capture originating `Span` data for all operands
- [x] Propagate spans when emitting constraints in expression, statement, and lambda checkers
- [x] Enhance unification and constraint solving to surface spans within `TypeError` diagnostics
- [x] Replace remaining `TypeError::unknown_span()` usages inside the checker with real span data
- [x] Update pattern-match validation helpers to accept span-aware inputs
- [x] Add regression tests proving span propagation for constraint failures and pattern mismatches

### ✅ Primitive Types

- [x] Core integer types (int32, int64, uint32, uint64)
- [x] Extended integer types (int8, int16, uint8, uint16)
- [x] Floating point types (float32, float64)
- [x] Boolean type (boolean)
- [x] String type (string)
- [x] Unit/void type (unit)
- [ ] Character type (char) if needed

### ✅ Composite Types

- [x] Array types ([T], fixed-size arrays)
- [x] Function types ((param_types) -> return_type)
- [x] Generic types (Type\<T\>) with type arguments
- [ ] Tuple types (if supported)
- [ ] Reference types (for memory management)

### ☐ Type Aliases

- [ ] Type alias declarations (type NewName = ExistingType)
- [ ] Type alias resolution and substitution
- [ ] Recursive type alias detection and handling
- [ ] Type alias scoping rules

## Generic Type Support

### ⏳ Generic Type Parameters

- [x] Type parameter representation (TypeVar struct with id and name)
- [x] Type parameter bounds/constraints (basic infrastructure in TypeEnvironment)
- [ ] Variance annotations (covariant, contravariant, invariant)
- [ ] Higher-kinded types (Type\<Type\<T\>\>)

### ⏳ Generic Type Instantiation

- [x] Generic type representation (Generic variant in CoreType)
- [x] Type argument storage (type_args field)
- [x] Generic type unification support
- [ ] Type argument inference at call sites
- [ ] Explicit type argument specification
- [ ] Generic type checking and validation
- [ ] Monomorphization preparation

### ☐ Generic Functions

- [ ] Generic function declarations
- [ ] Type parameter inference for function calls
- [ ] Generic function type checking
- [ ] Constraint satisfaction checking

### ☐ Generic ADTs

- [ ] Generic struct/enum declarations
- [ ] Generic ADT instantiation
- [ ] Generic field access type checking
- [ ] Generic pattern matching support

## Type Inference Engine

### ⏳ Hindley-Milner Type Inference

- [x] Type variable generation and management (TypeVar struct)
- [x] Unification algorithm implementation (complete with occurs check)
- [x] Occurs check for infinite types
- [x] Substitution system for type variables
- [x] Constraint collection during AST traversal
- [ ] Principal type inference

### ☐ Local Type Inference

- [ ] Variable declaration type inference (let x = expr)
- [ ] Function return type inference
- [ ] Literal type inference
- [ ] Expression type inference

### ☐ Flow-Sensitive Analysis

- [ ] Control flow type refinement
- [ ] Type narrowing in conditional branches
- [ ] Dead code analysis via type system
- [ ] Exhaustiveness checking for pattern matching

## Type Checking Framework

### ✅ Expression Type Checking

- [x] Literal expression type checking
- [x] Identifier type lookup and resolution
- [x] Binary operation type compatibility
- [x] Unary operation type checking
- [x] Function call type checking (parameter/argument matching)
- [x] Array access type checking
- [x] Cast expression validation

### ✅ Statement Type Checking

- [x] Variable declaration type checking
- [x] Assignment type compatibility
- [x] Return statement type checking
- [x] Control flow type checking (if, loops)
- [x] Block statement scoping

### ☐ Declaration Type Checking

- [x] Function declaration type validation
- [ ] Type declaration validation (no circular dependencies)
- [ ] Import statement type resolution
- [ ] Public/entry declaration type checking

### ☐ ADT Type Checking

- [ ] Struct field type checking
- [ ] Enum variant type checking
- [ ] Pattern matching exhaustiveness
- [ ] Constructor type checking

## Cast Validation and Safety

### ☐ Safe Cast Validation

- [ ] Widening casts (int8 -> int32) validation
- [ ] Numeric type compatibility checking
- [ ] Lossy cast detection and prevention
- [ ] Compile-time constant cast validation

### ☐ Explicit Cast Requirements

- [ ] Signed/unsigned conversion requirements
- [ ] Float/integer conversion checking
- [ ] Out-of-range literal detection
- [ ] Runtime trap requirements for unsafe casts

### ☐ Cast Error Reporting

- [ ] Clear error messages for invalid casts
- [ ] Suggestions for alternative conversion methods
- [ ] Safety warnings for potentially unsafe casts
- [ ] Documentation of cast behavior

## Arithmetic Type Safety

### ☐ Binary Operation Type Checking

- [ ] Same-type requirements for arithmetic operations
- [ ] Cross-type comparison prohibition
- [ ] Bitwise operation type restrictions (integers only)
- [ ] Logical operation type checking (boolean only)

### ☐ Overflow and Safety Checking

- [ ] Compile-time overflow detection for constants
- [ ] Runtime overflow trap generation (debug mode)
- [ ] Division by zero detection
- [ ] Shift operation bounds checking

### ☐ Type Promotion Rules

- [ ] No implicit type promotion
- [ ] Explicit cast requirements
- [ ] Error messages suggesting proper casts
- [ ] Type compatibility matrix

## Advanced Type Features

### ☐ Sum Types (Enums)

- [ ] Enum variant type checking
- [ ] Pattern matching type checking
- [ ] Exhaustiveness analysis
- [ ] Tagged union representation

### ☐ Product Types (Structs)

- [ ] Struct field type checking
- [ ] Constructor validation
- [ ] Field access type checking
- [ ] Struct update syntax validation

### ☐ Function Types

- [ ] Function signature type checking
- [ ] Higher-order function support
- [ ] Closure type inference
- [ ] Function pointer type checking

### ☐ Module System Integration

- [ ] Type visibility and scoping
- [ ] Cross-module type checking
- [ ] Type import/export validation
- [ ] Circular dependency detection

## Error Handling

### ☐ Type Error Reporting

- [ ] Clear, actionable error messages
- [ ] Type mismatch visualization
- [ ] Suggestion system for fixes
- [ ] Multiple error collection and reporting

### ☐ Type Error Recovery

- [ ] Graceful degradation for invalid types
- [ ] Continued checking after errors
- [ ] Error suppression to avoid cascading errors
- [ ] Recovery strategies for invalid type expressions

### ☐ Integration with miette

- [ ] Beautiful type error formatting
- [ ] Source code highlighting
- [ ] Multi-span error support
- [ ] Help text and suggestions

## Testing Strategy

### ☐ Unit Tests

- [ ] Test each type checking component individually
- [ ] Test type inference algorithms
- [ ] Test cast validation logic
- [ ] Test error reporting and recovery
- [ ] Test generic type instantiation

### ☐ Integration Tests

- [ ] Type check complete programs
- [ ] Test interaction between type system components
- [ ] Test error propagation through type system
- [ ] Test performance on large programs

### ☐ Property-Based Tests

- [ ] Type preservation properties
- [ ] Type inference soundness and completeness
- [ ] Type substitution correctness
- [ ] Unification algorithm properties

## Implementation Strategy

### ✅ Phase 1: Basic Infrastructure

- [x] Define core type representation (CoreType enum)
- [x] Implement basic type checking for primitives (all int/uint/float types)
- [x] Extended integer and float type support (int8, int16, uint8, uint16)
- [x] Set up type environment and context (TypeEnvironment struct)
- [x] Basic error reporting framework (TypeError with miette integration)
- [x] TypeChecker infrastructure with environment management
- [x] Core type compatibility checking
- [x] AST Type to CoreType conversion for basic types
- [x] Type variable system (TypeVar) for inference
- [x] Substitution system for type variables
- [x] Full unification algorithm with occurs check
- [x] Array, Function, and Generic type support in CoreType
- [x] Comprehensive test suite (141 tests covering all functionality)
- [x] Hierarchical scope management (ScopeId, Scope struct, parent chain lookup)
- [x] Span threading for error reporting (lookup_type, validate_type_name, fresh_type_var with Span)
- [x] Pattern matching consistency in unification algorithm
- [ ] Integration with parser for type annotations
- [ ] Complete primitive type support (char type if needed)
- [ ] Add proper type context management

### ⏳ Phase 2: Type Inference

**Phase 2 Preparation (Completed):**

- [x] Scope Management Infrastructure
  - [x] ScopeId struct for unique scope identification
  - [x] Scope struct with parent tracking and symbol map
  - [x] Hierarchical SymbolTable with scope stack
  - [x] enter_scope() / exit_scope() for nested scopes
  - [x] lookup() with parent chain traversal
  - [x] lookup_local() for current scope only
  - [x] register_in_scope() for specific scope registration
  - [x] Comprehensive scope management tests (4 tests)
  
- [x] Span Threading for Error Reporting
  - [x] Updated lookup_type(name, span) signature
  - [x] Updated validate_type_name(name, core_type, span) signature
  - [x] Updated fresh_type_var(name, span) signature
  - [x] Updated fresh_type_var_auto(span) signature
  - [x] Updated validate_adt_type to use field.span
  - [x] test_span() helper for consistent test spans
  - [x] All test call sites updated with span parameters
  - [x] Pattern matching consistency fixes in unification

**Phase 2 Implementation (In Progress):**

- [x] AST Integration Methods (8-12 hours) - NEXT PRIORITY
  - [x] Implement type_check_expr(expr: Expr, span: Span) result CoreType or TypeError
  - [x] Implement type_check_stmt(stmt: Stmt) result unit or TypeError
  - [x] Implement type_check_decl(decl: Decl) result unit or TypeError
  - [x] Implement type_check_program(program: Program) result unit or errors
  - [x] Add constraint collection during expr traversal
  - [x] Use fresh_type_var_auto() for implicit type variables
  - [x] Use symbol_table for variable resolution

- [x] Program Checking Sprint
  - [x] Pre-register top-level declarations before body checking
  - [x] Type check function declarations including parameters, body, and return types
  - [x] Type check top-level let declarations with optional annotations
  - [x] Validate type declarations for ADT correctness and register signatures
  - [x] Provide import declaration handling stubs with clear Phase 4 notes
  - [x] Collect multi-error results during program checking
  - [x] Add failing tests for declaration checking before implementation (≥3)
  - [x] Implement functionality and make tests pass
  - [x] Update plan checkboxes and documentation after verification

- [x] Constraint Solver Implementation (12-16 hours)
  - [x] Replace solve_constraints() stub with actual algorithm
  - [x] Unify all collected constraints
  - [x] Generate final substitution
  - [x] Apply substitution to get concrete types
  - [x] Implement Callable constraint handling
  - [x] Add ArityMismatch and NotCallable error variants
  - [x] Add 3 tests for Callable constraint scenarios
  - [ ] HasField constraint handling (deferred to Phase 3 - requires ADT Product types)

- [x] Cast Validation (4-6 hours)
  - [x] Define safe casts (widening: int32 -> int64, int32 -> float64)
  - [x] Define unsafe casts (narrowing: int64 -> int32, float64 -> int32)
  - [x] Implement is_safe_cast(from: CoreType, to: CoreType) result bool
  - [x] Add overflow detection strategy documentation

- [ ] Lambda and Closure Sprint
  - [ ] Add tests covering lambda expression type checking (expression + block bodies)
  - [ ] Implement parameter scope handling within lambdas
  - [ ] Enforce return type compatibility for lambdas
  - [ ] Document future capture analysis requirements in code comments
  - [ ] Ensure symbol tables unwind correctly after lambda scopes

- [ ] Integration Tests (6-8 hours)
  - [ ] Create tests/type_checking_integration.rs
  - [ ] Parser + type checker integration tests
  - [ ] Error message quality tests (miette formatting)
  - [ ] Multi-error collection tests

**Original Phase 2 Goals:**

- [ ] Implement Hindley-Milner inference
- [ ] Add constraint collection and solving
- [ ] Local type inference for let bindings
- [ ] Function type inference

### ☐ Phase 3: Advanced Types

- [ ] Generic type support
- [ ] ADT type checking
- [ ] Function type checking
- [ ] Cast validation

### ☐ Phase 4: Safety and Validation

- [ ] Arithmetic type safety
- [ ] Overflow checking
- [ ] Cast safety validation
- [ ] Comprehensive error reporting

### ☐ Phase 5: Integration and Optimization

- [ ] Parser integration
- [ ] Error reporting integration
- [ ] Performance optimization
- [ ] Documentation and testing

## Dependencies

### ☐ Required Crates

- [ ] Integration with AST module (uses crate::ast)
- [ ] miette for error reporting (Diagnostic trait)
- [ ] thiserror for error definitions (Error trait)
- [ ] Potential for petgraph for type dependency graphs

### ☐ Internal Dependencies

- [ ] AST node types from ast module
- [ ] Token/span types for error locations
- [ ] Parser integration for type annotation parsing
- [ ] Error module integration

## Validation Requirements

### ☐ Language Specification Compliance

- [ ] Follows math.md requirements for arithmetic type safety
- [ ] Implements cast requirements from overview.md
- [ ] Supports ADT requirements from types_example.types.op
- [ ] Maintains compatibility with module system

### ☐ Performance Requirements

- [ ] Type checking should be fast enough for interactive development
- [ ] Memory usage should be reasonable for large programs
- [ ] Error reporting should not significantly slow down compilation
- [ ] Generic instantiation should be efficient

### ☐ Quality Requirements

- [ ] All type errors must be helpful and actionable
- [ ] Type system must be sound (no runtime type errors)
- [ ] Type inference must be predictable and intuitive
- [ ] Documentation must be comprehensive

---

## Implementation Notes

- Focus on safety over convenience - explicit is better than implicit
- Error messages should guide users toward correct solutions
- Type inference should feel natural but never surprising
- Generic system should be powerful but not complex
- Cast system should prevent runtime errors through compile-time checking
- Integration with parser and error reporting should be seamless

## Success Criteria

1. All primitive types work correctly with proper type checking
2. Generic types work with proper inference and checking
3. ADTs (structs and enums) work with pattern matching
4. Cast system prevents runtime type errors
5. Arithmetic operations are type-safe
6. Error messages are clear and helpful
7. Performance is acceptable for interactive development
8. All language specification requirements are met
