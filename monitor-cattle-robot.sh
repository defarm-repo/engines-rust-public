#!/bin/bash

# Monitor cattle-robot deployment and verify it's running correctly

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔍 Cattle Robot Deployment Monitor"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Check Railway service
echo "📡 Checking Railway deployment status..."
railway status --service cattle-robot 2>&1 | grep -E "Status|Deployment"
echo

# Get latest logs
echo "📋 Latest logs (last 20 lines)..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
railway logs --service cattle-robot 2>&1 | tail -20
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Check for correct binary
if railway logs --service cattle-robot 2>&1 | grep -q "🤖 Cattle Robot Starting"; then
    echo "✅ SUCCESS! Cattle robot is running correctly!"
    echo
    echo "Next steps:"
    echo "1. Wait 5-120 minutes for first mint operation"
    echo "2. Monitor Stellar address for transactions:"
    echo "   https://stellar.expert/explorer/testnet/account/GDGKI43T7QJYOIPNFDH64M2LHICXTQO5NOKI523VZJSROKR34AEB5CKE"
    echo
elif railway logs --service cattle-robot 2>&1 | grep -q "defarm_api"; then
    echo "❌ PROBLEM: Still running defarm-api instead of cattle-robot"
    echo
    echo "Try manual redeploy:"
    echo "1. Go to Railway dashboard"
    echo "2. Click cattle-robot service"
    echo "3. Click Deployments tab"
    echo "4. Click 'Redeploy' on latest deployment"
    echo
else
    echo "⚠️  Status unclear. Check logs above."
    echo
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🌟 Stellar Testnet Address:"
echo "GDGKI43T7QJYOIPNFDH64M2LHICXTQO5NOKI523VZJSROKR34AEB5CKE"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
