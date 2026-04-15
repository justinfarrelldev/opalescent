# Decisions — cli-commands-wiring

## Design Decisions (from plan / user interviews)
- `opal test`: Stub — empty TestSuite, prints "0 tests found" (no .op discovery)
- `opal bench`: Run hardcoded compiler benchmarks or empty suite
- `opal lsp`: Minimal — instantiate LspServer + print "started" message, no JSON-RPC loop
- `opal doc`: Print generated markdown to stdout
- `opal build`: Read opal.toml → use src/main.op as entry → compile single file
- `opal watch`: PollingFileWatcher + loop; test only error paths (not the loop itself)
- `opal check`: lex→parse→typecheck (no codegen), optionally fmt check
- `opal run`: `opal run <file> [-- args...]` with arg passthrough; PRESERVE `--run` flag backward compat
- `opal fmt --check`: Print "would be reformatted" to stderr + exit 1
- `pkg` command: stays "not yet implemented" — NEVER touch it
