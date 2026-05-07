# windows-file-ops

A Wine-oriented Opalescent fixture that exercises Windows/MSVC filesystem operations with deterministic marker output. The fixture uses real stdlib filesystem APIs against paths containing spaces and non-ASCII characters so the Rust harness can validate both stdout markers and host-visible workspace artifacts.
