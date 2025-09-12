# Hot Reload Architecture & Metadata Preservation

This document outlines the architectural requirements and metadata preservation strategies needed to support Opalescent's hot reloading system throughout all development phases.

## Hot Reload System Overview

**Goal**: Versioned dynamic-library hot-swap with ABI guard and automatic fallback restart.

**Components**:
- **Host Process**: Owns all long-lived state and threads
- **Hot Modules**: Compiled to `.so/.dylib/.dll` with narrow C ABI
- **ABI Guard**: Machine-checkable interface signature hashing
- **Change Classifier**: Determines hot-swap vs restart eligibility
- **Version Management**: Prevents file locks with versioned filenames

## Metadata Requirements by Phase

### Phase 1: Foundation Infrastructure

#### AST Metadata Preservation

**Current Status**: ✅ Good foundation
- Source spans preserved in all AST nodes
- Node IDs for unique identification
- Visitor pattern support

**Additional Requirements**:
```rust
// AST nodes must preserve symbols for ABI generation
pub trait ASTNode {
    fn span(&self) -> Span;
    fn node_id(&self) -> NodeId;
    
    // NEW: Hot reload metadata
    fn abi_symbols(&self) -> Vec<SymbolInfo>;
    fn dependencies(&self) -> Vec<ModulePath>;
    fn is_hot_reloadable(&self) -> bool;
}

// Symbol information for ABI signature generation
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub symbol_type: SymbolType,
    pub signature: TypeSignature,
    pub visibility: Visibility,
    pub source_location: Span,
}

#[derive(Debug, Clone)]
pub enum SymbolType {
    Function,
    Type,
    Variable,
    Constant,
}
```

#### Type System Metadata

**Current Status**: ⚠️ Needs Enhancement
- Basic type representation complete
- Type environment infrastructure ready

**Required Enhancements**:
```rust
// Type system must support ABI signature generation
impl TypeEnvironment {
    // NEW: ABI signature methods
    pub fn generate_abi_signature(&self) -> ABISignature;
    pub fn compare_abi_compatibility(&self, other: &ABISignature) -> CompatibilityResult;
    pub fn get_exported_types(&self) -> Vec<(String, CoreType)>;
}

// ABI signature for compatibility checking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ABISignature {
    pub version: u64,
    pub exported_functions: BTreeMap<String, FunctionSignature>,
    pub exported_types: BTreeMap<String, TypeSignature>, 
    pub memory_layout: MemoryLayout,
    pub abi_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSignature {
    pub parameters: Vec<TypeSignature>,
    pub return_type: TypeSignature,
    pub calling_convention: CallingConvention,
}
```

### Phase 2: Language Features

#### Function System Hot Reload Support

**Requirements**:
- Function signature stability tracking
- Parameter type ABI compatibility
- Return type compatibility validation
- Entry point function special handling

```rust
// Function declarations must track ABI stability
impl Decl {
    fn abi_stability(&self) -> ABIStability;
    fn hot_reload_category(&self) -> HotReloadCategory;
}

#[derive(Debug, Clone)]
pub enum ABIStability {
    Stable,      // Safe for hot reload
    Breaking,    // Requires restart
    Compatible,  // Backward compatible change
}

#[derive(Debug, Clone)]
pub enum HotReloadCategory {
    HotSwappable,           // Function body changes
    RequiresRestart,        // Signature changes
    StateDependent,         // Depends on runtime state
}
```

#### Variable System Considerations

**Requirements**:
- Track which variables can be preserved across reloads
- Identify state that must be migrated
- Handle static/global variable compatibility

### Phase 3: Advanced Type Features

#### ADT Hot Reload Implications

**Critical Requirements**:
- Struct field addition/removal compatibility
- Enum variant changes
- Memory layout stability
- Pattern matching compatibility

```rust
// ADT changes must be classified for hot reload safety
#[derive(Debug, Clone)]
pub struct ADTChangeAnalysis {
    pub memory_layout_compatible: bool,
    pub field_additions: Vec<String>,
    pub field_removals: Vec<String>,
    pub field_type_changes: Vec<(String, CoreType, CoreType)>,
    pub reload_strategy: ReloadStrategy,
}

#[derive(Debug, Clone)]
pub enum ReloadStrategy {
    HotSwap,                    // Memory layout compatible
    StatePreservingRestart,     // Save state, restart, restore
    FullRestart,               // Complete restart required
}
```

### Phase 4: Module System

#### Import/Export Hot Reload Tracking

**Requirements**:
- Track module dependencies for change propagation
- Handle import graph updates
- Manage circular dependency hot reload
- Version management for modules

```rust
// Module system must support dependency analysis
#[derive(Debug, Clone)]
pub struct ModuleDependencyGraph {
    pub modules: BTreeMap<ModulePath, ModuleMetadata>,
    pub dependencies: BTreeMap<ModulePath, Vec<ModulePath>>,
    pub abi_hashes: BTreeMap<ModulePath, u64>,
}

#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub path: ModulePath,
    pub exports: Vec<SymbolInfo>,
    pub imports: Vec<ImportInfo>,
    pub abi_signature: ABISignature,
    pub hot_reload_eligible: bool,
}
```

### Phase 5: Code Generation

#### LLVM Backend Hot Reload Support

**Requirements**:
- Generate position-independent code
- Preserve debug information for state inspection
- Support dynamic linking and symbol resolution
- Handle LLVM IR hot-swapping

