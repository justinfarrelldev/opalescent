# Native test artifacts

`opal test` checks the original project sources before selecting or running cases.
Listing does not build artifacts or execute application/setup code.

Executable workers are cached only under `target/native-testing/cache`, separately
from production builds. Each SHA-256 content key includes:

- Original source bytes, canonical source paths and independent case identities.
- Compiler release and running executable identity (including embedded test/runtime
  sources and compiled feature configuration).
- Actual host target, native backend profile and test runtime ABI version.
- Test instrumentation configuration and interception declarations in source.
- Linker invocation/version and relevant toolchain/header/library-search environment.

The current executable worker backend supports the host target and its fixed
`native-default` profile. Other requested targets fail explicitly; they are not
silently treated as host builds. Cache identity describes the actual profile,
not an unimplemented user-selectable optimization profile.

Each build uses a private staging directory. Only a successfully linked executable
and complete manifest are atomically published. Concurrent builders may duplicate
compilation but cannot overwrite a published worker used by another attempt.
Failed builds do not publish entries. A hit verifies the namespace/schema, exact
case identities and executable digest. Corrupt/incompatible entries produce an
infrastructure error; remove the affected entry before rebuilding. Cache files
are trusted local build products, not a security boundary against their owner.

Attempts still run in fresh processes, with separate result/stdout/stderr files;
cache reuse never reuses World state, mutable globals, double configurations or
histories. Normal application binaries and instrumented workers are distinct
artifacts and cannot satisfy one another's cache manifests.
