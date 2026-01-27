#!/bin/bash
# DeFarm Engines - Repository Reorganization Script
# This script reorganizes the repository for professional team onboarding

set -e

echo "🚀 DeFarm Engines - Repository Reorganization"
echo "=============================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Confirmation
echo -e "${YELLOW}⚠️  This script will reorganize the repository structure.${NC}"
echo -e "${YELLOW}   A backup branch 'pre-cleanup-backup' will be created first.${NC}"
echo ""
read -p "Do you want to continue? (yes/no): " -r
echo ""

if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    echo "Aborted."
    exit 1
fi

# Step 1: Create backup
echo -e "${GREEN}📦 Step 1: Creating backup branch...${NC}"
git checkout -b pre-cleanup-backup 2>/dev/null || echo "Backup branch already exists"
git add -A
git commit -m "chore: backup before repository reorganization" 2>/dev/null || echo "Nothing to commit"
git checkout main
echo "✓ Backup created: pre-cleanup-backup"
echo ""

# Step 2: Create directory structure
echo -e "${GREEN}📁 Step 2: Creating directory structure...${NC}"
mkdir -p docs/security
mkdir -p docs/guides
mkdir -p docs/implementation
mkdir -p docs/examples
mkdir -p docs/archived/old-attempts
mkdir -p docs/archived/historical
mkdir -p scripts/development
mkdir -p scripts/deployment
mkdir -p scripts/testing
mkdir -p scripts/monitoring
mkdir -p scripts/maintenance
mkdir -p .github/workflows
mkdir -p .github/ISSUE_TEMPLATE
echo "✓ Directories created"
echo ""

# Step 3: Move files to appropriate locations
echo -e "${GREEN}📦 Step 3: Moving files to appropriate locations...${NC}"

# Security documents
echo "  Moving security documents..."
mv SECURITY_CHECKLIST.md docs/security/ 2>/dev/null || true
mv PUBLIC_REPO_SECURITY_SCAN.md docs/security/ 2>/dev/null || true
mv GITHUB_SECRETS_REFERENCE.md docs/security/ 2>/dev/null || true

# Guide documents
echo "  Moving guide documents..."
mv API_KEYS_README.md docs/guides/API_KEYS_GUIDE.md 2>/dev/null || true
mv API_SCHEMAS.md docs/guides/ 2>/dev/null || true
mv EXPLICACAO_API_KEYS_PT.md docs/guides/ 2>/dev/null || true
mv FRONTEND_INTEGRATION_GUIDE.md docs/guides/FRONTEND_INTEGRATION.md 2>/dev/null || true
mv FRONTEND_API_CONFIGURATION.md docs/guides/ 2>/dev/null || true
mv EMAIL_CONFIGURATION.md docs/guides/ 2>/dev/null || true
mv REDIS_MIGRATION_GUIDE.md docs/guides/REDIS_MIGRATION.md 2>/dev/null || true
mv SCALABILITY_SOLUTION.md docs/guides/SCALABILITY.md 2>/dev/null || true

# Implementation documents
echo "  Moving implementation documents..."
mv IMPLEMENTATION_SUMMARY.md docs/implementation/ 2>/dev/null || true

