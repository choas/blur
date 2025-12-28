#!/bin/bash
# Run all Blur examples and report results

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BLUR="$PROJECT_DIR/target/release/blur"
EXAMPLES_DIR="$PROJECT_DIR/examples"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Build if needed
if [ ! -f "$BLUR" ]; then
    echo -e "${YELLOW}Building blur...${NC}"
    cd "$PROJECT_DIR" && cargo build --release
fi

echo "========================================"
echo "Running all Blur examples"
echo "========================================"
echo ""

passed=0
failed=0
total=0

for example in "$EXAMPLES_DIR"/*.blur; do
    if [ -f "$example" ]; then
        filename=$(basename "$example")
        total=$((total + 1))

        echo -e "${YELLOW}>>> $filename${NC}"
        echo "----------------------------------------"

        if timeout 10s "$BLUR" "$example" 2>&1; then
            echo ""
            echo -e "${GREEN}[PASS]${NC} $filename"
            passed=$((passed + 1))
        else
            exit_code=$?
            echo ""
            if [ $exit_code -eq 124 ]; then
                echo -e "${RED}[TIMEOUT]${NC} $filename (exceeded 10s)"
            else
                echo -e "${RED}[FAIL]${NC} $filename (exit code: $exit_code)"
            fi
            failed=$((failed + 1))
        fi
        echo ""
    fi
done

echo "========================================"
echo "Results: $passed/$total passed"
if [ $failed -gt 0 ]; then
    echo -e "${RED}$failed example(s) failed${NC}"
    exit 1
else
    echo -e "${GREEN}All examples passed!${NC}"
fi
