#!/bin/bash
# Load test for login endpoint
# Tests concurrent login requests to verify mutex safety pattern effectiveness

set -e

# Configuration
API_BASE="${API_BASE:-https://defarm-engines-api-production.up.railway.app}"
USERNAME="${TEST_USERNAME:-hen}"
PASSWORD="${TEST_PASSWORD:-demo123}"
CONCURRENT_REQUESTS="${CONCURRENT_REQUESTS:-50}"
DURATION_SECONDS="${DURATION_SECONDS:-120}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔥 Login Endpoint Load Test"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Target: $API_BASE/api/auth/login"
echo "Username: $USERNAME"
echo "Concurrent requests: $CONCURRENT_REQUESTS"
echo "Duration: ${DURATION_SECONDS}s"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if API is reachable
echo "🔍 Checking API health..."
if ! curl -s -f "$API_BASE/health" > /dev/null 2>&1; then
    echo "❌ API is not reachable at $API_BASE"
    exit 1
fi
echo "✅ API is healthy"
echo ""

# Create temporary directory for results
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo "🚀 Starting load test..."
echo "Results will be written to: $TEMP_DIR"
echo ""

# Function to perform a single login
perform_login() {
    local request_id=$1
    local start_time=$(date +%s%N)

    response=$(curl -s -w "\n%{http_code}\n%{time_total}" -X POST \
        "$API_BASE/api/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\"}")

    local end_time=$(date +%s%N)
    local duration_ns=$((end_time - start_time))
    local duration_ms=$((duration_ns / 1000000))

    # Parse response (count total lines first for portable head)
    local total_lines=$(echo "$response" | wc -l | tr -d ' ')
    local body_lines=$((total_lines - 2))
    local body=$(echo "$response" | head -n "$body_lines")
    local http_code=$(echo "$response" | tail -n 2 | head -n 1)
    local curl_time=$(echo "$response" | tail -n 1)

    # Write result
    echo "$request_id,$http_code,$duration_ms,$curl_time" >> "$TEMP_DIR/results.csv"

    # Log errors
    if [ "$http_code" != "200" ]; then
        echo "Request $request_id: HTTP $http_code (${duration_ms}ms)" >> "$TEMP_DIR/errors.log"
        echo "$body" >> "$TEMP_DIR/errors.log"
    fi
}

# Initialize results file
echo "request_id,http_code,duration_ms,curl_time" > "$TEMP_DIR/results.csv"

# Run load test
start_time=$(date +%s)
request_count=0

echo "⏰ Test started at $(date)"
echo ""

while true; do
    current_time=$(date +%s)
    elapsed=$((current_time - start_time))

    if [ $elapsed -ge $DURATION_SECONDS ]; then
        break
    fi

    # Launch concurrent requests
    for i in $(seq 1 $CONCURRENT_REQUESTS); do
        perform_login $request_count &
        request_count=$((request_count + 1))
    done

    # Wait for this batch to complete
    wait

    # Progress update every 10 seconds
    if [ $((elapsed % 10)) -eq 0 ]; then
        echo "⏱️  ${elapsed}s elapsed | $request_count requests sent"
    fi

    # Small delay between batches to avoid overwhelming the server
    sleep 0.1
done

echo ""
echo "✅ Test completed at $(date)"
echo ""

# Analyze results
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Results Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

total_requests=$(wc -l < "$TEMP_DIR/results.csv")
total_requests=$((total_requests - 1)) # Subtract header

successful=$(awk -F',' '$2 == 200 { count++ } END { print count+0 }' "$TEMP_DIR/results.csv")
failed=$(awk -F',' '$2 != 200 && NR > 1 { count++ } END { print count+0 }' "$TEMP_DIR/results.csv")

echo "Total requests: $total_requests"
echo "Successful (HTTP 200): $successful"
echo "Failed (non-200): $failed"
echo ""

# Latency percentiles
echo "📈 Latency Statistics (ms):"
sorted_times=$(tail -n +2 "$TEMP_DIR/results.csv" | cut -d',' -f3 | sort -n)

if [ -n "$sorted_times" ]; then
    min=$(echo "$sorted_times" | head -n 1)
    max=$(echo "$sorted_times" | tail -n 1)

    # Calculate percentiles
    p50_index=$(echo "$sorted_times" | wc -l | awk '{print int($1 * 0.50)}')
    p95_index=$(echo "$sorted_times" | wc -l | awk '{print int($1 * 0.95)}')
    p99_index=$(echo "$sorted_times" | wc -l | awk '{print int($1 * 0.99)}')

    p50=$(echo "$sorted_times" | sed -n "${p50_index}p")
    p95=$(echo "$sorted_times" | sed -n "${p95_index}p")
    p99=$(echo "$sorted_times" | sed -n "${p99_index}p")

    avg=$(echo "$sorted_times" | awk '{sum+=$1} END {print int(sum/NR)}')

    echo "  Min: ${min}ms"
    echo "  Avg: ${avg}ms"
    echo "  P50: ${p50}ms"
    echo "  P95: ${p95}ms"
    echo "  P99: ${p99}ms"
    echo "  Max: ${max}ms"
fi

echo ""

# Check for errors
if [ -f "$TEMP_DIR/errors.log" ]; then
    error_count=$(wc -l < "$TEMP_DIR/errors.log")
    echo "⚠️  Found $error_count error entries. Sample errors:"
    head -n 10 "$TEMP_DIR/errors.log"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Load test completed!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Success criteria
if [ "$failed" -eq 0 ] && [ "$p95" -lt 1000 ]; then
    echo "✅ PASS: All requests successful and P95 latency < 1000ms"
    exit 0
elif [ "$failed" -gt $((total_requests / 10)) ]; then
    echo "❌ FAIL: More than 10% of requests failed"
    exit 1
elif [ "$p95" -gt 5000 ]; then
    echo "⚠️  WARNING: P95 latency > 5000ms (possible performance issue)"
    exit 1
else
    echo "✅ PASS: Test completed within acceptable parameters"
    exit 0
fi
