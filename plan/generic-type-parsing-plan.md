# Generic Type Parsing Implementation Plan

This document outlines the detailed implementation plan for parsing generic types in the Opalescent language.

## Overview

Generic types allow parameterized types like `Array<T>`, `Result<T, E>`, etc. The syntax uses angle brackets `<>` with comma-separated type parameters.

## Syntax Examples

From language spec examples:
- `Array<T>` - single type parameter
- `Result<T, E>` - multiple type parameters  
- `Tree<int32>` - concrete type argument
- `Map<string, Person>` - multiple concrete type arguments
- `Result<Array<T>, Error>` - nested generic types

## Implementation Tasks

### ✅ Parser Function Extension
- [x] Extend `parse_type` function to handle generic syntax
- [x] Add `parse_generic_arguments` helper function
- [x] Handle nested generic types recursively
- [x] Validate generic argument count (defer to semantic analysis)
- [x] Proper error handling for malformed generic syntax

### ✅ Error Handling
- [x] Handle unclosed angle brackets
- [x] Handle empty generic argument lists
- [x] Handle trailing commas in generic arguments
- [x] Handle invalid type arguments
- [x] Provide clear error messages for common mistakes

### ✅ Testing Strategy
- [x] Test simple generic types (`Array<T>`)
- [x] Test multiple type parameters (`Result<T, E>`)
- [x] Test concrete type arguments (`Array<int32>`)
- [x] Test nested generics (`Array<Result<T, E>>`)
- [x] Test complex combinations
- [x] Test error cases (malformed syntax)
- [x] Test integration with existing type parsing

### ✅ Integration
- [x] Ensure compatibility with array type parsing (`T[]`)
- [x] Ensure compatibility with function type parsing
- [x] Update error messages to be consistent
- [x] Test with existing type usage in function signatures

## Implementation Details

### Parsing Algorithm

Current `parse_type` handles:
```rust
// Basic type: int32, string, MyType
// Array type: int32[], MyType[]
```

Need to extend to handle:
```rust
// Generic type: Array<T>, Result<T, E>
// Combined: Array<T>[], Array<Result<T, E>>
```

### Modified Parse Flow

1. **Parse base type name** (existing logic)
2. **Check for generic arguments** `<`
   - If found, parse comma-separated type list
   - Recursively call `parse_type` for each argument
   - Consume closing `>`
3. **Check for array suffix** `[]` (existing logic)

### Error Recovery

- On unclosed `<`: suggest adding `>`
- On empty `<>`: suggest removing or adding type arguments
- On malformed type argument: skip to next comma or closing `>`

## Testing Approach

Following TDD methodology:

1. **Red Phase**: Write failing tests for each generic syntax pattern
2. **Green Phase**: Implement minimal parsing to make tests pass  
3. **Refactor Phase**: Clean up code and handle edge cases

Test categories:
- Simple generic types
- Multiple type parameters
- Nested generic types
- Error cases and recovery
- Integration with existing parsing

## Success Criteria

- [x] All generic type syntax from language spec parses correctly
- [x] Proper AST generation with Generic type variant
- [x] Comprehensive error handling and recovery
- [x] All tests passing with good coverage
- [x] Linter compliance maintained
- [x] Integration with existing parser maintained
