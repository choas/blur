#!/bin/bash
# Verify that ESOLANGS.wiki examples match actual code output

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BLUR="$PROJECT_DIR/target/release/blur"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Build if needed
if [ ! -f "$BLUR" ]; then
    echo -e "${YELLOW}Building blur...${NC}"
    cd "$PROJECT_DIR" && cargo build --release
fi

echo "=== Verifying ESOLANGS.wiki claims ==="
echo ""

passed=0
failed=0

check() {
    local name="$1"
    local expected="$2"
    local actual="$3"

    if [ "$expected" = "$actual" ]; then
        echo -e "${GREEN}[PASS]${NC} $name"
        ((passed++))
    else
        echo -e "${RED}[FAIL]${NC} $name"
        echo "       Expected: $expected"
        echo "       Actual:   $actual"
        ((failed++))
    fi
}

# Test 1: Variable Averaging
result=$($BLUR -e "int x = 10; x = 20; x = 30; print(x);")
check "Variable averaging (10,20,30) → 21" "21" "$result"

# Test 2: Int ceiling
result=$($BLUR -e "int x = 5; x = 6; print(x);")
check "Int ceiling avg(5,6) → 6" "6" "$result"

# Test 3: Bool with blur 0.5
result=$(echo '#blur 0.5
int blur() { bool b = true; b = false; print(b); return 0; }' | $BLUR -)
check "Bool with #blur 0.5 (true,false) → false" "false" "$result"

# Test 4: Increment operators
result=$($BLUR -e "int x = 5; x++; x++; print(x);")
check "Increment (5,++,++) → 6" "6" "$result"

# Test 5: Compound assignment with blur=1.0
result=$($BLUR --blur 1.0 -e "int x = 10; x += 5; print(x);")
check "Compound x=10, x+=5 (blur=1.0) → 13" "13" "$result"

# Test 6: Boolean averaging
result=$($BLUR -e "bool flag = true; flag = false; print(flag);")
check "Bool (true,false) with blur=0.9 → false" "false" "$result"

# Test 7: String hello + " a   "
result=$($BLUR -e 'string s = "hello"; s = " a   "; print(s);')
check "String 'hello' + ' a   ' → hcllo" "hcllo" "$result"

# Test 8: String abc + "  x"
result=$($BLUR -e 'string s = "abc"; s = "  x"; print(s);')
check "String 'abc' + '  x' → abo" "abo" "$result"

# Test 9: String case averaging
result=$($BLUR -e 'string s = "abc"; s = "ABC"; s = "ABC"; print(s);')
check "String 'abc'+'ABC'+'ABC' → KLM" "KLM" "$result"

# Test 10: String repetition
result=$($BLUR -e 'string s = "aaa" * 3; s = "zzz"; print(s);')
check "String 'aaa'*3 + 'zzz' → iii" "iii" "$result"

# Test 11: blurstr
result=$($BLUR -e 'print(blurstr("hello", " a   "));')
check "blurstr('hello',' a   ') → hcllo" "hcllo" "$result"

# Test 12: blurstr with repetition
result=$($BLUR -e 'print(blurstr("aaa" * 3, "zzz"));')
check "blurstr('aaa'*3,'zzz') → iii" "iii" "$result"

# Test 13: Array operations
result=$($BLUR -e 'int arr[3] = {1, 2, 3}; arr[0] = 10; print(arr[0]);')
check "Array arr[0]={1,10} → 6" "6" "$result"

result=$($BLUR -e 'int arr[3] = {1, 2, 3}; arr[1]++; print(arr[1]);')
check "Array arr[1]++ → 3" "3" "$result"

# Test 14: Money example
result=$(echo 'int blur() {
    int money = 5;
    money++;
    money = 100;
    print(money);
    return 0;
}' | $BLUR -)
check "Money (5,++,=100) → 41" "41" "$result"

# Test 15: sharp for (loop counter doesn't blur)
result=$($BLUR -e 'sharp for (int i = 0; i < 5; i++) { print(i); }' | wc -l | tr -d ' ')
check "sharp for prints 5 lines (0-4)" "5" "$result"

echo ""
echo "========================================"
echo "Results: $passed passed, $failed failed"
if [ $failed -gt 0 ]; then
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
fi
