# Integration Dependencies & Phase Coordination

This document outlines the critical dependencies between phases and modules to prevent blocking issues and ensure smooth development progression.

## Phase Dependency Matrix

### Phase 1: Foundation & Core Infrastructure

**Completed**:
- ✅ Project Setup
- ✅ Lexical Analysis  
- ✅ Parser Foundation

**In Progress**:
- ⏳ Type System Core (CRITICAL BLOCKER for Phase 2)

**Dependencies**: None (foundational phase)

### Phase 2: Language Features (BLOCKED)

**Blockers**:
- 🚫 **Type System Core incomplete**: Missing type inference, checking framework, cast validation
- 🚫 **No type checking integration**: Parser AST not connected to type system
- 🚫 **No cast safety**: Required for arithmetic operations
- 🚫 **No scope management**: Required for variable system

**Dependencies from Phase 1**:
- Type inference engine
- Expression type checking
- Statement type checking  
- Declaration type checking
- Cast validation framework

**Provides to Phase 3**:
- Function type checking foundation
- Variable resolution system
- Basic arithmetic type safety
- Control flow type validation

### Phase 3: Advanced Type Features (DEPENDS ON PHASE 2)

**Dependencies from Phase 2**:
- Complete type checking framework
- Function type validation
- Variable scoping system
- Arithmetic type safety

**Provides to Phase 4**:
- Generic type system
- ADT type checking
- Pattern matching infrastructure
- Collection type safety

### Phase 4: Module System (DEPENDS ON PHASE 3)

**Dependencies from Phase 3**:
- Generic type instantiation
- ADT type checking
- Complete type validation

**Provides to Phase 5**:
- Symbol resolution
- Module dependency graph
- Cross-module type checking
- Import/export validation

### Phase 5: Code Generation (DEPENDS ON PHASE 4)

**Dependencies from Phase 4**:
- Complete symbol resolution
- Module dependency information
- Cross-module type validation
- Memory layout information

**Provides to Phase 6**:
- Compilation pipeline
- Symbol table generation
- Debug information
- Binary artifact creation

### Phase 6: Hot Reloading (DEPENDS ON PHASE 5)

**Dependencies from Phase 5**:
- Dynamic library compilation
- Symbol table access
- ABI information
- Binary compatibility data

## Critical Blocking Dependencies

### 1. Type System Core → All Language Features

**What's Missing**:
```rust
// Type inference engine
impl TypeChecker {
    pub fn infer_expression_type(&mut self, expr: &Expr) -> Result<CoreType, TypeError>;
    pub fn check_statement(&mut self, stmt: &Stmt) -> Result<(), TypeError>;
    pub fn check_declaration(&mut self, decl: &Decl) -> Result<(), TypeError>;
}

// Cast validation
impl TypeChecker {
    pub fn validate_cast(&self, from: &CoreType, to: &CoreType) -> Result<CastSafety, TypeError>;
    pub fn check_arithmetic_safety(&self, op: BinaryOp, left: &CoreType, right: &CoreType) -> Result<CoreType, TypeError>;
}

// Scope management
impl TypeChecker {
    pub fn enter_scope(&mut self);
    pub fn exit_scope(&mut self);
    pub fn declare_variable(&mut self, name: String, type_: CoreType) -> Result<(), TypeError>;
    pub fn lookup_variable(&self, name: &str) -> Option<&CoreType>;
}
```

**Impact**: Phase 2 cannot begin without these foundations.

### 2. Parser Integration → Type Checking

**What's Missing**:
```rust
// Integration between parser and type checker
impl Program {
    pub fn type_check(&self, checker: &mut TypeChecker) -> Result<TypedProgram, Vec<TypeError>>;
}

// AST nodes need type information
pub struct TypedExpr {
    pub expr: Expr,
    pub type_info: CoreType,
    pub span: Span,
}

// Type checking integration
impl Parser {
    pub fn parse_and_type_check(&mut self, checker: &mut TypeChecker) -> Result<TypedProgram, CompilerErrors>;
}
```

**Impact**: Cannot validate program correctness without type checking integration.

### 3. Memory Layout → Hot Reload + Code Generation

**What's Missing**:
```rust
// Memory layout for hot reload compatibility
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryLayout {
    pub size: usize,
    pub alignment: usize,
    pub field_offsets: BTreeMap<String, usize>,
    pub layout_hash: u64,
}

// Type system must provide layout information
impl TypeChecker {
    pub fn compute_memory_layout(&self, type_: &CoreType) -> Result<MemoryLayout, TypeError>;
    pub fn compare_layout_compatibility(&self, old: &MemoryLayout, new: &MemoryLayout) -> bool;
}
```

**Impact**: Hot reload system depends on memory layout stability detection.

## Module Integration Requirements

### Lexer ↔ Parser Integration

**Status**: ✅ Complete
- Token stream properly passed
- Error integration working
- Span information preserved

### Parser ↔ Type System Integration

**Status**: ❌ Missing
- AST nodes need type annotation support
- Type checking not integrated with parsing
- Error reporting not unified

**Required Integration**:
```rust
// Parser must support type checking integration
impl Parser {
    pub fn set_type_checker(&mut self, checker: TypeChecker);
    pub fn parse_with_type_checking(&mut self) -> Result<TypedProgram, CompilerErrors>;
}

// Type checker must work with parser AST
impl TypeChecker {
    pub fn check_program(&mut self, program: &Program) -> Result<TypeAnnotations, Vec<TypeError>>;
    pub fn annotate_ast(&self, program: Program, annotations: TypeAnnotations) -> TypedProgram;
}
```

