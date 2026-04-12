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
- T5 verification note: `cargo test` fails in this repository due to many pre-existing failures outside lexer scope (parser/type_system/doc_gen/hot_reload), while lexer-focused suites pass including new Indent/Dedent tests.

- T3 implementation: embedded runtime with include_str! in src/compiler.rs; link_object_file now materializes runtime source to unique temp .c file and removes it via Drop guard after linking.
- Verification: compiler succeeds when runtime/ is temporarily renamed, confirming no runtime path dependency at execution time.
- Verification caveat: repository has pre-existing unrelated test failures on current branch state; task evidence captures a prior passing cargo test run and focused runtime-embedding QA outputs.
