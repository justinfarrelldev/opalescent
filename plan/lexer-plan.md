# Lexer Implementation Plan

This document outlines the detailed implementation plan for the Opalescent language lexer (tokenizer).

## Overview

The lexer is responsible for converting source code text into a stream of tokens that can be consumed by the parser. It must handle all Opalescent language constructs while maintaining strict whitespace consistency and providing excellent error reporting.

## Token Types

### ✅ Basic Token Structure
- [x] Token enum with type and value
- [x] Source location tracking (line, column, offset)
- [x] Span information for error reporting

### ✅ Literals
- [x] Integer literals (int8, int16, int32, int64)
- [x] Unsigned integer literals (uint8, uint16, uint32, uint64)
- [x] Float literals (float32, float64)
- [x] String literals with escape sequences
- [ ] String interpolation syntax ('Hello {world}')
- [ ] Character literals
- [x] Boolean literals (true, false)

### ✅ Identifiers and Keywords
- [x] Snake_case identifier validation
- [ ] PascalCase type identifier validation
- [x] Keywords: let, mutable, f, return, void, if, else, for, while, in, break, continue
- [x] Type keywords: int8, int16, int32, int64, uint8, uint16, uint32, uint64, float32, float64, string, boolean
- [x] Visibility keywords: public, entry
- [x] Import keywords: import, from, as, type
- [x] Type definition keywords: type

### ✅ Operators
- [x] Arithmetic: +, -, *, /, ^, %
- [x] Assignment: =
- [x] Comparison: <, <=, >, >=, is, is not
- [x] Logical: and, or, not, xor
- [x] Bitwise: band, bor, bxor, bnot, bshl, bshr, bushr
- [x] Cast operator: as
- [x] Type checking: type_of

### ✅ Punctuation
- [x] Parentheses: (, )
- [x] Square brackets: [, ]
- [x] Braces: {, }
- [x] Colon: :
- [x] Comma: ,
- [x] Arrow: =>
- [x] Dot: .
- [x] Hash: # (for comments)

### ✅ Comments
- [x] Single-line comments starting with #
- [x] Multi-line comments with ## ... ##
- [x] Doc comments (## Description: ... ##)
- [x] Nested comment handling
- [x] Comment preservation for documentation

### ✅ Whitespace Handling
- [x] Space vs tab detection
- [x] Consistent whitespace enforcement per file
- [ ] Indentation tracking
- [x] Line ending normalization
- [ ] Whitespace-sensitive parsing (Python-like indentation)

## Error Handling

### ✅ Lexical Errors
- [x] Invalid character sequences
- [x] Unterminated string literals
- [x] Invalid escape sequences
- [x] Mixed whitespace detection
- [x] Invalid number formats
- [x] Unclosed multi-line comments

### ✅ Error Recovery
- [x] Continue tokenizing after errors
- [x] Multiple error reporting
- [x] Helpful error messages with miette
- [ ] Suggestion system for common mistakes

## Advanced Features

### ☐ String Interpolation
- [ ] Parse string interpolation expressions
- [ ] Handle nested braces
- [ ] Type checking for interpolated expressions
- [ ] Escape sequence handling within interpolation

### ☐ Position Tracking
- [ ] Accurate line/column tracking
- [ ] UTF-8 character boundary handling
- [ ] Source map generation
- [ ] Span merging for multi-token constructs

### ☐ Lookahead and Context
- [ ] Multi-character operator recognition
- [ ] Context-sensitive tokenization
- [ ] Keyword vs identifier disambiguation
- [ ] Number literal type inference

## Implementation Strategy

### ✅ Core Lexer Structure
- [x] Input stream abstraction
- [x] Character iterator with lookahead
- [ ] Token buffer for backtracking
- [x] Error collection system

### ✅ State Machine
- [x] Finite state automaton for tokenization
- [x] State transitions for different contexts
- [ ] Backtracking for ambiguous tokens
- [x] Efficient character classification

### ✅ Testing Strategy
- [x] Unit tests for each token type
- [x] Integration tests with sample programs
- [x] Error case testing
- [ ] Performance benchmarks
- [ ] Property-based testing for edge cases

## Dependencies

### ☐ Required Crates
- [ ] miette for error reporting
- [ ] unicode-xid for identifier validation
- [ ] logos or custom implementation for tokenization
- [ ] thiserror for error types

### ☐ Internal Dependencies
- [ ] Source location types
- [ ] Error reporting framework
- [ ] Diagnostic system

## Validation

### ☐ Test Cases
- [ ] All example .op files tokenize correctly
- [ ] Error cases produce helpful messages
- [ ] Performance meets requirements
- [ ] Memory usage is reasonable
- [ ] All edge cases covered

### ☐ Integration
- [ ] Works with parser
- [ ] Integrates with error reporting
- [ ] Supports IDE features (syntax highlighting)
- [ ] Compatible with hot reload system

---

## Implementation Order

1. Basic token structure and source locations
2. Simple literals (numbers, strings, booleans)
3. Identifiers and keywords
4. Operators and punctuation
5. Comments (single and multi-line)
6. Whitespace handling and validation
7. String interpolation
8. Error handling and recovery
9. Advanced features and optimizations
10. Integration testing and validation

## Notes

- Focus on correctness over performance initially
- Use test-driven development throughout
- Ensure all error cases are well-tested
- Maintain compatibility with language specification
- Design for extensibility (new keywords, operators)
