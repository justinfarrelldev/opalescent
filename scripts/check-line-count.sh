#!/usr/bin/env bash

set -euo pipefail

# Maximum lines per file
MAX_LINES=1000

# Special cases for specific files that have legitimate reasons to exceed the limit
declare -A FILE_LIMITS
FILE_LIMITS["./src/app.rs"]=1200  # CLI app module needs help text for multiple commands

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