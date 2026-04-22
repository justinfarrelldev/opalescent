# PowerShell equivalent of check-line-count.sh for Windows parity

# Maximum lines per file
$MAX_LINES = 1000

# Special cases for specific files that have legitimate reasons to exceed the limit
$FILE_LIMITS = @{
    "src/app.rs" = 1200  # CLI app module needs help text for multiple commands
}

Write-Host "Checking Rust source files for line count limit..."
Write-Host "(Excluding test files: tests.rs, test_*.rs, *_test.rs)"

# Find all Rust source files, excluding test files and target/git directories
$RUST_FILES = Get-ChildItem -Recurse -Filter "*.rs" -Exclude "tests.rs", "test_*.rs", "*_test.rs" |
    Where-Object { $_.FullName -notmatch "\\target\\" -and $_.FullName -notmatch "\\.git\\" } |
    Select-Object -ExpandProperty FullName

if ($RUST_FILES.Count -eq 0) {
    Write-Host "✅ No Rust source files found."
    exit 0
}

$OVER_LIMIT_FILES = @()
$TOTAL_FILES = 0

# Check each file
foreach ($file in $RUST_FILES) {
    if (Test-Path $file) {
        $TOTAL_FILES++
        $LINE_COUNT = (Get-Content $file).Count
        
        # Handle case where file has only 1 line (Count returns 0 or 1)
        if ($LINE_COUNT -eq $null) {
            $LINE_COUNT = 1
        } elseif ($LINE_COUNT -eq 0) {
            $LINE_COUNT = 0
        }
        
        # Get the appropriate limit for this file (normalize path with forward slashes)
        $normalizedFile = $file -replace "\\", "/"
        $LIMIT = $MAX_LINES
        if ($FILE_LIMITS.ContainsKey($normalizedFile)) {
            $LIMIT = $FILE_LIMITS[$normalizedFile]
        }
        
        if ($LINE_COUNT -gt $LIMIT) {
            $OVER_LIMIT_FILES += "$file`:$LINE_COUNT"
            Write-Host "❌ $file has $LINE_COUNT lines (exceeds $LIMIT limit)"
        } else {
            Write-Host "✅ $file has $LINE_COUNT lines"
        }
    }
}

Write-Host ""
Write-Host "Summary: Checked $TOTAL_FILES Rust source files"

if ($OVER_LIMIT_FILES.Count -eq 0) {
    Write-Host "✅ All files are within the line limit."
    exit 0
} else {
    Write-Host "❌ $($OVER_LIMIT_FILES.Count) file(s) exceed the line limit:"
    foreach ($file_info in $OVER_LIMIT_FILES) {
        Write-Host "  - $file_info"
    }
    Write-Host ""
    Write-Host "Please refactor large files into smaller modules."
    exit 1
}
