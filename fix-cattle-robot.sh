#!/bin/bash

# Quick verification script for cattle-robot Railway deployment

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🤖 Cattle Robot Deployment Fix Verification"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Check if files exist
echo "✓ Checking local files..."
if [ -f "Dockerfile.cattle-robot" ]; then
    echo "  ✅ Dockerfile.cattle-robot exists"
else
    echo "  ❌ Dockerfile.cattle-robot NOT FOUND"
    exit 1
fi

if [ -f "src/bin/cattle_robot.rs" ]; then
    echo "  ✅ src/bin/cattle_robot.rs exists"
else
    echo "  ❌ src/bin/cattle_robot.rs NOT FOUND"
    exit 1
fi

echo

# Check git status
echo "✓ Checking git status..."
if git diff --quiet HEAD; then
    echo "  ✅ All changes committed"
else
    echo "  ⚠️  Uncommitted changes detected"
fi

# Check if pushed to remote
LOCAL=$(git rev-parse @)
REMOTE=$(git rev-parse @{u} 2>/dev/null)
if [ "$LOCAL" = "$REMOTE" ]; then
    echo "  ✅ Up to date with remote"
else
    echo "  ⚠️  Local commits not pushed (run: git push origin main)"
fi

echo

# Check Railway service
echo "✓ Checking Railway service..."
railway service cattle-robot >/dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "  ✅ Railway cattle-robot service found"
else
    echo "  ⚠️  Could not link to cattle-robot service"
fi

echo

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 NEXT STEPS - Update Railway Dashboard:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
echo "1. Go to: https://railway.app/dashboard"
echo "2. Select: defarm project"
echo "3. Click: cattle-robot service"
echo "4. Go to: Settings tab"
echo
echo "5. In Build section:"
echo "   - Builder: Dockerfile"
echo "   - Dockerfile Path: Dockerfile.cattle-robot"
echo
echo "6. In Deploy section:"
echo "   - Start Command: /app/cattle-robot"
echo
echo "7. Click: Deploy (top right)"
echo
echo "8. Wait 5-10 minutes for build"
echo
echo "9. Check logs should show:"
echo "   🤖 Cattle Robot Starting..."
echo "   ✓ Database connected"
echo "   ✓ API is healthy"
echo "   🚀 Robot is now running"
echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🌟 Stellar Testnet Address:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
echo "GDGKI43T7QJYOIPNFDH64M2LHICXTQO5NOKI523VZJSROKR34AEB5CKE"
echo
echo "View on Stellar Expert:"
echo "https://stellar.expert/explorer/testnet/account/GDGKI43T7QJYOIPNFDH64M2LHICXTQO5NOKI523VZJSROKR34AEB5CKE"
echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
echo "For detailed instructions, see: CATTLE_ROBOT_RAILWAY_FIX.md"
echo
