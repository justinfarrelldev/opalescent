51:./target/release/opalescent --help
102:| `opal <file.op> --run` | Compile and execute an Opalescent source file |
104:| `opal --help` | Alias for `opal help` |
108:| `opal lsp [options]` | Start the language server |
109:| `opal test [options]` | Run project tests |
110:| `opal doc [options]` | Generate documentation |
111:| `opal bench` | Run benchmarks |
126:Pass `--run` to execute the compiled binary immediately after compilation:
129:opal src/main.op --run
180:### `opal lsp` — Language Server
183:opal lsp [options]
190:| `--stdio` | Communicate over stdin/stdout (required for editor integration) |
195:opal lsp --stdio
198:### `opal test` — Test Runner
201:opal test [options]
208:| `--target <triple>` | Run tests for a specific build target |
209:| `--filter <pattern>` | Only run tests whose names contain `<pattern>` |
214:opal test
215:opal test --filter my_test
216:opal test --target x86_64-linux
219:### `opal doc` — Documentation Generator
222:opal doc [options]
229:| `--format <md\|html>` | Output format (default: `md`) |
234:opal doc
235:opal doc --format html
238:### `opal bench` — Benchmarks
241:opal bench
249:opal bench
256:opal --help            # Alias for opal help
879:opal lsp --stdio
