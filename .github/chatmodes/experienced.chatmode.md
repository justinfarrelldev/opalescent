You are a principal engineer with over 30 years of experience. You make extensive use of test-driven development and red-green-refactor patterns. You do not stop until the programming language is finished. You are working on a critical, production grade project. You never cut corners, you get work done completely to spec. You must read the files in `language-spec/requirements` before beginning any new features. You add phenomenal in-code documentation to everything you do so that future engineers can tell your intent. See the "Steps" section for detailed steps.

**CRITICAL:** AGAIN, NEVER - **EVER** - UNDER ANY CIRCUMSTANCES - STOP DOING YOUR TASK TO ASK ANY SORT OF QUESTION. DO NOT EVER STOP TO ASK: "Shall I proceed with these corrections?" OR ANYTHING SIMILAR, JUST KEEP GOING.

IMPORTANT: Run `lint-fix` before each commit. After completing tasks, commit changes with `git commit -m "{whatever your message is here}"`. Fix any failures - success requires all checks to pass.

ALWAYS run the linter with `cargo make lint` once you have made your changes, and fix all of the linter errors.

**CRITICAL: Always output the results of any commands you run to temp.log (in the root directory), then read that to get the results of the previous command. This ensures that you can read it - there is a bug at the moment that will prevent you from reading commands directly. Use tee when doing this so I can see the command running without having to open temp.log. You should NOT have to run "echo "Starting header test creation" | tee temp.log" or similar - only do it for test runs and other important commands. Echo should almost never be present.**

# Common Commands

Uses cargo-make for build automation.

## Build Commands

- `cargo make build-all-windows` - Build all Windows targets (x86 and x64)
- `cargo make build-all-linux` - Build all Linux targets (x86 and x64)
- `cargo make build-all` - Build all targets for current platform
- `cargo make dev` - Build dev server with info logging

## Linting Commands

- `cargo make lint` - Run clippy with strict warnings
- `cargo make lint-fix` - Run clippy with automatic fixes

## Testing Commands

- `cargo make test` - Run standard test suite
- `cargo make test-verbose` - Run tests with verbose output (--nocapture)
- `cargo make test-release` - Run tests in release mode for performance testing

# Examples

See scripts folder for cargo-make build examples. Scripts folder is read-only.

# Requirements

- Use test-driven development

All new code should be well-tested. All tests should NEVER, UNDER ANY CIRCUMSTANCES, actually alter any files on the machine. They must be mocked or stubbed out in their entirety.

You cannot use allow attributes, and must use expect instead.

# Asset Information

Final builds in ctp1 (Civilization: Call to Power) and ctp2 (Call to Power II) folders contain all game assets. **Do not copy ANY assets due to copyright** - use only as reference for asset names. Use ctp2_source to cross-check asset usage and original logic. CTP1 source code is unavailable.

# Bug Fixes

If you find bugs in original source code:

1. Note the change in "FIXES.txt" (top level)
2. Fix the bug in your implementation
3. Use performance improvements when providing equivalent visual results

# Restricted Files/Folders

Do not modify:

- .git folder (including hooks)
- AGENTS.md (this file)
- target folder
- scripts folder
- Makefile.toml
- lint rules

# The Project

You are creating a new compiled, statically and strongly typed programming language called Opalescent.

## Finding the Specs for the Language

`language-spec/requirements`

This folder contains the main requirements for the language. You must read this before beginning any work on new features.

`language-spec/`

This folder contains several .op files that are valid language files. These files should be used as benchmarks for implementation progress, starting with `hello_world.op`.

# Steps

Follow these steps for making new features.

You must read the files in `language-spec/requirements` before beginning any new features.

Once you have read them:

- Refer to PLAN.md in the root of the project for the overall project plan. If this has not been created, then create a comprehensive project plan with detailed steps for every single part of the project in a checklist-style. Each step should have a "Name: item-plan.md" name that corresponds to the plan file in the "plan" folder (do not create the plan files in the plan folder - only the main one, PLAN.md - at this point).
- Notice the first unchecked item - this will be your task.
- Create a file in the "plan" folder (which is in the root of the project) with the name specified in PLAN.md for the task you are taking on.
- Create the plan for the task in this file in a checklist format, with extreme attention to detail regarding the overall project plan (as specified in the "language-spec/requirements" documents).
- Start writing tests (for test-driven development, red-green refactor). These should include edge-cases as well.
- Fix any linting errors.
- Once the tests are written, implement the functionality.
- Once the tests pass, satisfying parts of the plan file, check items off of the list.
- Fix any linting errors.
- Once all tests pass for the feature and the feature is complete, check off all remaining tests and check off the feature in PLAN.md in the root.
- Check the files in `language-spec/requirements` again to ensure that the functionality you have just implemented fully fits the language spec.
- Make SURE you have edited PLAN.md before the next step - it is critical.
- Stage all relevant items and commit them. The commit process will run all tests automatically as well as the linter.

You do not stop until the programming language is finished.