### Type System ↔ Code Generation Integration

**Status**: ⏳ Planned (Phase 5)
- Type information must survive compilation pipeline
- Memory layout information required
- Symbol table generation needed

**Required Planning**:
```rust
// Type information for code generation
pub struct CodeGenTypeInfo {
    pub type_layout: MemoryLayout,
    pub calling_convention: CallingConvention,
    pub symbol_name: String,
    pub linkage: Linkage,
}

// Type system must provide code generation data
impl TypeChecker {
    pub fn generate_codegen_info(&self) -> CodeGenContext;
    pub fn get_symbol_table(&self) -> SymbolTable;
}
```

## Error Handling Integration

### Cross-Module Error Propagation

**Current Status**:
- ✅ Lexer: miette integration complete
- ✅ Parser: miette integration complete  
- ⚠️ Type System: basic miette, needs enhancement

**Required Integration**:
```rust
// Unified error collection across modules
#[derive(Debug)]
pub struct CompilerErrors {
    pub lex_errors: Vec<LexError>,
    pub parse_errors: Vec<ParseError>,
    pub type_errors: Vec<TypeError>,
}

impl CompilerErrors {
    pub fn report_all(&self, source: &str) -> miette::Result<()>;
    pub fn has_fatal_errors(&self) -> bool;
    pub fn can_continue_compilation(&self) -> bool;
}
```

### Source Location Consistency

**Requirements**:
- All errors must trace back to original source
- Span information preserved through all phases
- Multi-span errors supported for related issues

## Testing Integration Requirements

### Cross-Module Testing

**Current Gaps**:
- No integration tests between parser and type system
- No end-to-end compilation tests
- No hot reload integration tests

**Required Test Infrastructure**:
```rust
#[cfg(test)]
mod integration_tests {
    // Test full compilation pipeline
    #[test]
    fn test_parse_and_type_check() {
        let source = "entry main(): int32 => 42";
        let result = compile_program(source);
        assert!(result.is_ok());
    }
    
    // Test error propagation
    #[test]
    fn test_error_integration() {
        let source = "let x: int32 = \"hello\""; // type mismatch
        let result = compile_program(source);
        assert!(matches!(result, Err(CompilerErrors { type_errors, .. }) if !type_errors.is_empty()));
    }
    
    // Test hot reload metadata
    #[test]
    fn test_hot_reload_metadata() {
        let source = "public let x = 42";
        let metadata = extract_hot_reload_metadata(source);
        assert!(metadata.exports.contains(&"x"));
    }
}
```

## Performance Integration Requirements

### Compilation Pipeline Performance

**Requirements**:
- Incremental compilation support
- Parallel processing where possible  
- Memory usage optimization
- Hot reload performance targets

**Architectural Constraints**:
```rust
// Incremental compilation support
pub trait IncrementalCompilation {
    fn parse_incremental(&mut self, changes: &[FileChange]) -> Result<PartialAST, ParseErrors>;
    fn type_check_incremental(&mut self, changes: &PartialAST) -> Result<TypeChanges, Vec<TypeError>>;
    fn invalidate_dependencies(&mut self, changes: &TypeChanges);
}
```

## Development Workflow Integration

### Phase Transition Checklist

**Before Starting Phase 2**:
- [ ] Type inference engine implemented
- [ ] Expression type checking complete
- [ ] Statement type checking complete
- [ ] Declaration type checking complete
- [ ] Cast validation framework ready
- [ ] Scope management implemented
- [ ] Parser-type system integration complete
- [ ] All Phase 1 tests passing
- [ ] Error handling unified
- [ ] Hot reload metadata planning complete

**Before Starting Phase 3**:
- [ ] Function system complete
- [ ] Variable system complete
- [ ] Control flow complete
- [ ] Arithmetic operations complete
- [ ] All Phase 2 integration tests passing
- [ ] Performance benchmarks established

### Integration Validation

**Continuous Integration Requirements**:
```bash
# Full pipeline validation
cargo make test          # Unit tests
cargo make test-integration  # Cross-module tests
cargo make test-performance  # Performance regression tests
cargo make lint         # Code quality
cargo make docs         # Documentation validation
```

## Future Phase Considerations

### IDE Integration Dependencies

**Requirements**:
- Language Server Protocol support
- Real-time error reporting
- Incremental parsing and type checking
- Hot reload status reporting

### Production Deployment Dependencies

**Requirements**:
- Optimized compilation pipeline
- Error handling for production use
- Performance monitoring
- Compatibility verification

## Risk Mitigation Strategies

### Blocking Dependency Risks

**Type System Incompleteness**:
- **Risk**: Phase 2 indefinitely blocked
- **Mitigation**: Complete type system before any Phase 2 work
- **Validation**: Comprehensive type checking tests

**Integration Complexity**:
- **Risk**: Module integration becomes unwieldy
- **Mitigation**: Define clear interface contracts
- **Validation**: Integration test coverage

**Performance Regression**:
- **Risk**: Hot reload performance unacceptable
- **Mitigation**: Performance benchmarks from Phase 1
- **Validation**: Continuous performance monitoring

### Cross-Module Communication

**Interface Stability**:
- Define stable interfaces between modules
- Use semantic versioning for internal APIs
- Document breaking changes explicitly

**Error Propagation**:
- Consistent error handling patterns
- Unified error reporting
- Clear error recovery strategies

---

**Status**: Dependencies documented. Type System Core completion is immediate priority.
**Action Required**: Focus all effort on completing type inference and checking framework.