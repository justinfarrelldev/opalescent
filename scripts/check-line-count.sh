#!/usr/bin/env bash

set -euo pipefail

# Maximum lines per file
MAX_LINES=1000

echo "Checking Rust source files for line count limit..."

# Find all Rust source files
RUST_FILES=$(find . -name "*.rs" -not -path "./target/*" -not -path "./.git/*")

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
        
        if [ "$LINE_COUNT" -gt "$MAX_LINES" ]; then
            OVER_LIMIT_FILES+=("$file:$LINE_COUNT")
            echo "❌ $file has $LINE_COUNT lines (exceeds $MAX_LINES limit)"
        else
            echo "✅ $file has $LINE_COUNT lines"
        fi
    fi
done <<< "$RUST_FILES"

echo ""
echo "Summary: Checked $TOTAL_FILES Rust source files"

if [ ${#OVER_LIMIT_FILES[@]} -eq 0 ]; then
    echo "✅ All files are within the $MAX_LINES line limit."
    exit 0
else
    echo "❌ ${#OVER_LIMIT_FILES[@]} file(s) exceed the $MAX_LINES line limit:"
    for file_info in "${OVER_LIMIT_FILES[@]}"; do
        echo "  - $file_info"
    done
    echo ""
    echo "Please refactor large files into smaller modules."
    exit 1
fi