# Type Declaration Parsing Implementation Plan

This document outlines the detailed implementation plan for parsing type declarations in the Opalescent language.

## Overview

Type declarations define custom types (ADTs - Algebraic Data Types) with variants and fields. They support:
- Documentation comments
- Generic parameters
- Sum types (variants with different structures)
- Product types (fields within variants)
- Recursive types

## Syntax Examples

From language-spec/types_example.types.op:

```opalescent
##
    Description: A simple message type
##
type Message:
    Text:
        sender: string
        body: string
    Join:
        user: string
    Leave:
        user: string
    Error:
        code: int32
        description: string

##
    Description: returns T when Ok and E when Error. A generic ADT
##
type Result<T, E>:
    Ok:
        value: T
    Error:
        error: E

##
    Description: An enum implementation
##
type Direction:
    North
    East
    South
    West
```

## Implementation Tasks

### ☐ AST Node Extensions
- [ ] Add `TypeDeclaration` to the `Decl` enum
- [ ] Add `TypeVariant` struct for variant definitions
- [ ] Add `TypeField` struct for field definitions within variants
- [ ] Add `GenericParameter` struct for type parameters
- [ ] Update AST display implementations for new nodes

### ☐ Parser Extensions
- [ ] Implement `parse_type_declaration` function
- [ ] Implement `parse_type_variants` function  
- [ ] Implement `parse_type_variant` function
- [ ] Implement `parse_type_fields` function
- [ ] Implement `parse_generic_parameters` function
- [ ] Handle indentation-based variant/field parsing
- [ ] Integrate with existing documentation comment parsing

### ☐ Error Handling
- [ ] Add specific error types for type declaration parsing
- [ ] Handle malformed variant definitions
- [ ] Handle malformed field definitions
- [ ] Handle invalid generic parameter syntax
- [ ] Handle indentation errors
- [ ] Provide helpful error messages for common mistakes

### ☐ Testing Strategy
- [ ] Test simple type declarations without generics
- [ ] Test type declarations with generic parameters
- [ ] Test enum-style types (variants without fields)
- [ ] Test struct-style types (single variant with fields)
- [ ] Test complex ADTs with multiple variants and fields
- [ ] Test recursive type definitions
- [ ] Test error cases (malformed syntax, invalid indentation)
- [ ] Test integration with documentation comments

### ☐ Integration
- [ ] Update declaration parsing dispatcher to handle `type` keyword
- [ ] Ensure proper integration with existing AST infrastructure
- [ ] Update visitor pattern to handle new AST nodes
- [ ] Verify compatibility with existing error reporting

## Implementation Details

### AST Node Structure

```rust
// In Decl enum
TypeDeclaration {
    name: String,
    generic_parameters: Vec<GenericParameter>,
    variants: Vec<TypeVariant>,
    doc_comment: Option<String>,
    visibility: Visibility,
    span: Span,
    id: NodeId,
}

// New structs
struct TypeVariant {
    name: String,
    fields: Vec<TypeField>,
    span: Span,
}

struct TypeField {
    name: String,
    field_type: Type,
    span: Span,
}

struct GenericParameter {
    name: String,
    span: Span,
}
```

### Parsing Algorithm

1. **Parse Declaration Header**
   - Consume `type` keyword
   - Parse type name (must be PascalCase)
   - Parse optional generic parameters `<T, E>`
   - Consume colon `:`

2. **Parse Variants**
   - For each indented line, parse as variant
   - Variant can have fields (indented further) or be empty
   - Handle proper indentation validation

3. **Parse Fields**
   - For each field, parse `name: type` syntax
   - Support all existing type parsing (basic, array, generic)

### Error Recovery

- On malformed variant: skip to next variant or end of type
- On malformed field: skip to next field or next variant
- On indentation errors: provide clear diagnostic about expected structure

## Testing Approach

Following TDD methodology:

1. **Red Phase**: Write failing tests for each syntax pattern
2. **Green Phase**: Implement minimal parsing to make tests pass
3. **Refactor Phase**: Clean up code and handle edge cases

Test categories:
- Basic type declarations
- Generic type declarations  
- Enum-style declarations
- Complex ADT declarations
- Error cases and recovery
- Integration with existing parser

## Success Criteria

- [ ] All type declaration examples from language-spec parse correctly
- [ ] Proper AST generation with all required fields
- [ ] Comprehensive error handling and recovery
- [ ] All tests passing with good coverage
- [ ] Linter compliance maintained
- [ ] Integration with existing parser infrastructure complete
