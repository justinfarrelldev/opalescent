# Issues — spec-alignment-runtime-embedding

## Known Issues (Pre-implementation)

### Parser
- parse_if_statement() expects LeftBrace/RightBrace — colon-block not supported
- parse_while_statement() same issue
- parse_for_statement() same issue
- No loop/break/continue/guard/import keywords parsed

### Lexer
- Indent/Dedent tokens defined in token.rs but NEVER emitted
- Position tracking exists but no indent stack

### Compiler
- src/compiler.rs line 194: hardcoded Path::new("runtime/opal_runtime.c")
- link_object_file() reads runtime from disk — fails if runtime/ folder missing

### Type System
- No int32 type — only int64

### Test Files
- All 4 test-project .op files diverge from language-spec (wrong syntax, types, signatures)

## Issues Found During Implementation
(append here as work progresses)
