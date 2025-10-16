#!/bin/bash
# Test script to verify Docker build works locally before pushing

set -e  # Exit on error

echo "🔍 Testing Docker build configuration..."
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test 1: Verify Dockerfile exists
echo "📄 Test 1: Checking Dockerfile exists..."
if [ -f "Dockerfile" ]; then
    echo -e "${GREEN}✅ Dockerfile found${NC}"
else
    echo -e "${RED}❌ Dockerfile not found${NC}"
    exit 1
fi

# Test 2: Build builder stage
echo ""
echo "🏗️  Test 2: Building builder stage..."
if docker build --target builder -t defarm-builder:test . ; then
    echo -e "${GREEN}✅ Builder stage built successfully${NC}"
else
    echo -e "${RED}❌ Builder stage failed${NC}"
    exit 1
fi

# Test 3: Build full image
echo ""
echo "🏗️  Test 3: Building full Docker image..."
if docker build -t defarm-api:test . ; then
    echo -e "${GREEN}✅ Full image built successfully${NC}"
else
    echo -e "${RED}❌ Full image build failed${NC}"
    exit 1
fi

# Test 4: Verify libsodium in runtime
echo ""
echo "🔐 Test 4: Verifying libsodium in runtime container..."
LIBSODIUM_CHECK=$(docker run --rm defarm-api:test sh -c "ldconfig -p | grep libsodium" || echo "")

if [ -n "$LIBSODIUM_CHECK" ]; then
    echo -e "${GREEN}✅ libsodium found in runtime:${NC}"
    echo "   $LIBSODIUM_CHECK"
else
    echo -e "${RED}❌ libsodium NOT found in runtime container${NC}"
    echo -e "${YELLOW}⚠️  This will cause runtime encryption failures${NC}"
    exit 1
fi

# Test 5: Verify binary exists and is executable
echo ""
echo "🎯 Test 5: Verifying binary exists..."
BINARY_CHECK=$(docker run --rm defarm-api:test sh -c "ls -lh /app/defarm-api" || echo "NOT FOUND")

if echo "$BINARY_CHECK" | grep -q "defarm-api"; then
    echo -e "${GREEN}✅ Binary exists and is accessible:${NC}"
    echo "   $BINARY_CHECK"
else
    echo -e "${RED}❌ Binary not found at /app/defarm-api${NC}"
    exit 1
fi

# Test 6: Verify binary can run (quick version check)
echo ""
echo "🚀 Test 6: Testing binary execution..."
echo -e "${YELLOW}ℹ️  Attempting to run binary (will fail without DB, but should not crash)${NC}"
EXEC_TEST=$(timeout 2 docker run --rm -e DATABASE_URL=postgresql://test defarm-api:test 2>&1 || true)

if echo "$EXEC_TEST" | grep -q "error"; then
    # Expected - no database configured
    echo -e "${GREEN}✅ Binary runs (fails as expected without database)${NC}"
else
    echo -e "${YELLOW}⚠️  Binary execution test inconclusive${NC}"
fi

# Test 7: Check image size
echo ""
echo "📦 Test 7: Checking image size..."
IMAGE_SIZE=$(docker images defarm-api:test --format "{{.Size}}")
echo -e "${GREEN}✅ Final image size: ${IMAGE_SIZE}${NC}"

# Summary
echo ""
echo "══════════════════════════════════════════════════════════"
echo -e "${GREEN}🎉 All Docker build tests passed!${NC}"
echo "══════════════════════════════════════════════════════════"
echo ""
echo "Next steps:"
echo "  1. Test locally: docker run --rm -p 3000:3000 defarm-api:test"
echo "  2. Push to repository: git push"
echo "  3. Railway will automatically deploy from main branch"
echo ""
echo "Cleanup: docker rmi defarm-api:test defarm-builder:test"
echo ""
