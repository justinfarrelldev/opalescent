# delete-downloads-strict-legal

An end-to-end test fixture for the Opalescent compiler. This project verifies that a strict `guard` clause can log a message before propagating an error, and that an outer `guard` in the entry point correctly handles the propagated error.

The program always produces this output and exits cleanly:

```
STRICT_LEGAL_LIST_ERR=handled-before-propagate
STRICT_LEGAL_PROPAGATED=outer-guard
```

## What it tests

- A function that calls `guard … into … else … propagate` to log a deterministic failure and then forward the error to the caller.
- An entry-point outer `guard` that catches the propagated error and falls back to a default value.
- That the runtime correctly reaches and prints both lines, confirming the error-handling path was exercised.

## Setup

You need the `opal` binary on your PATH. See [CONTRIBUTING.md](../../CONTRIBUTING.md) for full setup instructions.

## How to run

From this project directory:

```bash
opal run
```
