#!/usr/bin/env bash
# Lint script enforcing code quality rules
# - Max 500 lines per file
# - Max 40 lines per function  
# - Max 10 complexity

set -e

MAX_FILE_LINES=500
MAX_FUNC_LINES=40
MAX_COMPLEXITY=10

ERRORS=0

echo "Running linter..."

for crate in crates/*/; do
    echo "Checking $crate..."
    
    for src in "$crate/src"/*.rs; do
        [[ -f "$src" ]] || continue
        
        # Count code lines (exclude blank and comments)
        CODE_LINES=$(grep -cvE '^\s*$|^\s*//' "$src" 2>/dev/null || echo 0)
        
        if (( CODE_LINES > MAX_FILE_LINES )); then
            echo "  ERROR: $src has $CODE_LINES lines (max $MAX_FILE_LINES)"
            ((ERRORS++))
        fi
        
        # Check function complexity using ast
        # This is a simplified check - for full AST parsing, use syn
        if grep -q 'fn ' "$src"; then
            # Count high complexity patterns per function
            # Reset for each function
            awk '
                /^fn |^async fn / { in_func=1; fun_lines=0; complexity=1 }
                in_func {
                    fun_lines++
                    # Count control flow complexity
                    gsub(/[^if while for match]/, "")
                    if (/if |while |for |match /) complexity++
                    if (/else /) complexity++
                }
                /^{/ { brace++ }
                /^}/ { 
                    brace--
                    if (brace == 0 && in_func) {
                        if (fun_lines > '$MAX_FUNC_LINES') {
                            print "  ERROR: Function at line " NR " has " fun_lines " lines (max '$MAX_FUNC_LINES')"
                            exit 1
                        }
                        if (complexity > '$MAX_COMPLEXITY') {
                            print "  ERROR: Function at line " NR " has complexity " complexity " (max '$MAX_COMPLEXITY')"
                            exit 1
                        }
                        in_func = 0
                    }
                }
            ' "$src" || ((ERRORS++))
        fi
    done
done

if (( ERRORS > 0 )); then
    echo ""
    echo "Lint failed with $ERRORS errors"
    exit 1
fi

echo "Lint passed!"
