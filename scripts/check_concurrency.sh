#!/bin/bash
# Concurrency Pattern Enforcement Script
# See: docs/adr/001-concurrency-model.md

set -e

echo "🔍 Checking concurrency patterns..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ERRORS=0

# Check 1: No Arc<RwLock<>> in active source
echo ""
echo "1️⃣  Checking for Arc<RwLock<>> usage..."
if rg -n "Arc<RwLock<" src tests 2>/dev/null | grep -v ".backup" | grep -v ".bak"; then
    echo -e "${RED}❌ Arc<RwLock<>> usage detected!${NC}"
    echo -e "${YELLOW}   Use Arc<Mutex<>> instead. See docs/adr/001-concurrency-model.md${NC}"
    ERRORS=$((ERRORS + 1))
else
    echo -e "${GREEN}✅ No Arc<RwLock<>> found${NC}"
fi

# Check 2: No .read()/.write() in active source
echo ""
echo "2️⃣  Checking for RwLock .read()/.write() usage..."
if rg -n "\.(read|write)\s*\(" src tests 2>/dev/null | grep -v ".backup" | grep -v ".bak"; then
    echo -e "${RED}❌ RwLock .read()/.write() usage detected!${NC}"
    echo -e "${YELLOW}   Use .lock() instead. See docs/adr/001-concurrency-model.md${NC}"
    ERRORS=$((ERRORS + 1))
else
    echo -e "${GREEN}✅ No .read()/.write() calls found${NC}"
fi

# Check 3: No await with lock held (dangerous!)
echo ""
echo "3️⃣  Checking for await with lock held..."
if rg -n "\.lock\(\).*await|await.*\.lock\(\)" src tests 2>/dev/null; then
    echo -e "${RED}❌ CRITICAL: await with lock held detected!${NC}"
    echo -e "${YELLOW}   This can cause deadlocks. Drop lock before .await${NC}"
    ERRORS=$((ERRORS + 1))
else
    echo -e "${GREEN}✅ No await-with-lock patterns found${NC}"
fi

# Summary
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✅ All concurrency checks passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Found $ERRORS concurrency pattern violation(s)${NC}"
    echo -e "${YELLOW}📖 See docs/adr/001-concurrency-model.md for guidance${NC}"
    exit 1
fi