# Historical/archived documents
echo "  Moving historical documents..."
mv REQUEST_TO_BACKBONE_AI.md docs/archived/historical/ 2>/dev/null || true
mv DUAL_REMOTE_*.md docs/archived/historical/ 2>/dev/null || true
mv BACKEND_CORS_*.md docs/archived/historical/ 2>/dev/null || true
mv PERSISTENCE_CHANGES_ANALYSIS.md docs/archived/historical/ 2>/dev/null || true
mv ITEM_PERSISTENCE_ANALYSIS.md docs/archived/historical/ 2>/dev/null || true
mv PROMPT_FIX_ITEMS_PERSISTENCE.md docs/archived/historical/ 2>/dev/null || true
mv CATTLE_ROBOT_RAILWAY_FIX.md docs/archived/historical/ 2>/dev/null || true
mv PASSWORD_RESET_*.md docs/archived/historical/ 2>/dev/null || true
mv ROBOT_SUMMARY.md docs/archived/historical/ 2>/dev/null || true
mv IMMEDIATE_ACTIONS_REQUIRED.md docs/archived/historical/ 2>/dev/null || true
mv RESEARCH_SUMMARY.md docs/archived/historical/ 2>/dev/null || true
mv ARCHITECTURE_MIGRATION_ANALYSIS.md docs/archived/historical/ 2>/dev/null || true
mv IPCM_V2_UPGRADE_SUMMARY.md docs/archived/historical/ 2>/dev/null || true
mv RAILWAY_EVENT_LISTENER_CONFIG.md docs/archived/historical/ 2>/dev/null || true

# Testing scripts
echo "  Moving test scripts..."
mv test-production-evidence.sh scripts/testing/ 2>/dev/null || true
mv test-items-persistence.sh scripts/testing/ 2>/dev/null || true
mv test-stellar-evidence.sh scripts/testing/ 2>/dev/null || true
mv test-api-keys*.sh scripts/testing/ 2>/dev/null || true

# Deployment scripts
echo "  Moving deployment scripts..."
mv setup-event-listener-env.sh scripts/deployment/ 2>/dev/null || true

# Monitoring scripts
echo "  Moving monitoring scripts..."
mv monitor-cattle-robot.sh scripts/monitoring/ 2>/dev/null || true

# Maintenance scripts
echo "  Moving maintenance scripts..."
mv fix-cattle-robot.sh scripts/maintenance/ 2>/dev/null || true

# Archive old attempts
echo "  Archiving old project attempts..."
mv engines-rust-public-verify docs/archived/old-attempts/ 2>/dev/null || true

# Move examples
echo "  Consolidating examples..."
if [ -f "examples/test_bigint_serialization.rs" ]; then
    mkdir -p tests/integration
    mv examples/test_bigint_serialization.rs tests/integration/ 2>/dev/null || true
    rmdir examples 2>/dev/null || true
fi

# Move public demo
echo "  Moving public demos..."
if [ -f "public/demo-circuits.html" ]; then
    mv public/demo-circuits.html docs/examples/ 2>/dev/null || true
    rmdir public 2>/dev/null || true
fi

# Archive hotfix docs
echo "  Archiving hotfix documentation..."
mv docs/hotfix-2025-10-25 docs/archived/ 2>/dev/null || true

echo "✓ Files moved"
echo ""

# Step 4: Create README files for new directories
echo -e "${GREEN}📝 Step 4: Creating README files...${NC}"

# docs/archived/README.md
cat > docs/archived/README.md << 'EOF'
# Archived Documentation

This directory contains historical documentation that is no longer actively maintained but preserved for reference.

## Structure

- `historical/` - Resolved issues, old guides, and completed work
- `old-attempts/` - Previous implementation attempts and experiments
- `hotfix-2025-10-25/` - Historical hotfix documentation

## Note

These documents are kept for historical reference. For current documentation, see the main `/docs` directory.
EOF

# docs/security/README.md
cat > docs/security/README.md << 'EOF'
# Security Documentation

Security guidelines, checklists, and best practices for DeFarm Engines.

## Documents

- `SECURITY_CHECKLIST.md` - Security audit checklist
- `PUBLIC_REPO_SECURITY_SCAN.md` - Public repository security analysis
- `GITHUB_SECRETS_REFERENCE.md` - Secrets management guide

## Reporting Security Issues

Please report security vulnerabilities to: security@defarm.net
EOF

# docs/guides/README.md
cat > docs/guides/README.md << 'EOF'
# Developer Guides

Comprehensive guides for working with DeFarm Engines.

## Available Guides

