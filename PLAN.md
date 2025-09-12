# Opalescent Language Implementation Plan

This document outlines the comprehensive plan for implementing the Opalescent programming language, a compiled, statically and strongly typed language with hot reloading capabilities.

## Phase 1: Foundation & Core Infrastructure

### ✅ Project Setup
- [x] Initialize Rust project structure
- [x] Set up cargo-make configuration
- [x] Configure linting and testing infrastructure

### ✅ Lexical Analysis (Name: lexer-plan.md)
- [x] Implement tokenization for keywords, identifiers, literals
- [x] Handle operators and punctuation
- [x] Support string interpolation syntax
- [x] Whitespace consistency checking (spaces vs tabs)
- [x] Comment handling (single # and multi-line ##)

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
- [x] Integration Tests (comprehensive test coverage, 89 tests passing)
- [x] Error handling validation

### ⏳ Type System Core (Name: type-system-core-plan.md)
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
- [x] Comprehensive test suite (31 tests passing)
- [ ] **CRITICAL FOR PHASE 2:** Generic type support (runtime instantiation)
- [ ] **CRITICAL FOR PHASE 2:** Type inference engine (constraint collection and solving)
- [ ] **CRITICAL FOR PHASE 2:** Complete type checking framework
  - [ ] Expression type checking (literals, identifiers, binary/unary ops, calls, casts)
  - [ ] Statement type checking (let bindings, assignments, returns, blocks)
  - [ ] Declaration type checking (functions, types, imports)
  - [ ] Scope management and variable resolution
  - [ ] Type checking integration with parser AST
- [ ] **CRITICAL FOR PHASE 2:** Cast validation and safety
  - [ ] Safe cast checking (widening vs narrowing)
  - [ ] Runtime cast validation planning
  - [ ] Arithmetic overflow handling strategy
  - [ ] Integration with error handling

## Phase 2: Language Features

**NOTE: Phase 2 CANNOT BEGIN until Type System Core is 100% complete. All items below require functional type checking, type inference, and cast validation.**

### ☐ Function System (Name: function-system-plan.md)

- [ ] Function declaration and definition parsing
- [ ] Parameter and return type handling
- [ ] Lambda expressions (f(): type => ...)
- [ ] Function call resolution
- [ ] Entry point validation (single entry keyword)

### ☐ Variable System (Name: variable-system-plan.md)

- [ ] Let bindings (immutable by default)
- [ ] Mutable variables
- [ ] Scope management
- [ ] Variable shadowing rules
- [ ] Type inference for variables

### ☐ Control Flow (Name: control-flow-plan.md)

- [ ] If expressions (Rust-style)
- [ ] For loops with iterators
- [ ] While loops
- [ ] Return statement validation
- [ ] Break/continue semantics

### ☐ Arithmetic & Logic (Name: arithmetic-logic-plan.md)

- [ ] Basic operators (+, -, *, /, ^)
- [ ] Comparison operators with type safety
- [ ] Boolean operators (and, or, not, xor)
- [ ] Bitwise operators (band, bor, bxor, bnot, bshl, bshr, bushr)
- [ ] Overflow handling (debug vs release)
- [ ] Division by zero protection

## Phase 3: Advanced Type Features

### ☐ ADT Implementation (Name: adt-implementation-plan.md)

- [ ] Sum types (enum-like with variants)
- [ ] Product types (struct-like)
- [ ] Pattern matching
- [ ] Generic ADT support
- [ ] Type validation and checking

### ☐ Array & Collection Support (Name: collections-plan.md)

- [ ] Array types and literals
- [ ] String handling and interpolation
- [ ] Collection operations
- [ ] Iterator support
- [ ] Memory management for collections

### ☐ Generic System (Name: generics-plan.md)

- [ ] Generic function definitions
- [ ] Generic type constraints
- [ ] Type parameter inference
- [ ] Monomorphization
- [ ] Generic ADT instantiation

## Phase 4: Module System

### ☐ Import/Export System (Name: module-system-plan.md)
- [ ] Public keyword for exports
- [ ] Import statement parsing
- [ ] Local file imports (./path)
- [ ] Standard library imports
- [ ] Package imports (@scope/name)
- [ ] Type imports (.types files)
- [ ] Import aliasing
- [ ] Dependency resolution

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
- [ ] Doc comment parsing
- [ ] Documentation generation
- [ ] API documentation
- [ ] Examples and tutorials
- [ ] Language reference

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

## Current Status

The project is in **Phase 1**, and the lexical analysis system has been completed. The **Type System Core** is partially complete but missing critical components required for Phase 2.

**CRITICAL BLOCKER**: Phase 2 cannot begin until the Type System Core is 100% complete, specifically:
- Generic type runtime instantiation
- Type inference engine (constraint collection and solving)
- Complete type checking framework
- Cast validation and safety

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

## Notes

- Each phase builds upon the previous ones - **no exceptions**
- Test-driven development should be used throughout
- All code must pass linting before commits
- Hot reloading is a key differentiator and should be prioritized
- Safety and type checking are more important than compile speed
- Developer experience is paramount
- **Type System Core completion is the immediate priority**
