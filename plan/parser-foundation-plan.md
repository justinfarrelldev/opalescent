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
- [x] Type checking expressions (type_of)
- [x] String interpolation expressions

### ✅ Statement AST Nodes

- [x] Let bindings (immutable variables)
- [x] Mutable variable declarations
- [x] Assignment statements
- [x] Return statements
- [x] Expression statements
- [x] Block statements

### ✅ Declaration AST Nodes

- [x] Function declarations
- [x] Type declarations (structs, enums, type aliases)
- [x] Import declarations (with named imports, type imports, aliases)
- [x] Public/entry declarations

### ✅ Control Flow AST Nodes

- [x] If expressions/statements
- [x] For loop statements
- [x] While loop statements
- [x] Loop statement (seen in language-spec/requirements/simple_quiz.op)
- [x] Break/continue statements
- [ ] Labeled break/continue with multiple return values (break label: value1, value2)

### ✅ Type AST Nodes

- [x] Basic types (int32, string, etc.)
- [x] Generic types
- [x] Array types
- [x] Function types
- [x] Custom types (TypeDef enum with struct/enum/alias variants)

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
- [x] Assignment parsing
- [x] Return statement parsing
- [x] Block parsing with scope

### ✅ Declaration Parsing

- [x] Function declaration parsing
- [x] Type declaration parsing (structs, enums, type aliases)
- [x] Import statement parsing (named imports, type imports, aliases)
- [x] Visibility modifier parsing (public, entry)

### ✅ Type Parsing

- [x] Basic type parsing (identifiers, built-in types)
- [x] Generic type parsing (Type\<T\>, Map\<K, V\>)
- [x] Array type parsing ([Type])
- [x] Function type parsing ((param_types) -> return_type)

## Error Handling

### ✅ Parse Errors

- [x] Unexpected token errors (UnexpectedToken variant)
- [x] Missing token errors (MissingToken variant)  
- [x] Invalid syntax errors (InvalidSyntax variant)
- [x] Unexpected EOF errors (UnexpectedEof variant)
- [x] Type annotation errors (handled within parse errors)
- [x] Visibility modifier errors (handled within parse errors)

### ✅ Error Recovery

- [x] Panic mode recovery (synchronize function)
- [x] Synchronization points (at declaration boundaries)
- [x] Multiple error collection (ParseErrors struct)
- [x] Context-aware error messages (with miette integration)
- [x] Error span highlighting (ParseError::span_from_token)

### ✅ Error Reporting

- [x] Miette integration for beautiful errors
- [x] Source context highlighting (SourceSpan in error types)
- [x] Multi-span error support (via miette)
- [x] Helpful diagnostics and suggestions (help text in error variants)

## Advanced Features

### ✅ Operator Precedence

- [x] Pratt parser implementation (parse_expression_with_precedence)
- [x] Precedence table for all operators (get_precedence function)
- [x] Right-associative operators (assignment)
- [x] Left-associative operators (arithmetic, comparison, logical)
- [x] Proper operator precedence handling

### ✅ String Interpolation

- [x] Parse interpolated expressions
- [x] Handle nested braces
- [x] StringPart enum for text and expression parts
- [x] Escape sequence handling for braces

### ☐ Comments and Documentation

- [x] Preserve documentation comments
- [ ] Associate comments with AST nodes
- [ ] Extract API documentation
- [ ] Support for attribute-like comments

## Testing Strategy

### ✅ Unit Tests

- [x] Test each AST node type (comprehensive test coverage)
- [x] Test expression parsing (literals, operators, precedence)
- [x] Test statement parsing (let, assignments, returns, blocks, control flow)
- [x] Test declaration parsing (functions, types, imports)
- [x] Test error cases (invalid syntax, missing tokens, etc.)

### ✅ Integration Tests

- [x] Parse complex programs (89 tests passing)
- [x] Round-trip testing capabilities (parse -> AST structure validation)
- [x] Error recovery testing (synchronization, multiple errors)
- [x] Import parsing integration tests

### ☐ Property-Based Tests

- [ ] Random expression generation
- [ ] Parse/unparse invariants
- [ ] Error handling properties

## Implementation Strategy

### ✅ Core Infrastructure

- [x] AST node definitions (comprehensive AST in ast.rs)
- [x] Parser struct and basic methods (Parser implementation)
- [x] Token consumption utilities (advance, consume, check, etc.)
- [x] Error handling framework (ParseError types, ParseErrors collection)

### ✅ Expression Parser

- [x] Literal parsing (integers, floats, strings, booleans)
- [x] Identifier parsing
- [x] Binary operator parsing with precedence (Pratt parser)
- [x] Unary operator parsing (-, !, ~)
- [x] Parenthesized expressions
- [x] Function call expressions
- [x] Array access expressions
- [x] Cast expressions (as operator)
- [x] Type checking expressions (type_of)

### ✅ Statement Parser

- [x] Variable declarations (let, mut)
- [x] Assignments (with type annotations)
- [x] Return statements
- [x] Expression statements
- [x] Block statements
- [x] Control flow (if, for, while, loop, break, continue)

### ✅ Declaration Parser

- [x] Function declarations (with parameters, return types)
- [x] Import statements (named, type, with aliases)
- [x] Type declarations (struct, enum, type alias)
- [x] Visibility modifiers (public, entry)

### ✅ Advanced Features

- [x] String interpolation parsing
- [x] Complex type expressions (generics, arrays, functions)
- [x] Error recovery (synchronization)
- [x] Comprehensive test coverage

## Dependencies

### ✅ Required Crates

- [x] Integration with lexer module (uses crate::lexer)
- [x] miette for error reporting (Diagnostic trait implemented)
- [x] thiserror for error definitions (Error trait derived)

### ✅ Internal Dependencies

- [x] Token types from lexer (TokenType, Token, Span)
- [x] Error types from error module (LexError for span conversion)  
- [x] Source location types (Span from token module)

## Validation

### ✅ Test Cases

- [x] All core parsing functionality tested (89 tests passing)
- [x] Error cases produce helpful messages (miette integration)
- [x] AST structure is correct and complete (comprehensive AST nodes)
- [x] Parser performance is acceptable (efficient recursive descent)

### ✅ Integration

- [x] Works with lexer seamlessly (TokenType integration)
- [x] Integrates with error reporting (miette Diagnostic trait)
- [x] Ready for type checker integration (complete AST)
- [x] Supports comprehensive language features

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