- `API_KEYS_GUIDE.md` - API key management and usage
- `API_SCHEMAS.md` - API data schemas reference
- `FRONTEND_INTEGRATION.md` - Frontend integration guide
- `EMAIL_CONFIGURATION.md` - Email system setup
- `REDIS_MIGRATION.md` - Redis migration guide
- `SCALABILITY.md` - Scalability solutions and patterns

## Getting Started

New to DeFarm? Start with:
1. Main README.md at repository root
2. `/docs/api/COMPLETE_DEVELOPER_GUIDE.md`
3. `/docs/development/SETUP.md`
EOF

# docs/implementation/README.md
cat > docs/implementation/README.md << 'EOF'
# Implementation Notes

Documentation of major implementations and feature additions.

## Recent Implementations

- `IMPLEMENTATION_SUMMARY.md` - Complete developer ecosystem implementation (2026-01-24)

This directory tracks significant implementations, architectural changes, and feature additions to the DeFarm Engines platform.
EOF

# docs/examples/README.md
cat > docs/examples/README.md << 'EOF'
# Examples

Example code, demos, and sample implementations for DeFarm Engines.

## Available Examples

- `demo-circuits.html` - Interactive circuit demonstration

## Looking for More Examples?

Check out:
- `/docs/api/COMPLETE_DEVELOPER_GUIDE.md` - Complete examples in multiple languages
- `/sdk/typescript/example.ts` - TypeScript SDK example
- `/sdk/python/example.py` - Python SDK example
- `/cli/README.md` - CLI usage examples
EOF

# scripts/README.md
cat > scripts/README.md << 'EOF'
# Scripts

Utility scripts for development, testing, deployment, and maintenance.

## Directory Structure

- `development/` - Development and setup scripts
- `deployment/` - Deployment automation scripts
- `testing/` - Testing and validation scripts
- `monitoring/` - Monitoring and health check scripts
- `maintenance/` - Maintenance and repair scripts

## Usage

All scripts should be run from the repository root:

```bash
./scripts/testing/test-api-keys.sh
./scripts/deployment/setup-event-listener-env.sh
```

## Creating New Scripts

1. Place in appropriate category directory
2. Make executable: `chmod +x script.sh`
3. Add shebang: `#!/bin/bash`
4. Add description header
5. Update this README
EOF

echo "✓ README files created"
echo ""

# Step 5: Update .gitignore
echo -e "${GREEN}🔧 Step 5: Updating .gitignore...${NC}"
cat >> .gitignore << 'EOF'

# Additional ignores from reorganization
**/*.rs.bk
.AppleDouble
.LSOverride
test-output/
docs/_build/
docs/.doctrees/
*.bak
~*
EOF
echo "✓ .gitignore updated"
echo ""

# Step 6: Clean build artifacts
echo -e "${GREEN}🧹 Step 6: Cleaning build artifacts...${NC}"
cargo clean 2>/dev/null || true
rm -rf target/
echo "✓ Build artifacts cleaned"
echo ""

# Summary
echo ""
echo -e "${GREEN}✅ Repository Reorganization Complete!${NC}"
echo ""
echo "Summary:"
echo "  ✓ Backup branch created: pre-cleanup-backup"
echo "  ✓ Directory structure created"
echo "  ✓ Files moved to appropriate locations"
echo "  ✓ README files created"
echo "  ✓ .gitignore updated"
echo "  ✓ Build artifacts cleaned"
echo ""
echo "Next steps:"
echo "  1. Review changes: git status"
echo "  2. Create root README.md (see REPOSITORY_CLEANUP_PLAN.md)"
echo "  3. Create CONTRIBUTING.md"
echo "  4. Commit changes: git add -A && git commit -m 'chore: repository reorganization'"
echo "  5. Push changes: git push origin main"
echo ""
echo -e "${YELLOW}Note: If you need to rollback, use: git checkout pre-cleanup-backup${NC}"
echo ""
