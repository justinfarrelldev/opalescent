# simple-quiz

An end-to-end test project for the Opalescent compiler. This project implements an interactive number-guessing quiz to verify function imports, string interpolation, mutable variables, while loops, and the `is` operator for equality comparisons.

## Setup

You need the `opal` binary on your PATH. See [CONTRIBUTING.md](../../CONTRIBUTING.md) for full setup instructions.

## How to run

From this project directory:

```bash
opal run
```

The program prompts for your name, generates a random number between 1 and 5, then prompts you to guess. It reports whether your guess was correct, too low, or too high.
