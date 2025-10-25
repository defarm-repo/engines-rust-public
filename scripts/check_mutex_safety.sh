#!/bin/bash
# Detects dangerous patterns: MutexGuard held during CPU/IO intensive operations

set -e

echo "🔍 Checking for unsafe mutex usage patterns..."
echo ""

VIOLATIONS=0

# Pattern 1: .lock() followed by bcrypt operations
echo "Checking for: .lock() → bcrypt operations..."
BCRYPT_VIOLATIONS=$(grep -rn "\.lock()" src/ --include="*.rs" -A 20 | \
  grep -B 5 "bcrypt::\|verify(\|hash(" | \
  grep "\.lock()" | \
  grep -v "backup\|\.bak" || true)

if [ -n "$BCRYPT_VIOLATIONS" ]; then
  echo "❌ FAIL: Found .lock() before bcrypt operations:"
  echo "$BCRYPT_VIOLATIONS"
  echo ""
  VIOLATIONS=$((VIOLATIONS + 1))
fi

# Pattern 2: .lock() followed by JWT operations  
echo "Checking for: .lock() → JWT/token generation..."
JWT_VIOLATIONS=$(grep -rn "\.lock()" src/ --include="*.rs" -A 20 | \
  grep -B 5 "generate_token\|encode(\|jwt::" | \
  grep "\.lock()" | \
  grep -v "backup\|\.bak" || true)

if [ -n "$JWT_VIOLATIONS" ]; then
  echo "⚠️  WARNING: Found .lock() before JWT operations:"
  echo "$JWT_VIOLATIONS"
  echo ""
fi

# Pattern 3: .lock() followed by HTTP requests
echo "Checking for: .lock() → HTTP/network calls..."
HTTP_VIOLATIONS=$(grep -rn "\.lock()" src/ --include="*.rs" -A 20 | \
  grep -B 5 "reqwest::\|curl\|http_client" | \
  grep "\.lock()" | \
  grep -v "backup\|\.bak" || true)

if [ -n "$HTTP_VIOLATIONS" ]; then
  echo "⚠️  WARNING: Found .lock() before HTTP calls:"
  echo "$HTTP_VIOLATIONS"
  echo ""
fi

# Pattern 4: .lock() followed by tokio::spawn
echo "Checking for: .lock() → tokio::spawn..."
SPAWN_VIOLATIONS=$(grep -rn "\.lock()" src/ --include="*.rs" -A 10 | \
  grep -B 3 "tokio::spawn" | \
  grep "\.lock()" | \
  grep -v "backup\|\.bak" || true)

if [ -n "$SPAWN_VIOLATIONS" ]; then
  echo "⚠️  WARNING: Found .lock() before tokio::spawn:"
  echo "$SPAWN_VIOLATIONS"
  echo ""
fi

# Pattern 5: MutexGuard not dropped before .await
echo "Checking for: MutexGuard held across .await..."
AWAIT_VIOLATIONS=$(grep -rn "let.*=.*\.lock()" src/ --include="*.rs" -A 10 | \
  grep -B 3 "\.await" | \
  grep "let.*=.*\.lock()" | \
  grep -v "backup\|\.bak\|drop(" || true)

if [ -n "$AWAIT_VIOLATIONS" ]; then
  echo "⚠️  WARNING: Potential MutexGuard held across .await:"
  echo "$AWAIT_VIOLATIONS"
  echo ""
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $VIOLATIONS -gt 0 ]; then
  echo "❌ FAIL: Found $VIOLATIONS critical mutex safety violations"
  echo ""
  echo "Fix by using scoped blocks:"
  echo "  let data = {"
  echo "    let guard = storage.lock().unwrap();"
  echo "    guard.get_data()"
  echo "  }; // guard dropped here"
  echo "  expensive_operation(data); // no lock held"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  exit 1
else
  echo "✅ PASS: No critical mutex safety violations found"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  exit 0
fi
