# Error Handling Standards for Opalescent

This document establishes the consistent error handling patterns that must be used across all modules in the Opalescent compiler to ensure maintainability and excellent developer experience.

## Core Principles

1. **Consistent Error Experience**: All errors must follow the same patterns across lexer, parser, type system, and future modules
2. **Beautiful Error Reporting**: Use miette for all user-facing errors with helpful diagnostics
3. **Source Location Preservation**: All errors must preserve source span information
4. **Multiple Error Collection**: Support collecting and reporting multiple errors simultaneously
5. **Recovery Strategies**: Graceful error recovery to continue processing when possible

## Error Type Standards

### Required Error Traits

All error types MUST derive or implement:
```rust
#[derive(Error, Debug, Diagnostic)]
pub enum ModuleError {
    // variants here
}
```

- `Error` from thiserror for standard error handling
- `Debug` for debugging and logging
- `Diagnostic` from miette for beautiful error formatting

### Error Variant Structure

Each error variant MUST include:
1. **Clear error message** with interpolated values
2. **Diagnostic code** for programmatic error handling
3. **Help text** providing actionable guidance
4. **Source span** for highlighting error location

#### Standard Pattern:
```rust
#[error("Clear description of what went wrong: {context}")]
#[diagnostic(
    code(opalescent::module::error_type),
    help("Actionable advice on how to fix this")
)]
ErrorVariant {
    /// Context information for the error
    context: String,
    /// Position information for debugging
    position: Position, // if applicable
    #[label("descriptive label")]
    /// Source span highlighting the error location
    span: SourceSpan,
},
```

### Source Location Requirements

All errors MUST preserve source location information:

- **Span Information**: Use `SourceSpan` from miette for error highlighting
- **Position Context**: Include `Position` for debugging when applicable  
- **Label Attributes**: Use `#[label("description")]` for span highlighting
- **Multi-span Support**: Support multiple spans when errors span multiple locations

### Error Collection Patterns

Follow these patterns for collecting multiple errors:

```rust
/// Collection of errors for a module
#[derive(Debug)]
pub struct ModuleErrors {
    errors: Vec<ModuleError>,
}

impl ModuleErrors {
    pub fn new() -> Self { /* standard implementation */ }
    pub fn push(&mut self, error: ModuleError) { /* standard implementation */ }
    pub fn is_empty(&self) -> bool { /* standard implementation */ }
    pub fn len(&self) -> usize { /* standard implementation */ }
    pub fn into_vec(self) -> Vec<ModuleError> { /* standard implementation */ }
}
```

## Module-Specific Standards

### Lexer Error Standards

**Location**: `src/error.rs` (already implemented correctly)

**Requirements**:
- Preserve character position and source spans
- Handle whitespace validation (spaces vs tabs)
- Support string literal and number format errors
- Include context for what was expected vs found

**Example Adherence**:
```rust
// ✅ GOOD - Follows standards
#[error("Unexpected character '{character}' at position {position:?}")]
#[diagnostic(
    code(opalescent::lexer::unexpected_character),
    help("Remove or replace this character with a valid token")
)]
UnexpectedCharacter {
    character: char,
    position: Position,
    #[label("unexpected character here")]
    span: SourceSpan,
},
```

### Parser Error Standards

**Location**: `src/parser.rs` (already implemented correctly)

**Requirements**:
- Token-level error recovery and synchronization
- Multiple error collection during parsing
- Context-aware error messages based on parsing state
- Integration with lexer span information

**Example Adherence**:
```rust
// ✅ GOOD - Follows standards
#[error("Expected {expected} but found '{found}'")]
#[diagnostic(
    code(opalescent::parser::unexpected_token),
    help("Check the syntax - {expected} is required here")
)]
UnexpectedToken {
    expected: String,
    found: String,
    #[label("found this instead")]
    span: SourceSpan,
},
```

### Type System Error Standards

**Location**: `src/type_system.rs` (partially implemented, needs enhancement)

**Requirements**:
- Type mismatch visualization with both types shown
- Clear unification failure explanations
- Cast safety violation descriptions
- Integration with type inference error context

