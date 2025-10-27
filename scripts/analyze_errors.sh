#!/bin/bash
# Analyze Railway logs for error patterns and classification

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 DeFarm API Error Analysis"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Fetch recent logs from Railway
echo "Fetching logs from Railway (last 10,000 lines)..."
railway logs 2>&1 | tail -10000 > /tmp/railway_logs.txt || {
    echo "⚠️  Warning: Could not fetch Railway logs. Using local log file if available."
    if [ ! -f /tmp/railway_logs.txt ]; then
        echo "❌ No log file available. Exiting."
        exit 1
    fi
}

echo "Analyzing $(wc -l < /tmp/railway_logs.txt | tr -d ' ') log lines..."
echo ""

# Error breakdown by kind
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "1️⃣  ERROR BREAKDOWN BY KIND"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Extract error_kind field from structured logs
grep -i "error_kind=" /tmp/railway_logs.txt 2>/dev/null | \
  sed 's/.*error_kind="\([^"]*\)".*/\1/' | \
  sort | uniq -c | sort -rn > /tmp/errors_by_kind.txt

if [ -s /tmp/errors_by_kind.txt ]; then
    cat /tmp/errors_by_kind.txt
    echo ""

    TOTAL_ERRORS=$(awk '{sum += $1} END {print sum}' /tmp/errors_by_kind.txt)
    echo "Total errors logged: $TOTAL_ERRORS"
else
    echo "No errors with error_kind classification found."
fi

echo ""

# Error breakdown by endpoint
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "2️⃣  ERROR BREAKDOWN BY ENDPOINT (Top 10)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

grep -i "error_kind=" /tmp/railway_logs.txt 2>/dev/null | \
  sed 's/.*endpoint="\([^"]*\)".*/\1/' | \
  sort | uniq -c | sort -rn | head -10 > /tmp/errors_by_endpoint.txt

if [ -s /tmp/errors_by_endpoint.txt ]; then
    cat /tmp/errors_by_endpoint.txt
else
    echo "No endpoint information found in error logs."
fi

echo ""

# Status code distribution
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "3️⃣  HTTP STATUS CODE DISTRIBUTION"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

grep -i "status_code=" /tmp/railway_logs.txt 2>/dev/null | \
  sed 's/.*status_code=\([0-9]*\).*/\1/' | \
  sort | uniq -c | sort -rn > /tmp/errors_by_status.txt

if [ -s /tmp/errors_by_status.txt ]; then
    cat /tmp/errors_by_status.txt
    echo ""

    ERRORS_5XX=$(grep -E ' (5[0-9]{2})$' /tmp/errors_by_status.txt | awk '{sum += $1} END {print sum+0}')
    ERRORS_4XX=$(grep -E ' (4[0-9]{2})$' /tmp/errors_by_status.txt | awk '{sum += $1} END {print sum+0}')

    echo "5xx (Server Errors): $ERRORS_5XX"
    echo "4xx (Client Errors): $ERRORS_4XX"
else
    echo "No status code information found in logs."
fi

echo ""

# Top 5 error samples with trace_id
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "4️⃣  TOP 5 ERROR SAMPLES (with trace_id)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

grep -i "error_kind=" /tmp/railway_logs.txt 2>/dev/null | head -5 > /tmp/error_samples.txt

if [ -s /tmp/error_samples.txt ]; then
    SAMPLE_NUM=1
    while IFS= read -r line; do
        echo "Sample #$SAMPLE_NUM:"
        echo "$line" | sed 's/.*trace_id="\([^"]*\)".*/  trace_id: \1/'
        echo "$line" | sed 's/.*error_kind="\([^"]*\)".*/  kind: \1/'
        echo "$line" | sed 's/.*endpoint="\([^"]*\)".*/  endpoint: \1/'
        echo "$line" | sed 's/.*message="\([^"]*\)".*/  message: \1/'
        echo ""
        SAMPLE_NUM=$((SAMPLE_NUM + 1))
    done < /tmp/error_samples.txt
else
    echo "No error samples found."
fi

# Performance metrics (if duration_ms is logged)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "5️⃣  ERROR LATENCY STATISTICS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

grep -i "error_kind=.*duration_ms=" /tmp/railway_logs.txt 2>/dev/null | \
  sed 's/.*duration_ms=\([0-9]*\).*/\1/' | \
  sort -n > /tmp/error_durations.txt

if [ -s /tmp/error_durations.txt ]; then
    COUNT=$(wc -l < /tmp/error_durations.txt | tr -d ' ')
    MIN=$(head -n 1 /tmp/error_durations.txt)
    MAX=$(tail -n 1 /tmp/error_durations.txt)
    AVG=$(awk '{sum+=$1} END {print int(sum/NR)}' /tmp/error_durations.txt)

    P50_LINE=$(( COUNT * 50 / 100 ))
    P95_LINE=$(( COUNT * 95 / 100 ))
    P99_LINE=$(( COUNT * 99 / 100 ))

    P50=$(sed -n "${P50_LINE}p" /tmp/error_durations.txt)
    P95=$(sed -n "${P95_LINE}p" /tmp/error_durations.txt)
    P99=$(sed -n "${P99_LINE}p" /tmp/error_durations.txt)

    echo "Error request durations (ms):"
    echo "  Count: $COUNT"
    echo "  Min: ${MIN}ms"
    echo "  Avg: ${AVG}ms"
    echo "  P50: ${P50}ms"
    echo "  P95: ${P95}ms"
    echo "  P99: ${P99}ms"
    echo "  Max: ${MAX}ms"
else
    echo "No duration information found in error logs."
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Analysis Complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 Next Steps:"
echo "  1. Review error_kind breakdown to identify top causes"
echo "  2. Check endpoint list to find problematic routes"
echo "  3. Use trace_id from samples to debug specific requests"
echo "  4. Save this output: ./scripts/analyze_errors.sh > docs/runbooks/errors_breakdown.md"
echo ""
