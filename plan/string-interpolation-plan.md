# String Interpolation Implementation Plan

This document outlines the detailed implementation plan for string interpolation in the Opalescent language parser.

## Overview

String interpolation allows expressions to be embedded within string literals using the syntax `'text {expression} more text'`. The expressions inside `{}` are evaluated and their string representation is inserted into the final string.

## Requirements Analysis

From the language specification examples:
- `'Hello {world}'` - simple variable interpolation
- `'fib({n}) = {result}'` - multiple interpolations with complex expressions
- `'Error: {e}'` - interpolation with error values

## Token Support Required

### ✅ Existing Token Support
- [x] String literal tokens (already exist)
- [x] Left/Right brace tokens (already exist for blocks)

### ☐ New Token Support Needed
- [ ] Interpolated string start token
- [ ] Interpolated string middle token  
- [ ] Interpolated string end token
- [ ] String interpolation expression tokens

## AST Node Implementation

### ☐ String Interpolation Expression
- [ ] `StringInterpolation` variant in `Expr` enum
- [ ] `InterpolationPart` enum for string/expression parts
- [ ] Source location tracking for each part
- [ ] Display implementation for debugging

### ☐ AST Structure
```rust
pub enum InterpolationPart {
    String(String),           // Static string content
    Expression(Box<Expr>),    // Interpolated expression
}

// In Expr enum:
StringInterpolation {
    parts: Vec<InterpolationPart>,
    span: Span,
}
```

## Lexer Updates Required

### ☐ String Literal Lexing Enhancement
- [ ] Detect interpolated strings (start with ' and contain {})
- [ ] State machine for parsing interpolated content
- [ ] Proper handling of nested braces
- [ ] Escape sequence support within interpolated strings

### ☐ Token Stream Generation
- [ ] Generate sequence of string/expression tokens
- [ ] Maintain position tracking through interpolation
- [ ] Handle edge cases (empty expressions, nested strings)

## Parser Implementation

### ☐ String Interpolation Parsing
- [ ] `parse_string_interpolation` method
- [ ] State tracking for interpolation parsing
- [ ] Expression parsing within braces
- [ ] Error recovery for malformed interpolations

### ☐ Integration with Expression Parsing
- [ ] Add string interpolation to `parse_primary`
- [ ] Proper precedence handling
- [ ] Error reporting for interpolation syntax errors

## Error Handling

### ☐ Interpolation-Specific Errors
- [ ] Unclosed interpolation braces
- [ ] Empty interpolation expressions
- [ ] Nested interpolation strings (if not supported)
- [ ] Invalid expressions within interpolation

### ☐ Error Recovery
- [ ] Recovery from malformed interpolations
- [ ] Context-aware error messages
- [ ] Suggestions for common mistakes

## Testing Strategy

### ☐ Unit Tests (TDD Implementation)
- [ ] Simple variable interpolation: `'Hello {world}'`
- [ ] Multiple interpolations: `'fib({n}) = {result}'`
- [ ] Complex expressions: `'Result: {a + b * c}'`
- [ ] Function call interpolation: `'Value: {get_value()}'`
- [ ] Type expressions: `'Type: {type_of(x)}'`
- [ ] Edge cases: empty string parts, only interpolation
- [ ] Error cases: unclosed braces, empty expressions

### ☐ Integration Tests
- [ ] Parse full example files with interpolation
- [ ] Interaction with other expression types
- [ ] AST roundtrip testing
- [ ] Error message quality testing

### ☐ Property-Based Tests
- [ ] Random interpolation generation
- [ ] Nested expression complexity
- [ ] String content variations

## Implementation Strategy

### ☐ Phase 1: AST Foundation
- [ ] Define InterpolationPart enum
- [ ] Add StringInterpolation to Expr enum
- [ ] Implement Display and Debug traits
- [ ] Write initial failing tests

### ☐ Phase 2: Lexer Support
- [ ] Identify when string contains interpolation
- [ ] Implement state machine for parsing
- [ ] Generate appropriate token sequence
- [ ] Test lexer output for interpolated strings

### ☐ Phase 3: Parser Integration
- [ ] Implement parse_string_interpolation
- [ ] Integrate with parse_primary
- [ ] Add comprehensive error handling
- [ ] Ensure all tests pass

### ☐ Phase 4: Advanced Features
- [ ] Nested brace handling
- [ ] Complex expression support
- [ ] Performance optimization
- [ ] Enhanced error messages

## Edge Cases to Handle

### ☐ Brace Handling
- [ ] Literal braces in string content (escaped?)
- [ ] Nested braces in expressions
- [ ] Unmatched braces
- [ ] Empty brace pairs `{}`

### ☐ Expression Complexity
- [ ] Function calls within interpolation
- [ ] Arithmetic expressions
- [ ] Member access expressions
- [ ] Nested interpolated strings (if supported)

### ☐ String Content
- [ ] Empty string parts between interpolations
- [ ] Only interpolation (no static string content)
- [ ] Special characters and escape sequences
- [ ] Unicode content

## Validation Criteria

### ☐ Correctness
- [ ] All example .op files parse correctly
- [ ] AST structure accurately represents interpolation
- [ ] Error cases produce helpful messages
- [ ] Integration with existing parser features

### ☐ Performance
- [ ] Parsing performance acceptable for complex interpolations
- [ ] Memory usage reasonable for large interpolated strings
- [ ] No unnecessary allocations during parsing

### ☐ Code Quality
- [ ] All linting rules satisfied
- [ ] Comprehensive test coverage
- [ ] Clear error messages
- [ ] Maintainable code structure

---

## Implementation Checklist

1. [ ] Write comprehensive TDD tests for string interpolation
2. [ ] Implement AST nodes for string interpolation
3. [ ] Update lexer to handle interpolated strings (if needed)
4. [ ] Implement parser support for string interpolation
5. [ ] Ensure all tests pass and linter is clean
6. [ ] Test against example .op files
7. [ ] Update parser foundation plan
8. [ ] Commit changes with appropriate message

## Notes

- Start with simple cases and build complexity
- Focus on error handling and recovery
- Ensure compatibility with existing string literal parsing
- Consider future extensions (format specifiers, etc.)
- Maintain test-driven development approach
