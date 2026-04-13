186:let add = f(a: int32, b: int32): int32 =>
196:entry main = f(args: string[]): void =>
217:  Returns: int32
219:let fib = f(n: int32): int32 =>
234:let count: int32 = 42
252:    age: int32
270:        code: int32
308:        value: int32
345:let parse_number = f(text: string): int32 errors ParseError =>
352:let load_number = f(path: string): int32 errors IoError, ParseError =>
360:let parse_number = f(text: string): int32 errors ParseError =>
371:let load_number = f(path: string): int32 errors IoError, ParseError =>
542:5. **Linking** — The object file is linked with the C runtime (embedded in the compiler binary) to produce the final binary. No `runtime/` folder is needed at runtime.
616:- Use **`int32` for all numeric types**
620:- Entry function must be named `f(args: string[]): void` (legacy signatures without parameters are also supported):
623:entry main = f(args: string[]): void =>
847:- Built-in types (`int32`, `int64`, `float64`, `string`, `boolean`, `void`)
