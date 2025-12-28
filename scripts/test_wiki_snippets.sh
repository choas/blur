#!/bin/bash
# Extract and test all code snippets from ESOLANGS.wiki

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BLUR="$PROJECT_DIR/target/release/blur"
WIKI_FILE="$PROJECT_DIR/ESOLANGS.wiki"
TEMP_DIR=$(mktemp -d)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Cleanup on exit
trap "rm -rf $TEMP_DIR" EXIT

# Build if needed
if [ ! -f "$BLUR" ]; then
    echo -e "${YELLOW}Building blur...${NC}"
    cd "$PROJECT_DIR" && cargo build --release
fi

echo "========================================"
echo "Testing ESOLANGS.wiki code snippets"
echo "========================================"
echo ""

# Extract code snippets between <pre> and </pre> tags
snippets=()
in_pre=false
current_snippet=""

while IFS= read -r line; do
    if [[ "$line" == "<pre>" ]]; then
        in_pre=true
        current_snippet=""
        continue
    fi

    if [[ "$line" == "</pre>" ]]; then
        in_pre=false
        if [ -n "$current_snippet" ]; then
            snippets+=("$current_snippet")
        fi
        continue
    fi

    if $in_pre; then
        current_snippet+="$line"$'\n'
    fi
done < "$WIKI_FILE"

echo "Found ${#snippets[@]} code snippets"
echo ""

passed=0
failed=0
skipped=0

is_shell_command() {
    local snippet="$1"
    local first_line=$(echo "$snippet" | head -1 | xargs)

    # Shell commands
    [[ "$first_line" == blur\ * ]] && return 0
    [[ "$first_line" == echo\ * ]] && return 0
    [[ "$snippet" == *"| blur"* ]] && return 0

    return 1
}

is_output_example() {
    local snippet="$1"
    local first_line=$(echo "$snippet" | head -1)

    # Output examples like "i = 0    (1 iteration)"
    [[ "$first_line" =~ ^[a-z]\ =\ [0-9]+\ +\( ]] && return 0

    # Contains ... (continuation)
    [[ "$snippet" == *"..."* ]] && return 0

    return 1
}

is_valid_blur_code() {
    local snippet="$1"

    # Must contain at least one semicolon or closing brace (actual code)
    [[ "$snippet" == *";"* ]] && return 0
    [[ "$snippet" == *"}"* ]] && return 0

    return 1
}

# Strip single-line comments from code (but not from inside strings)
# This is a simplified version that just removes // to end of line
strip_comments() {
    sed 's|//.*||g'
}

for i in "${!snippets[@]}"; do
    snippet="${snippets[$i]}"
    num=$((i + 1))

    # Skip shell commands
    if is_shell_command "$snippet"; then
        echo -e "${CYAN}[SKIP]${NC} Snippet $num (shell command)"
        skipped=$((skipped + 1))
        continue
    fi

    # Skip output examples
    if is_output_example "$snippet"; then
        echo -e "${CYAN}[SKIP]${NC} Snippet $num (output example)"
        skipped=$((skipped + 1))
        continue
    fi

    # Skip if not valid blur code
    if ! is_valid_blur_code "$snippet"; then
        echo -e "${CYAN}[SKIP]${NC} Snippet $num (not valid code)"
        skipped=$((skipped + 1))
        continue
    fi

    # Create temp file
    temp_file="$TEMP_DIR/snippet_$num.blur"
    echo "$snippet" > "$temp_file"

    echo -e "${YELLOW}>>> Snippet $num${NC}"
    echo "----------------------------------------"
    # Show first few lines
    echo "$snippet" | head -5
    lines=$(echo "$snippet" | wc -l)
    if [ "$lines" -gt 5 ]; then
        echo "... ($lines lines total)"
    fi
    echo "----------------------------------------"

    # Check if it's a complete program (has blur() function) or just statements
    if echo "$snippet" | grep -q "int blur()\|void blur()"; then
        # Complete program - run directly (keep comments, they work in files)
        if timeout 10s "$BLUR" "$temp_file" 2>&1; then
            echo -e "${GREEN}[PASS]${NC} Snippet $num (program)"
            passed=$((passed + 1))
        else
            exit_code=$?
            if [ $exit_code -eq 124 ]; then
                echo -e "${RED}[TIMEOUT]${NC} Snippet $num"
            else
                echo -e "${RED}[FAIL]${NC} Snippet $num (exit code: $exit_code)"
            fi
            failed=$((failed + 1))
        fi
    else
        # Just statements - strip comments, write to temp file, and run as file
        # This avoids shell quoting issues with -e
        stripped_file="$TEMP_DIR/snippet_${num}_stripped.blur"
        cat "$temp_file" | strip_comments > "$stripped_file"

        # Wrap in a blur() function to make it runnable
        wrapped_file="$TEMP_DIR/snippet_${num}_wrapped.blur"
        echo "void blur() {" > "$wrapped_file"
        cat "$stripped_file" >> "$wrapped_file"
        echo "}" >> "$wrapped_file"

        if timeout 10s "$BLUR" "$wrapped_file" 2>&1; then
            echo -e "${GREEN}[PASS]${NC} Snippet $num (statements)"
            passed=$((passed + 1))
        else
            exit_code=$?
            if [ $exit_code -eq 124 ]; then
                echo -e "${RED}[TIMEOUT]${NC} Snippet $num"
            else
                echo -e "${RED}[FAIL]${NC} Snippet $num (exit code: $exit_code)"
            fi
            failed=$((failed + 1))
        fi
    fi
    echo ""
done

total=$((passed + failed))
echo "========================================"
echo "Results: $passed/$total tested, $skipped skipped"
if [ $failed -gt 0 ]; then
    echo -e "${RED}$failed snippet(s) failed${NC}"
    exit 1
else
    echo -e "${GREEN}All tested snippets passed!${NC}"
fi