```rust
// Code generation must support hot reload requirements
pub struct CodeGenContext {
    pub target_info: TargetInfo,
    pub hot_reload_config: HotReloadConfig,
    pub symbol_table: SymbolTable,
    pub debug_info: DebugInfo,
}

#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    pub enable_hot_reload: bool,
    pub position_independent: bool,
    pub preserve_debug_info: bool,
    pub symbol_export_strategy: SymbolExportStrategy,
}
```

### Phase 6: Hot Reloading Implementation

#### ABI Signature Generation

**Implementation Strategy**:
```rust
// ABI signature generation from type system
impl TypeChecker {
    pub fn generate_module_abi(&self, module: &Module) -> Result<ABISignature, ABIError> {
        let mut signature = ABISignature::new();
        
        // Collect exported functions
        for decl in module.declarations() {
            if let Decl::Function { visibility: Visibility::Public, .. } = decl {
                signature.add_function(self.function_signature(decl)?);
            }
        }
        
        // Collect exported types
        for type_decl in module.type_declarations() {
            if type_decl.is_public() {
                signature.add_type(self.type_signature(type_decl)?);
            }
        }
        
        // Generate hash
        signature.abi_hash = self.compute_abi_hash(&signature);
        Ok(signature)
    }
}
```

#### Change Classification

**Change Analysis Algorithm**:
```rust
pub struct ChangeClassifier {
    pub old_abi: ABISignature,
    pub new_abi: ABISignature,
    pub dependency_graph: ModuleDependencyGraph,
}

impl ChangeClassifier {
    pub fn classify_changes(&self) -> ChangeClassification {
        let mut classification = ChangeClassification::new();
        
        // Analyze function changes
        for (name, old_func) in &self.old_abi.exported_functions {
            if let Some(new_func) = self.new_abi.exported_functions.get(name) {
                classification.function_changes.insert(
                    name.clone(),
                    self.classify_function_change(old_func, new_func)
                );
            } else {
                classification.removed_functions.push(name.clone());
            }
        }
        
        // Analyze type changes
        // ... similar analysis for types
        
        // Determine overall reload strategy
        classification.overall_strategy = self.determine_reload_strategy(&classification);
        classification
    }
}
```

## Implementation Guidelines

### AST Node Requirements

All AST nodes MUST implement:
1. **Symbol Collection**: Extract all symbols that affect ABI
2. **Dependency Tracking**: Identify module dependencies
3. **Hot Reload Classification**: Determine reload eligibility
4. **Metadata Preservation**: Maintain all information needed for analysis

### Type System Requirements

The type system MUST:
1. **Generate ABI Signatures**: Create machine-readable interface descriptions
2. **Compare Compatibility**: Determine if changes are compatible
3. **Track Memory Layout**: Monitor struct/enum layout changes
4. **Preserve Type Information**: Maintain types through compilation pipeline

### Parser Requirements

The parser MUST:
1. **Preserve Source Context**: Maintain spans and location information
2. **Track Declarations**: Identify public vs private declarations
3. **Handle Incremental Parsing**: Support partial re-parsing for hot reload
4. **Maintain Symbol Tables**: Build symbol information during parsing

## Testing Strategy

### Hot Reload Test Categories

1. **ABI Compatibility Tests**:
   - Compatible function signature changes
   - Incompatible type changes
   - Memory layout modifications

2. **Change Classification Tests**:
   - Function body modifications (hot-swappable)
   - Function signature changes (restart required)
   - Type definition changes (various categories)

3. **Integration Tests**:
   - End-to-end hot reload scenarios
   - State preservation across reloads
   - Error recovery and fallback

4. **Performance Tests**:
   - ABI signature generation speed
   - Change analysis performance
   - Hot reload latency measurement

### Test Implementation

```rust
#[cfg(test)]
mod hot_reload_tests {
    use super::*;
    
    #[test]
    fn test_compatible_function_change() {
        // Test that function body changes are classified as hot-swappable
    }
    
    #[test]
    fn test_incompatible_type_change() {
        // Test that struct field removal requires restart
    }
    
    #[test]
    fn test_abi_signature_generation() {
        // Test that ABI signatures are generated correctly
    }
    
    #[test]
    fn test_change_classification() {
        // Test change classification algorithm
    }
}
```

## Performance Considerations

### ABI Signature Caching

```rust
// Cache ABI signatures to avoid recomputation
pub struct ABICache {
    signatures: BTreeMap<ModulePath, (u64, ABISignature)>, // (file_hash, signature)
    change_cache: BTreeMap<(u64, u64), ChangeClassification>, // (old_hash, new_hash)
}
```

### Incremental Analysis

- Only analyze changed modules and their dependents
- Cache change classifications for common scenarios
- Use file watching to trigger minimal rebuilds

## Future Considerations

### IDE Integration

Hot reload metadata will support:
- Real-time change preview in IDEs
- Hot reload eligibility indicators
- Performance impact warnings

### Debugging Support

- State inspection across hot reloads
- Breakpoint preservation
- Variable value migration

### Production Deployment

- Hot reload system can be disabled for production builds
- ABI signatures useful for deployment compatibility checking
- Metadata overhead minimal when hot reload disabled

---

**Status**: Architecture planned. Implementation needed across all phases.
**Priority**: High - affects fundamental design decisions in all modules.