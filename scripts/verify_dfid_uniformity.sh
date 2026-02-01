#!/bin/bash
# Verify DFID uniformity across the codebase
# Ensures all engines use consistent concurrency model

set -e

echo "🔍 Verifying DFID uniformity..."

# Check for DfidEngine usage
echo ""
echo "📋 Checking DfidEngine usage..."
DFID_ENGINE_COUNT=$(grep -r "dfid_engine:" src/ --include="*.rs" | wc -l | tr -d ' ')
echo "   Found $DFID_ENGINE_COUNT DfidEngine field declarations"

# Check for DfidClient usage
echo ""
echo "📋 Checking DfidClient usage..."
DFID_CLIENT_COUNT=$(grep -r "dfid_client:" src/ --include="*.rs" | wc -l | tr -d ' ')
echo "   Found $DFID_CLIENT_COUNT DfidClient field declarations"

# Check for generate_dfid calls
echo ""
echo "📋 Checking generate_dfid() calls..."
GENERATE_CALLS=$(grep -r "generate_dfid()" src/ --include="*.rs" | grep -v "pub fn\|pub async fn\|test" | wc -l | tr -d ' ')
echo "   Found $GENERATE_CALLS direct generate_dfid() calls"

# Check for generate_dfid_internal calls
echo ""
echo "📋 Checking generate_dfid_internal() calls..."
INTERNAL_CALLS=$(grep -r "generate_dfid_internal()" src/ --include="*.rs" | grep -v "pub fn\|pub async fn" | wc -l | tr -d ' ')
echo "   Found $INTERNAL_CALLS generate_dfid_internal() calls (should use .await)"

# Check for missing .await on async DFID calls
echo ""
echo "📋 Checking for missing .await on async DFID calls..."
MISSING_AWAIT=$(grep -r "generate_dfid_internal()" src/ --include="*.rs" | grep -v "\.await" | grep -v "pub fn\|pub async fn" | wc -l | tr -d ' ')
if [ "$MISSING_AWAIT" -gt "0" ]; then
    echo "   ❌ ERROR: Found $MISSING_AWAIT calls missing .await"
    grep -rn "generate_dfid_internal()" src/ --include="*.rs" | grep -v "\.await" | grep -v "pub fn\|pub async fn"
    exit 1
else
    echo "   ✅ All async DFID calls have .await"
fi

# Check for with_dfid_client pattern
echo ""
echo "📋 Checking with_dfid_client() pattern..."
WITH_CLIENT_COUNT=$(grep -r "with_dfid_client" src/ --include="*.rs" | grep -v "pub fn" | wc -l | tr -d ' ')
echo "   Found $WITH_CLIENT_COUNT with_dfid_client() usage"

# Verify ItemsEngine has both fields
echo ""
echo "📋 Verifying ItemsEngine hybrid mode..."
if grep -q "dfid_engine: DfidEngine," src/items_engine.rs && grep -q "dfid_client: Option<DfidClient>," src/items_engine.rs; then
    echo "   ✅ ItemsEngine has hybrid mode (both DfidEngine and Option<DfidClient>)"
else
    echo "   ❌ ERROR: ItemsEngine missing hybrid mode fields"
    exit 1
fi

# Verify CircuitsEngine has both fields
echo ""
echo "📋 Verifying CircuitsEngine hybrid mode..."
if grep -q "dfid_engine: DfidEngine," src/circuits_engine.rs && grep -q "dfid_client: Option<DfidClient>," src/circuits_engine.rs; then
    echo "   ✅ CircuitsEngine has hybrid mode (both DfidEngine and Option<DfidClient>)"
else
    echo "   ❌ ERROR: CircuitsEngine missing hybrid mode fields"
    exit 1
fi

# Check AppState configuration
echo ""
echo "📋 Verifying AppState configuration..."
if grep -q "dfid_client: Option<DfidClient>" src/api/shared_state.rs; then
    echo "   ✅ AppState has optional DfidClient field"
else
    echo "   ❌ ERROR: AppState missing dfid_client field"
    exit 1
fi

# Check bin/api.rs configuration
echo ""
echo "📋 Verifying bin/api.rs DFID client setup..."
if grep -q "DFID_SERVICE_URL" src/bin/api.rs; then
    echo "   ✅ bin/api.rs configures DFID Service URL"
else
    echo "   ❌ ERROR: bin/api.rs missing DFID_SERVICE_URL configuration"
    exit 1
fi

echo ""
echo "✅ DFID uniformity verification complete!"
echo ""
echo "Summary:"
echo "  - DfidEngine fields: $DFID_ENGINE_COUNT"
echo "  - DfidClient fields: $DFID_CLIENT_COUNT"
echo "  - Direct generate_dfid calls: $GENERATE_CALLS"
echo "  - Internal generate_dfid calls: $INTERNAL_CALLS"
echo "  - with_dfid_client usage: $WITH_CLIENT_COUNT"
echo ""
echo "✨ All checks passed! DFID architecture is uniform."
