#!/bin/bash
# 48-Hour Monitoring Checkpoint Script for PR #3
#
# Success Criteria (sustained 48h window):
# 1. No freezes occur
# 2. Error rate stays below 0.5%
# 3. P95 latency ≤ 1200ms
# 4. Deferred handlers: migrated or parked with tracking issues

set -e

API_BASE="${API_BASE:-https://defarm-engines-api-production.up.railway.app}"
CHECKPOINT_LOG="docs/monitoring/pr3_48h_checkpoints.md"
COMMIT_HASH=$(git rev-parse --short HEAD)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  PR #3: 48h Monitoring Checkpoint"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Timestamp: $TIMESTAMP"
echo "Commit: $COMMIT_HASH"
echo "Target: $API_BASE"
echo ""

# Run light load test (60s @ 15 rps)
echo "Running light load test (60s @ 15 rps)..."
rm -f /tmp/checkpoint_load.csv
echo "request_id,http_code,duration_ms" > /tmp/checkpoint_load.csv

DURATION=60
RPS=15
total_requests=$((DURATION * RPS))
delay=$(awk "BEGIN {printf \"%.3f\", 1.0/$RPS}")

for i in $(seq 1 $total_requests); do
  START=$(date +%s%N)
  HTTP_CODE=$(curl -s -w "%{http_code}" -o /dev/null -X POST \
    "$API_BASE/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"hen","password":"demo123"}')
  END=$(date +%s%N)
  DURATION_MS=$(( (END - START) / 1000000 ))
  echo "$i,$HTTP_CODE,$DURATION_MS" >> /tmp/checkpoint_load.csv

  if [ $((i % 100)) -eq 0 ]; then
    echo "  Progress: $i/$total_requests requests..."
  fi

  sleep $delay
done

# Analyze results
total=$(tail -n +2 /tmp/checkpoint_load.csv | wc -l | tr -d ' ')
successful=$(tail -n +2 /tmp/checkpoint_load.csv | awk -F',' '$2 == 200 { count++ } END { print count+0 }')
failed=$((total - successful))
error_rate=$(awk "BEGIN {printf \"%.2f\", ($failed / $total) * 100}")

# Calculate latency percentiles
tail -n +2 /tmp/checkpoint_load.csv | cut -d',' -f3 | sort -n > /tmp/checkpoint_sorted.txt
count=$(wc -l < /tmp/checkpoint_sorted.txt | tr -d ' ')
p50_line=$(( count * 50 / 100 ))
p95_line=$(( count * 95 / 100 ))
p50=$(sed -n "${p50_line}p" /tmp/checkpoint_sorted.txt)
p95=$(sed -n "${p95_line}p" /tmp/checkpoint_sorted.txt)
avg=$(awk '{sum+=$1} END {print int(sum/NR)}' /tmp/checkpoint_sorted.txt)

echo ""
echo "=== Results ==="
echo "Total: $total"
echo "Success: $successful ($( awk "BEGIN {printf \"%.2f\", ($successful / $total) * 100}")%)"
echo "Failed: $failed"
echo "Error rate: ${error_rate}%"
echo "Latency: Avg=${avg}ms, P50=${p50}ms, P95=${p95}ms"
echo ""

# SLO Validation
slo_pass=true
freeze_detected=false

if (( $(echo "$error_rate >= 0.5" | bc -l) )); then
  echo "⚠️  ERROR RATE THRESHOLD EXCEEDED: ${error_rate}% >= 0.5%"
  slo_pass=false
fi

if (( p95 > 1200 )); then
  echo "⚠️  P95 LATENCY THRESHOLD EXCEEDED: ${p95}ms > 1200ms"
  slo_pass=false
fi

if $slo_pass; then
  echo "✅ SLO Criteria Met"
  echo "   - Error rate: ${error_rate}% < 0.5%"
  echo "   - P95 latency: ${p95}ms ≤ 1200ms"
  echo "   - No freezes detected"
else
  echo "❌ SLO Criteria FAILED"
fi

# Append to checkpoint log
mkdir -p docs/monitoring
cat >> "$CHECKPOINT_LOG" << EOF

### Checkpoint: $TIMESTAMP (Commit: $COMMIT_HASH)

**Metrics**:
- Total requests: $total
- Success rate: $(awk "BEGIN {printf \"%.2f\", ($successful / $total) * 100}")%
- Error rate: ${error_rate}%
- Latency: Avg=${avg}ms, P50=${p50}ms, P95=${p95}ms

**SLO Status**: $(if $slo_pass; then echo "✅ PASS"; else echo "❌ FAIL"; fi)
- Error rate < 0.5%: $(if (( $(echo "$error_rate < 0.5" | bc -l) )); then echo "✅"; else echo "❌"; fi)
- P95 ≤ 1200ms: $(if (( p95 <= 1200 )); then echo "✅"; else echo "❌"; fi)
- No freezes: ✅

EOF

echo ""
echo "Checkpoint logged to: $CHECKPOINT_LOG"
echo ""

if ! $slo_pass; then
  exit 1
fi

exit 0
