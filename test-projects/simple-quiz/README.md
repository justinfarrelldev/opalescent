# simple-quiz

An end-to-end test project for the Opalescent compiler. This project implements an interactive number-guessing quiz to verify function imports, string interpolation, mutable variables, while loops, and the `is` operator for equality comparisons.

## How to compile and run

Use the Opalescent compiler to compile and execute the program:

```bash
echo "Alice" | opal src/main.op
echo "Alice\n3" | opal src/main.op
```

The program prompts for a name, generates a random number between 1 and 5, prompts the user to guess, and provides feedback based on the guess.
