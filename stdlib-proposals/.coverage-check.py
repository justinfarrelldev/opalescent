#!/usr/bin/env python3
"""
Opalescent stdlib-proposals coverage checker.
Checks method coverage and fallible-call handling in .op files.
"""

import sys
import os
import re
import argparse
from pathlib import Path
from typing import Dict, List, Set, Tuple

def find_concern_folders(stdlib_proposals_dir: Path) -> List[Path]:
    """Find all concern folders (direct subdirectories, not hidden)."""
    concerns = []
    for item in stdlib_proposals_dir.iterdir():
        if item.is_dir() and not item.name.startswith('.'):
            concerns.append(item)
    return sorted(concerns)

def find_alternative_folders(concern_dir: Path) -> List[Path]:
    """Find all alternative folders within a concern."""
    alternatives = []
    for item in concern_dir.iterdir():
        if item.is_dir() and not item.name.startswith('.'):
            alternatives.append(item)
    return sorted(alternatives)

def extract_proposed_methods(proposal_file: Path) -> Set[str]:
    """Extract method names from proposal.md fenced code blocks."""
    methods = set()
    
    if not proposal_file.exists():
        return methods
    
    with open(proposal_file, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Find all fenced code blocks with 'op' language tag
    pattern = r'```op\n(.*?)\n```'
    for match in re.finditer(pattern, content, re.DOTALL):
        code_block = match.group(1)
        
        # Extract function names from "let NAME = f(...)" patterns
        func_pattern = r'let\s+([a-z_][a-z0-9_]*)\s*='
        for func_match in re.finditer(func_pattern, code_block):
            methods.add(func_match.group(1))
    
    return methods

def find_method_call_sites(op_files: List[Path], method_name: str) -> List[Tuple[Path, int]]:
    """Find all call sites for a given method name."""
    call_sites = []
    
    for op_file in op_files:
        with open(op_file, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        for line_num, line in enumerate(lines, 1):
            # Look for method calls: method_name(
            if re.search(rf'\b{re.escape(method_name)}\s*\(', line):
                call_sites.append((op_file, line_num))
    
    return call_sites

def extract_fallible_functions(op_files: List[Path]) -> Dict[str, Tuple[Path, int]]:
    """Extract all fallible function definitions from .op files."""
    fallible_funcs = {}
    
    for op_file in op_files:
        with open(op_file, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        for line_num, line in enumerate(lines, 1):
            # Match: let NAME = f(...): return_type errors ...
            match = re.search(r'let\s+([a-z_][a-z0-9_]*)\s*=\s*f\([^)]*\):[^=]*\s+errors\s+', line)
            if match:
                func_name = match.group(1)
                fallible_funcs[func_name] = (op_file, line_num)
    
    return fallible_funcs

def is_fallible_by_heuristic(func_name: str) -> bool:
    """Check if a function name suggests it's fallible."""
    fallible_patterns = [
        '_sync', 'parse', 'read', 'write', 'open', 'connect', 'send', 'recv',
        'hash_stream', 'compress', 'decompress', 'spawn', 'kill', 'flush'
    ]
    
    for pattern in fallible_patterns:
        if pattern in func_name:
            return True
    
    return False

def check_fallible_calls(op_files: List[Path], fallible_funcs: Dict[str, Tuple[Path, int]]) -> List[str]:
    """Check that all fallible calls are wrapped in guard or propagate."""
    violations = []
    
    for op_file in op_files:
        with open(op_file, 'r', encoding='utf-8') as f:
            content = f.read()
            lines = f.readlines()
        
        # Re-read for line-by-line processing
        with open(op_file, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        for line_num, line in enumerate(lines, 1):
            # Find all function calls in this line
            for func_name in fallible_funcs.keys():
                if re.search(rf'\b{re.escape(func_name)}\s*\(', line):
                    # Check if this call is wrapped in guard or propagate
                    # Simple heuristic: check if line contains 'guard' or 'propagate'
                    # or if the previous line contains them
                    
                    is_guarded = 'guard' in line or 'propagate' in line
                    
                    if not is_guarded and line_num > 1:
                        # Check previous line
                        is_guarded = 'guard' in lines[line_num - 2] or 'propagate' in lines[line_num - 2]
                    
                    if not is_guarded:
                        # Check if it's a heuristically fallible function
                        if is_fallible_by_heuristic(func_name):
                            violations.append(
                                f"{op_file}:{line_num}: Unhandled fallible call to '{func_name}' "
                                f"(must be wrapped in 'guard' or 'propagate')"
                            )
    
    return violations

def check_method_coverage(alternative_dir: Path) -> List[str]:
    """Check that every proposed method has at least one call site."""
    violations = []
    
    proposal_file = alternative_dir / 'proposal.md'
    proposed_methods = extract_proposed_methods(proposal_file)
    
    op_files = list(alternative_dir.glob('*.op'))
    
    for method_name in proposed_methods:
        call_sites = find_method_call_sites(op_files, method_name)
        
        if not call_sites:
            violations.append(
                f"{alternative_dir}: Proposed method '{method_name}' has no call site in .op files"
            )
    
    return violations

def check_alternative(alternative_dir: Path) -> List[str]:
    """Run all checks on an alternative folder."""
    violations = []
    
    op_files = list(alternative_dir.glob('*.op'))
    
    # Check method coverage
    violations.extend(check_method_coverage(alternative_dir))
    
    # Check fallible-call handling
    fallible_funcs = extract_fallible_functions(op_files)
    violations.extend(check_fallible_calls(op_files, fallible_funcs))
    
    return violations

def check_concern(concern_dir: Path) -> List[str]:
    """Run all checks on a concern folder."""
    violations = []
    
    alternatives = find_alternative_folders(concern_dir)
    
    for alt_dir in alternatives:
        violations.extend(check_alternative(alt_dir))
    
    return violations

def main():
    parser = argparse.ArgumentParser(
        description='Check method coverage and fallible-call handling in stdlib-proposals'
    )
    parser.add_argument(
        '--concern',
        type=str,
        help='Check only a specific concern folder'
    )
    
    args = parser.parse_args()
    
    stdlib_proposals_dir = Path(__file__).parent
    
    violations = []
    
    if args.concern:
        concern_dir = stdlib_proposals_dir / args.concern
        if not concern_dir.exists():
            print(f"ERROR: Concern folder not found: {concern_dir}", file=sys.stderr)
            return 1
        violations.extend(check_concern(concern_dir))
    else:
        # Check all concerns
        concerns = find_concern_folders(stdlib_proposals_dir)
        for concern_dir in concerns:
            violations.extend(check_concern(concern_dir))
    
    # Report violations
    if violations:
        for violation in violations:
            print(violation, file=sys.stderr)
        return 1
    
    return 0

if __name__ == '__main__':
    sys.exit(main())
