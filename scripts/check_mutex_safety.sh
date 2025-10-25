#!/bin/bash
# CI Guardrail: Fail if any src/api/*.rs contains .lock().unwrap()
# Only allow lock usage inside with_storage/with_lock helpers

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔒 Checking Mutex Safety in API Handlers"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Exclude backup files
FILES=$(find src/api -name "*.rs" -type f | grep -v backup | grep -v "\.bak")

VIOLATIONS=0

for file in $FILES; do
    # Check for .lock().unwrap()
    if grep -n "\.lock()\.unwrap()" "$file" > /dev/null 2>&1; then
        echo "❌ FAIL: $file contains .lock().unwrap()"
        grep -n "\.lock()\.unwrap()" "$file"
        VIOLATIONS=$((VIOLATIONS + 1))
    fi
done

echo ""
if [ $VIOLATIONS -eq 0 ]; then
    echo "✅ PASS: No unsafe .lock().unwrap() found in API handlers"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 0
else
    echo "❌ FAIL: Found $VIOLATIONS file(s) with unsafe .lock().unwrap()"
    echo ""
    echo "Use with_storage() or with_lock() helpers instead:"
    echo "  with_storage(&state.shared_storage, \"label\", |storage| {...})"
    echo "  with_lock(&state.mutex, \"label\", |data| {...})"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 1
fi
