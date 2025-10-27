━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 DeFarm API Error Analysis
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Fetching logs from Railway (last 10,000 lines)...
⚠️  Warning: Could not fetch Railway logs. Using local log file if available.
Analyzing 8 log lines...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1️⃣  ERROR BREAKDOWN BY KIND
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

No errors with error_kind classification found.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
2️⃣  ERROR BREAKDOWN BY ENDPOINT (Top 10)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

No endpoint information found in error logs.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
3️⃣  HTTP STATUS CODE DISTRIBUTION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

No status code information found in logs.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
4️⃣  TOP 5 ERROR SAMPLES (with trace_id)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

No error samples found.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
5️⃣  ERROR LATENCY STATISTICS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

No duration information found in error logs.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Analysis Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

💡 Next Steps:
  1. Review error_kind breakdown to identify top causes
  2. Check endpoint list to find problematic routes
  3. Use trace_id from samples to debug specific requests
  4. Save this output: ./scripts/analyze_errors.sh > docs/runbooks/errors_breakdown.md

