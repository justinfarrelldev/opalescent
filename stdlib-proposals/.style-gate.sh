#!/bin/bash
set -euo pipefail

# Opalescent stdlib-proposals style-gate script
# Checks all style rules across the stdlib-proposals folder

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STDLIB_PROPOSALS_DIR="$SCRIPT_DIR"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track violations
VIOLATIONS=0

# Helper function to report violations
report_violation() {
    local file="$1"
    local line="$2"
    local message="$3"
    echo -e "${RED}VIOLATION${NC}: $file:$line - $message" >&2
    ((VIOLATIONS++))
}

# Helper function to report info
report_info() {
    local message="$1"
    echo -e "${YELLOW}INFO${NC}: $message" >&2
}

# Check 1: Forbidden patterns in .op files
check_forbidden_patterns() {
    report_info "Checking for forbidden patterns in .op files..."
    
    while IFS= read -r file; do
        # Check for Result<, Option<, Either<
        if grep -nE 'Result<|Option<|Either<' "$file" >/dev/null 2>&1; then
            while IFS=: read -r line_num content; do
                if [[ "$content" =~ Result\<|Option\<|Either\< ]]; then
                    report_violation "$file" "$line_num" "Forbidden type wrapper (Result<, Option<, Either<)"
                fi
            done < <(grep -nE 'Result<|Option<|Either<' "$file")
        fi
        
        # Check for [T] array syntax (but not inside comments)
        if grep -nE ':\s*\[[A-Za-z_]' "$file" >/dev/null 2>&1; then
            while IFS=: read -r line_num content; do
                if [[ "$content" =~ :[[:space:]]*\[[A-Za-z_] ]]; then
                    report_violation "$file" "$line_num" "Forbidden array syntax [T] - use T[] instead"
                fi
            done < <(grep -nE ':\s*\[[A-Za-z_]' "$file")
        fi
        
        # Check for semicolons at line end (but not in comments)
        if grep -nE ';[[:space:]]*$' "$file" >/dev/null 2>&1; then
            while IFS=: read -r line_num content; do
                # Skip if it's in a comment
                if [[ ! "$content" =~ ^[[:space:]]*# ]]; then
                    report_violation "$file" "$line_num" "Forbidden semicolon at line end"
                fi
            done < <(grep -nE ';[[:space:]]*$' "$file")
        fi
        
        # Check for non_blocking/wait_for_completion/DeferredResult/LaterResult
        if grep -nEw 'non_blocking|wait_for_completion|DeferredResult|LaterResult' "$file" >/dev/null 2>&1; then
            while IFS=: read -r line_num content; do
                if [[ "$content" =~ (^|[^a-zA-Z_])(non_blocking|wait_for_completion|DeferredResult|LaterResult)([^a-zA-Z_]|$) ]]; then
                    report_violation "$file" "$line_num" "Forbidden non_blocking/wait_for_completion/DeferredResult/LaterResult keyword"
                fi
            done < <(grep -nEw 'non_blocking|wait_for_completion|DeferredResult|LaterResult' "$file")
        fi
    done < <(find "$STDLIB_PROPOSALS_DIR" -name "*.op" -type f)
}

# Check 2: _sync suffix enforcement in I/O-bearing concerns
check_sync_suffix() {
    report_info "Checking _sync suffix enforcement in I/O-bearing concerns..."
    
    local io_concerns=("file-io-surface" "network-http-layer" "subprocess-exec" "logging" "time-date-api")
    local fallible_patterns=("read" "write" "open" "connect" "send" "recv" "flush" "sleep" "spawn" "kill" "compress" "decompress" "hash_stream" "serialize" "parse")
    
    for concern in "${io_concerns[@]}"; do
        local concern_dir="$STDLIB_PROPOSALS_DIR/$concern"
        if [[ ! -d "$concern_dir" ]]; then
            continue
        fi
        
        while IFS= read -r file; do
            # Extract function names from "let NAME = f(...)" patterns
            while IFS= read -r func_line; do
                # Extract function name
                if [[ $func_line =~ let[[:space:]]+([a-z_][a-z0-9_]*)[[:space:]]*= ]]; then
                    local func_name="${BASH_REMATCH[1]}"
                    
                    # Check if function name contains any fallible pattern
                    local has_fallible=0
                    for pattern in "${fallible_patterns[@]}"; do
                        if [[ "$func_name" =~ $pattern ]]; then
                            has_fallible=1
                            break
                        fi
                    done
                    
                    # If it has a fallible pattern, it must end with _sync
                    if [[ $has_fallible -eq 1 ]] && [[ ! "$func_name" =~ _sync$ ]]; then
                        local line_num=$(grep -n "let $func_name" "$file" | head -1 | cut -d: -f1)
                        report_violation "$file" "$line_num" "Function '$func_name' in I/O concern must end with _sync suffix"
                    fi
                fi
            done < <(grep -n "^let " "$file")
        done < <(find "$concern_dir" -name "*.op" -type f)
    done
}

# Check 3: 10-section template conformance in proposal.md files
check_template_conformance() {
    report_info "Checking 10-section template conformance in proposal.md files..."
    
    local required_sections=("## Overview" "## Assumes" "## Syntax Design" "## Example Applications" "## Strengths" "## Weaknesses" "## Impact on Existing Syntax" "## Interactions with Other Concerns" "## Implementation Difficulty" "## Must NOT Have")
    
    while IFS= read -r file; do
        for section in "${required_sections[@]}"; do
            if ! grep -q "^$section\$" "$file"; then
                report_violation "$file" "1" "Missing required section: $section"
            fi
        done
    done < <(find "$STDLIB_PROPOSALS_DIR" -name "proposal.md" -type f)
}

# Check 4: Line budget (≤250 lines per proposal.md)
check_line_budget() {
    report_info "Checking line budget for proposal.md files..."
    
    while IFS= read -r file; do
        local line_count=$(wc -l < "$file")
        if [[ $line_count -gt 250 ]]; then
            report_violation "$file" "1" "Exceeds 250-line budget ($line_count lines)"
        fi
    done < <(find "$STDLIB_PROPOSALS_DIR" -name "proposal.md" -type f)
}

# Check 5: COMPARISON.md presence in every concern folder
check_comparison_presence() {
    report_info "Checking COMPARISON.md presence in concern folders..."
    
    # Find all direct subdirectories of stdlib-proposals that are not hidden and not README.md
    while IFS= read -r concern_dir; do
        # Only check if the concern folder has alternative subfolders (is non-empty)
        local alt_count=$(find "$concern_dir" -maxdepth 1 -mindepth 1 -type d ! -name ".*" | wc -l)
        if [[ $alt_count -gt 0 ]]; then
            if [[ ! -f "$concern_dir/COMPARISON.md" ]]; then
                report_violation "$concern_dir" "1" "Missing COMPARISON.md"
            fi
        fi
    done < <(find "$STDLIB_PROPOSALS_DIR" -maxdepth 1 -mindepth 1 -type d ! -name ".*" | sort)
}

# Check 6: Minimum .op files per alternative folder
check_minimum_op_files() {
    report_info "Checking minimum .op files per alternative folder..."
    
    # Find all alternative folders (subdirectories of concern folders)
    while IFS= read -r alt_dir; do
        local op_count=$(find "$alt_dir" -maxdepth 1 -name "*.op" -type f | wc -l)
        if [[ $op_count -lt 2 ]]; then
            report_violation "$alt_dir" "1" "Alternative folder must have at least 2 .op files (found $op_count)"
        fi
    done < <(find "$STDLIB_PROPOSALS_DIR" -maxdepth 2 -mindepth 2 -type d ! -name ".*" | sort)
}

# Check 7: Doc-block presence on public functions in .op files
check_doc_blocks() {
    report_info "Checking doc-block presence on public functions..."
    
    while IFS= read -r file; do
        # Find all "let" or "public let" declarations
        local line_num=0
        local prev_line=""
        while IFS= read -r line; do
            ((line_num++))
            
            # Check if this line is a function declaration
            if [[ $line =~ ^(public[[:space:]]+)?let[[:space:]]+[a-z_] ]]; then
                # Check if previous line is a doc block start
                if [[ ! "$prev_line" =~ ^##$ ]]; then
                    report_violation "$file" "$line_num" "Public function missing doc block (##...##)"
                fi
            fi
            
            prev_line="$line"
        done < "$file"
    done < <(find "$STDLIB_PROPOSALS_DIR" -name "*.op" -type f)
}

# Check 8: Placeholder errors clause rejection
check_placeholder_errors() {
    report_info "Checking for placeholder errors clauses..."
    
    while IFS= read -r file; do
        if grep -nE 'errors\s+[^,]+,\s*\.\.\.|errors\s+.*etc' "$file" >/dev/null 2>&1; then
            while IFS=: read -r line_num content; do
                report_violation "$file" "$line_num" "Placeholder in errors clause (..., etc)"
            done < <(grep -nE 'errors\s+[^,]+,\s*\.\.\.|errors\s+.*etc' "$file")
        fi
    done < <(find "$STDLIB_PROPOSALS_DIR" -name "*.op" -type f)
}

# Check 9: Scenario comment presence
check_scenario_comments() {
    report_info "Checking for scenario comments in .op files..."
    
    while IFS= read -r file; do
        if ! grep -q "^# " "$file"; then
            report_violation "$file" "1" "Missing scenario comment (# ...)"
        fi
    done < <(find "$STDLIB_PROPOSALS_DIR" -name "*.op" -type f)
}

# Check 10: Delegate to coverage-check.py
check_coverage() {
    report_info "Running coverage checks via .coverage-check.py..."
    
    if [[ -f "$STDLIB_PROPOSALS_DIR/.coverage-check.py" ]]; then
        if ! python3 "$STDLIB_PROPOSALS_DIR/.coverage-check.py" 2>&1; then
            ((VIOLATIONS++))
        fi
    else
        report_info "Skipping coverage checks (.coverage-check.py not found)"
    fi
}

# Main execution
main() {
    echo "Running Opalescent stdlib-proposals style-gate..."
    echo ""
    
    # Run all checks
    check_forbidden_patterns || true
    check_sync_suffix || true
    check_template_conformance || true
    check_line_budget || true
    check_comparison_presence || true
    check_minimum_op_files || true
    check_doc_blocks || true
    check_placeholder_errors || true
    check_scenario_comments || true
    check_coverage || true
    
    echo ""
    if [[ $VIOLATIONS -eq 0 ]]; then
        echo -e "${GREEN}All style checks passed.${NC}"
        return 0
    else
        echo -e "${RED}Found $VIOLATIONS style violations.${NC}" >&2
        return 1
    fi
}

main "$@"
