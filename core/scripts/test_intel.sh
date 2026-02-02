#!/bin/bash
# Intel Service E2E Test Suite
# Run from apps/tastematter/core directory

TASTEMATTER="./target/release/tastematter"
PASSED=0
FAILED=0

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

pass() {
    echo -e "${GREEN}✓ $1${NC}"
    PASSED=$((PASSED + 1))
}

fail() {
    echo -e "${RED}✗ $1${NC}"
    FAILED=$((FAILED + 1))
}

echo "=== Intel Service E2E Tests ==="
echo ""

# Test 1: Health check
echo "Test 1: Intel health check..."
if $TASTEMATTER intel health 2>&1 | grep -q "Intel service: OK"; then
    pass "Health check returns OK"
else
    fail "Health check failed (is Intel service running on :3002?)"
    echo "  Start with: cd ../intel && bun run dev"
    echo ""
    echo "=== Results ==="
    echo -e "${RED}Failed: 1${NC}"
    exit 1
fi

# Test 2: Name a chain
echo "Test 2: Name chain via CLI..."
RESULT=$($TASTEMATTER intel name-chain test123 --files "src/main.rs,src/lib.rs" --session-count 5 2>&1)
if echo "$RESULT" | grep -q "generated_name"; then
    pass "Name chain returns generated_name"
else
    fail "Name chain missing generated_name"
    echo "  Output: $RESULT"
fi

# Test 3: Query chains shows names
echo "Test 3: Query chains includes generated_name..."
CHAINS=$($TASTEMATTER query chains --limit 10 2>&1)
if echo "$CHAINS" | grep -q "generated_name"; then
    pass "Query chains includes generated_name field"
else
    fail "Query chains missing generated_name (run daemon once first)"
fi

# Test 4: Daemon completes successfully
echo "Test 4: Daemon sync completes..."
DAEMON=$($TASTEMATTER daemon once 2>&1)
if echo "$DAEMON" | grep -q "Sync complete"; then
    pass "Daemon sync completes successfully"
    # Check for Intel activity (may be empty if all cached)
    if echo "$DAEMON" | grep -q "Intel:"; then
        echo "  (Intel named some chains)"
    else
        echo "  (All chains already cached)"
    fi
else
    fail "Daemon sync failed"
    echo "  Output: $DAEMON"
fi

# Test 5: Cache prevents duplicates
echo "Test 5: Cache prevents duplicate API calls..."
FIRST=$($TASTEMATTER daemon once 2>&1 | grep -o 'Named [0-9]* chains' || echo "Named 0 chains")
SECOND=$($TASTEMATTER daemon once 2>&1 | grep -o 'Named [0-9]* chains' || echo "Named 0 chains")
FIRST_COUNT=$(echo "$FIRST" | grep -oE '[0-9]+' | head -1)
SECOND_COUNT=$(echo "$SECOND" | grep -oE '[0-9]+' | head -1)
# Default to 0 if empty
FIRST_COUNT=${FIRST_COUNT:-0}
SECOND_COUNT=${SECOND_COUNT:-0}
if [ "$SECOND_COUNT" -le "$FIRST_COUNT" ]; then
    pass "Second run names fewer/equal chains (cache working)"
else
    fail "Second run named MORE chains than first (cache broken?)"
fi

# Summary
echo ""
echo "=== Results ==="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi

echo ""
echo "All tests passed!"
