#!/bin/bash
# Smoke test: 10 serial login requests to production API
# Expected: 10/10 successful (100% success rate)

set -e

API_BASE="${API_BASE:-http://localhost:3000}"
USERNAME="${USERNAME:-hen}"
PASSWORD="${PASSWORD:-demo123}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔥 Smoke Test: Sequential Login Requests"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Target: $API_BASE/api/auth/login"
echo "User: $USERNAME"
echo "Count: 10 serial requests"
echo ""

success=0
failed=0
total_ms=0

for i in {1..10}; do
  echo -n "Request $i: "
  START=$(date +%s%N)

  HTTP_CODE=$(curl -sS -o /dev/null -w "%{http_code}" -X POST \
    "$API_BASE/api/auth/login" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\"}" 2>&1)

  END=$(date +%s%N)
  DURATION_MS=$(( (END - START) / 1000000 ))
  total_ms=$((total_ms + DURATION_MS))

  if [ "$HTTP_CODE" = "200" ]; then
    echo "✅ ${HTTP_CODE} (${DURATION_MS}ms)"
    success=$((success + 1))
  else
    echo "❌ ${HTTP_CODE} (${DURATION_MS}ms)"
    failed=$((failed + 1))
  fi

  sleep 0.5
done

avg_ms=$((total_ms / 10))

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "RESULTS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Successful: $success"
echo "Failed: $failed"
echo "Average latency: ${avg_ms}ms"
echo ""

if [ $failed -eq 0 ]; then
  echo "✅ SMOKE TEST PASSED (10/10 successful)"
  exit 0
else
  echo "❌ SMOKE TEST FAILED ($failed failures)"
  exit 1
fi
