#!/usr/bin/env bash

set -euo pipefail

# Maximum lines per file
MAX_LINES=1000

# Special cases for specific files that have legitimate reasons to exceed the limit
declare -A FILE_LIMITS
FILE_LIMITS["./src/app.rs"]=1200  # CLI app module needs help text for multiple commands
FILE_LIMITS["./tests/array_integration.rs"]=1400  # Integration fixture matrix intentionally keeps array ownership regressions together
FILE_LIMITS["./tests/integration_e2e/project_execution.rs"]=1050  # Project execution harness is a legacy umbrella test pending later split
FILE_LIMITS["./src/ast.rs"]=1050  # AST facade remains a transitional re-export layer while split submodules stabilize
FILE_LIMITS["./src/formatter/printer.rs"]=1050  # Formatter printer keeps paired syntax emitters together for round-trip stability
FILE_LIMITS["./src/type_system/errors.rs"]=1050  # Type-error surface remains centralized for diagnostics and report rendering
FILE_LIMITS["./src/type_system/checker.rs"]=1050  # Type-checker root coordinates extracted helpers and shared context state
FILE_LIMITS["./src/codegen/expressions_array.rs"]=2000  # Array lowering still centralizes RC/COW ownership paths and sanitizer regressions
FILE_LIMITS["./src/codegen/functions_stdlib.rs"]=1050  # Stdlib declaration bridge is still organized as one internal registry module
FILE_LIMITS["./src/codegen/control_flow.rs"]=1200  # Control-flow lowering keeps error-ABI return and loop wiring together
FILE_LIMITS["./src/codegen/adts.rs"]=1050  # ADT codegen remains a single layout/lowering module pending later refactor
FILE_LIMITS["./src/codegen/statements.rs"]=1250  # Statement lowering coordinates multi-return destructuring with RC-safe stores
FILE_LIMITS["./src/codegen/functions_call.rs"]=1180  # Call lowering keeps ordinary and fallible expression lowering in one module
FILE_LIMITS["./src/codegen/functions_call/array/intrinsics.rs"]=2000  # Array intrinsic lowering remains intentionally consolidated
FILE_LIMITS["./src/compiler.rs"]=1050  # Compiler pipeline root keeps verification and object emission together

echo "Checking Rust source files for line count limit..."
echo "(Excluding test files: tests.rs, test_*.rs, *_test.rs)"

# Find all Rust source files, excluding test files
RUST_FILES=$(find . -name "*.rs" \
    -not -path "./target/*" \
    -not -path "./.git/*" \
    -not -name "tests.rs" \
    -not -name "test_*.rs" \
    -not -name "*_test.rs")

if [ -z "$RUST_FILES" ]; then
    echo "✅ No Rust source files found."
    exit 0
fi

OVER_LIMIT_FILES=()
TOTAL_FILES=0

# Check each file
while IFS= read -r file; do
    if [ -f "$file" ]; then
        TOTAL_FILES=$((TOTAL_FILES + 1))
        LINE_COUNT=$(wc -l < "$file")
        
        # Get the appropriate limit for this file
        LIMIT=${FILE_LIMITS["$file"]:-$MAX_LINES}
        
        if [ "$LINE_COUNT" -gt "$LIMIT" ]; then
            OVER_LIMIT_FILES+=("$file:$LINE_COUNT")
            echo "❌ $file has $LINE_COUNT lines (exceeds $LIMIT limit)"
        else
            echo "✅ $file has $LINE_COUNT lines"
        fi
    fi
done <<< "$RUST_FILES"

echo ""
echo "Summary: Checked $TOTAL_FILES Rust source files"

if [ ${#OVER_LIMIT_FILES[@]} -eq 0 ]; then
    echo "✅ All files are within the line limit."
    exit 0
else
    echo "❌ ${#OVER_LIMIT_FILES[@]} file(s) exceed the line limit:"
    for file_info in "${OVER_LIMIT_FILES[@]}"; do
        echo "  - $file_info"
    done
    echo ""
    echo "Please refactor large files into smaller modules."
    exit 1
fi
