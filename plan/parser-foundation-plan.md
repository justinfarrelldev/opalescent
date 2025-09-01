# Parser Foundation Implementation Plan

This document outlines the detailed implementation plan for the Opalescent language parser foundation.

## Overview

The parser takes the stream of tokens from the lexer and builds an Abstract Syntax Tree (AST) representing the structure of the program. It implements a recursive descent parser with proper error recovery and reporting.

## AST Node Definitions

### ✅ Core AST Infrastructure
- [x] Base AST node trait/enum
- [x] Source location preservation
- [x] Visitor pattern support
- [x] Display implementation for debugging

### ✅ Expression AST Nodes
- [x] Literal expressions (integer, float, string, boolean)
- [x] Identifier expressions
- [x] Binary operations (arithmetic, comparison, logical, bitwise)
- [x] Unary operations (negation, not, bitwise not)
- [x] Function call expressions
- [x] Array/collection access expressions
- [x] Cast expressions (expr as Type)
- [ ] Type checking expressions (type_of)
- [ ] String interpolation expressions

### ✅ Statement AST Nodes
- [x] Let bindings (immutable variables)
- [x] Mutable variable declarations
- [ ] Assignment statements
- [x] Return statements
- [x] Expression statements
- [x] Block statements

### ✅ Declaration AST Nodes
- [x] Function declarations
- [ ] Type declarations
- [ ] Import declarations
- [x] Public/entry declarations

### ☐ Control Flow AST Nodes
- [ ] If expressions/statements
- [ ] For loop statements
- [ ] While loop statements
- [ ] Break/continue statements

### ☐ Type AST Nodes
- [x] Basic types (int32, string, etc.)
- [ ] Generic types
- [x] Array types
- [ ] Function types
- [ ] Custom types

## Parser Implementation

### ✅ Core Parser Structure
- [x] Parser struct with token stream
- [x] Current token tracking
- [x] Lookahead functionality
- [x] Error collection system
- [x] Recovery strategies

### ✅ Expression Parsing
- [x] Pratt parser for operator precedence
- [x] Primary expression parsing
- [x] Binary expression parsing
- [x] Unary expression parsing
- [x] Parenthesized expressions
- [x] Function call parsing

### ✅ Statement Parsing
- [x] Statement dispatcher
- [x] Variable declaration parsing
- [ ] Assignment parsing
- [x] Return statement parsing
- [x] Block parsing with scope

### ✅ Declaration Parsing
- [x] Function declaration parsing
- [ ] Type declaration parsing
- [ ] Import statement parsing
- [ ] Visibility modifier parsing

### ☐ Type Parsing
- [ ] Basic type parsing
- [ ] Generic type parsing
- [ ] Array type parsing
- [ ] Function type parsing

## Error Handling

### ☐ Parse Errors
- [ ] Unexpected token errors
- [ ] Missing token errors (semicolons, braces, etc.)
- [ ] Invalid syntax errors
- [ ] Type annotation errors
- [ ] Visibility modifier errors

### ☐ Error Recovery
- [ ] Panic mode recovery
- [ ] Synchronization points
- [ ] Multiple error collection
- [ ] Context-aware error messages
- [ ] Suggestion system

### ☐ Error Reporting
- [ ] Miette integration for beautiful errors
- [ ] Source context highlighting
- [ ] Multi-span error support
- [ ] Helpful diagnostics and suggestions

## Advanced Features

### ☐ Operator Precedence
- [ ] Pratt parser implementation
- [ ] Precedence table for all operators
- [ ] Right-associative operators (^, =)
- [ ] Left-associative operators
- [ ] Non-associative operators

### ☐ String Interpolation
- [ ] Parse interpolated expressions
- [ ] Handle nested braces
- [ ] Type checking for interpolated values
- [ ] Escape sequence handling

### ☐ Comments and Documentation
- [ ] Preserve documentation comments
- [ ] Associate comments with AST nodes
- [ ] Extract API documentation
- [ ] Support for attribute-like comments

## Testing Strategy

### ☐ Unit Tests
- [ ] Test each AST node type
- [ ] Test expression parsing
- [ ] Test statement parsing
- [ ] Test declaration parsing
- [ ] Test error cases

### ☐ Integration Tests
- [ ] Parse example .op files
- [ ] Round-trip testing (parse -> print -> parse)
- [ ] Error recovery testing
- [ ] Performance testing

### ☐ Property-Based Tests
- [ ] Random expression generation
- [ ] Parse/unparse invariants
- [ ] Error handling properties

## Implementation Strategy

### ☐ Core Infrastructure
- [ ] AST node definitions
- [ ] Parser struct and basic methods
- [ ] Token consumption utilities
- [ ] Error handling framework

### ☐ Expression Parser
- [ ] Literal parsing
- [ ] Identifier parsing
- [ ] Binary operator parsing with precedence
- [ ] Unary operator parsing
- [ ] Parenthesized expressions

### ☐ Statement Parser
- [ ] Variable declarations
- [ ] Assignments
- [ ] Return statements
- [ ] Expression statements

### ☐ Declaration Parser
- [ ] Function declarations
- [ ] Import statements
- [ ] Type declarations

### ☐ Advanced Features
- [ ] String interpolation
- [ ] Complex type expressions
- [ ] Error recovery
- [ ] Performance optimization

## Dependencies

### ☐ Required Crates
- [ ] Integration with lexer module
- [ ] miette for error reporting
- [ ] Additional parsing utilities if needed

### ☐ Internal Dependencies
- [ ] Token types from lexer
- [ ] Error types from error module
- [ ] Source location types

## Validation

### ☐ Test Cases
- [ ] All example .op files parse correctly
- [ ] Error cases produce helpful messages
- [ ] AST structure is correct and complete
- [ ] Parser performance is acceptable

### ☐ Integration
- [ ] Works with lexer seamlessly
- [ ] Integrates with error reporting
- [ ] Ready for type checker integration
- [ ] Supports IDE features (syntax highlighting, completion)

---

## Implementation Order

1. Core AST node definitions and infrastructure
2. Basic expression parsing (literals, identifiers, binary ops)
3. Statement parsing (let bindings, assignments, returns)
4. Function declaration parsing
5. Error handling and recovery
6. Advanced expression features (calls, casts)
7. Control flow parsing (if, loops)
8. Type declaration parsing
9. Import/export parsing
10. String interpolation and advanced features

## Notes

- Focus on correctness and clear error messages
- Use test-driven development throughout
- Ensure AST preserves all source information
- Design for extensibility (new syntax features)
- Maintain compatibility with language specification