**Current Issues to Address**:
```rust
// ❌ NEEDS IMPROVEMENT - Missing source spans
#[error("Cannot unify types '{left}' and '{right}'")]
UnificationFailed {
    left: String,
    right: String,
    // MISSING: source spans for where the types come from
},

// ✅ SHOULD BE - Improved with spans
#[error("Cannot unify types '{left}' and '{right}'")]
#[diagnostic(
    code(opalescent::type_system::unification_failed),
    help("These types are incompatible. Consider using a cast or changing one of the types.")
)]
UnificationFailed {
    left: String,
    right: String,
    #[label("type '{left}' found here")]
    left_span: SourceSpan,
    #[label("type '{right}' expected here")]  
    right_span: SourceSpan,
},
```

## Implementation Requirements

### Span Conversion Utilities

Each module MUST provide utilities for converting internal spans to miette `SourceSpan`:

```rust
impl ModuleError {
    /// Convert internal position to miette SourceSpan
    pub fn span_from_position(pos: Position, len: usize) -> SourceSpan {
        SourceSpan::new(pos.offset.into(), len)
    }
    
    /// Convert internal span to miette SourceSpan  
    pub fn span_from_span(span: InternalSpan) -> SourceSpan {
        // Implementation specific to module's span type
    }
}
```

### Error Recovery Standards

**Parser Recovery**:
- Synchronize at statement/declaration boundaries
- Continue parsing to collect multiple errors
- Use panic mode recovery for syntax errors

**Type System Recovery**:
- Continue type checking after type errors
- Use error types for failed type inference
- Suppress cascading errors from the same root cause

**Integration Points**:
- Pass source file information to miette for file context
- Maintain error context through compilation phases
- Support IDE integration for real-time error reporting

## Testing Standards

### Error Testing Requirements

All error types MUST have tests covering:
1. **Error Creation**: Verify error variants are created correctly
2. **Message Formatting**: Test that error messages are clear and helpful
3. **Span Accuracy**: Verify that source spans highlight correct locations
4. **Recovery Behavior**: Test that error recovery works as expected

### Test Organization

```rust
#[cfg(test)]
mod error_tests {
    use super::*;
    
    #[test]
    fn test_error_variant_creation() { /* test error creation */ }
    
    #[test] 
    fn test_error_message_formatting() { /* test message quality */ }
    
    #[test]
    fn test_span_conversion() { /* test span accuracy */ }
    
    #[test]
    fn test_error_recovery() { /* test recovery behavior */ }
}
```

## Future Phase Considerations

### Hot Reload Error Handling

- Errors during hot reload must not crash the host process
- ABI mismatch errors need special handling and automatic fallback
- Hot reload errors should suggest manual restart when needed

### LLVM Backend Integration

- Compilation errors must map back to source locations
- LLVM errors need to be translated to user-friendly messages  
- Optimization warnings should be preserved and reported

### IDE Integration

- Error reporting must support Language Server Protocol
- Real-time error checking requires incremental error updates
- Error recovery affects IDE responsiveness

## Validation Checklist

Before committing any error handling code, verify:

- [ ] All error variants derive `Error`, `Debug`, and `Diagnostic`
- [ ] Every error includes appropriate diagnostic code and help text
- [ ] Source spans are preserved and converted correctly
- [ ] Error messages are clear and actionable
- [ ] Multiple error collection is supported where applicable
- [ ] Error recovery strategies are implemented
- [ ] Tests cover all error scenarios
- [ ] Integration with existing error handling is seamless

## Migration Strategy

For updating existing error types that don't follow these standards:

1. **Add missing traits**: Ensure Error, Debug, Diagnostic are implemented
2. **Add source spans**: Include SourceSpan fields with appropriate labels
3. **Improve messages**: Make error messages more descriptive and actionable
4. **Add help text**: Include diagnostic help for common fixes
5. **Update tests**: Ensure error tests cover new requirements
6. **Verify integration**: Test with existing error collection systems

---

**Status**: This document defines the standards. Implementation updates needed for type system error enhancement.