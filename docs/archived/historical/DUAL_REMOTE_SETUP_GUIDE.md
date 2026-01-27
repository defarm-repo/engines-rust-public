# Quick Start Implementation Guide: GitHub Actions + git-filter-repo

## Phase 1: Initial Setup (30 minutes)

### Step 1: Create Public Repository
1. Go to GitHub.com → New Repository
2. Name: `defarm-rust-engine-public`
3. Description: "Public filtered version (documentation and tests excluded)"
4. Visibility: **Public**
5. **Do NOT initialize with README** (keep completely empty)
6. Create repository

### Step 2: Add Public Remote Locally
```bash
cd /Users/gabrielrondon/rust/engines
git remote add public git@github.com:gabrielrondon/defarm-rust-engine-public.git
git remote -v  # Verify: should show both origin and public
```

### Step 3: Create GitHub Workflow

Create file: `.github/workflows/sync-to-public.yml`

```yaml
name: Sync to Public Repository

on:
  push:
    branches: [ main ]

permissions:
  contents: read

jobs:
  sync-to-public:
    runs-on: ubuntu-latest
    
    steps:
      - name: Checkout code
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
          
      - name: Setup Git
        run: |
          git config --global user.name "GitHub Actions"
          git config --global user.email "action@github.com"
          
      - name: Install git-filter-repo
        run: pip install git-filter-repo
          
      - name: Prepare filtered repo
        run: |
          # Create working directory
          mkdir -p /tmp/sync-work
          cd /tmp/sync-work
          
          # Clone private repo (full history)
          git clone --mirror ${{ github.server_url }}/${{ github.repository }}.git private.git
          
          # Extract filtered copy
          git clone private.git filtered
          cd filtered
          
          # Filter out excluded files
          # This removes .md files, tests/, and docs/ from all commits
          git filter-repo \
            --path-glob '*.md' \
            --path-glob 'tests/*' \
            --path-glob 'docs/*' \
            --invert-paths \
            --force
          
      - name: Push to public remote
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          cd /tmp/sync-work/filtered
          
          # Configure git credentials
          git config --global url."https://x-access-token:${GITHUB_TOKEN}@github.com/".insteadOf "https://github.com/"
          
          # Add public remote and force push
          git remote add public https://github.com/gabrielrondon/defarm-rust-engine-public.git
          git push -f public main
          
      - name: Verify sync
        run: |
          cd /tmp/sync-work/filtered
          
          # Check for any .md files in public repo
          if git log --name-only --pretty=format: | grep -E "\.md$"; then
            echo "ERROR: Found .md files in filtered repo!"
            exit 1
          fi
          
          # Check for tests directory
          if git log --name-only --pretty=format: | grep "^tests/"; then
            echo "ERROR: Found tests/ in filtered repo!"
            exit 1
          fi
          
          # Check for docs directory
          if git log --name-only --pretty=format: | grep "^docs/"; then
            echo "ERROR: Found docs/ in filtered repo!"
            exit 1
          fi
          
          echo "✓ Sync verification passed: all excluded files filtered correctly"
```

### Step 4: Create Initial Push
```bash
# Commit the workflow file
git add .github/workflows/sync-to-public.yml
git commit -m "ci: add workflow to sync filtered content to public repository"
git push origin main
```

### Step 5: Monitor First Sync

1. Go to GitHub repository → Actions tab
2. Watch "Sync to Public Repository" workflow
3. Should complete in 1-2 minutes
4. Check public repo to verify filtered content

### Step 6: Verify Public Repo

```bash
# Clone public repo to verify filtering
cd /tmp
git clone git@github.com:gabrielrondon/defarm-rust-engine-public.git test-public
cd test-public

# These should return NOTHING (empty):
git log --name-only --pretty=format: | grep -c "\.md$"  # Should be: 0
git log --name-only --pretty=format: | grep -c "^tests/"  # Should be: 0
git log --name-only --pretty=format: | grep -c "^docs/"  # Should be: 0

# This should return files (verify src/ is there):
git log --name-only --pretty=format: | grep "^src/" | head -5
```

---

## Phase 2: Ongoing Usage

### Normal Workflow
1. Develop normally in private repo (`origin`)
2. Commit and push: `git push origin main`
3. GitHub Actions automatically syncs to public repo
4. Monitor Actions tab if needed, usually no interaction required

### View Both Remotes
```bash
git remote -v
# origin  git@github.com:gabrielrondon/defarm-rust-engine.git (fetch)
# origin  git@github.com:gabrielrondon/defarm-rust-engine.git (push)
# public  git@github.com:gabrielrondon/defarm-rust-engine-public.git (fetch)
# public  git@github.com:gabrielrondon/defarm-rust-engine-public.git (push)
```

### Emergency Manual Sync
If workflow fails and you need to manually sync:

```bash
# Create filtered version locally
mkdir -p /tmp/emergency-sync
cd /tmp/emergency-sync

git clone --mirror /Users/gabrielrondon/rust/engines/.git private.git
git clone private.git filtered
cd filtered

# Install git-filter-repo if needed
pip install git-filter-repo

# Filter
git filter-repo \
  --path-glob '*.md' \
  --path-glob 'tests/*' \
  --path-glob 'docs/*' \
  --invert-paths \
  --force

# Push
git remote add public git@github.com:gabrielrondon/defarm-rust-engine-public.git
git push -f public main
```

---

## Phase 3: Monitoring & Maintenance

### Weekly Check
```bash
# View recent Actions runs
# https://github.com/gabrielrondon/defarm-rust-engine/actions

# Quick sync status
git fetch public
git rev-parse public/main  # Latest commit in public repo
git rev-parse origin/main  # Latest commit in private repo
# These should match
```

### Monthly Audit
```bash
# Verify no excluded files in public repo
git ls-remote public | wc -l  # Should show refs

# Check sync logs
# Go to Actions tab → Sync to Public Repository → Latest run
```

### If You Need to Change Filter Patterns

1. Edit `.github/workflows/sync-to-public.yml`
2. Update the `git filter-repo` command with new patterns
3. Commit and push
4. Next workflow run uses new patterns
5. Public repo will reflect filtered history with new rules

---

## Fallback: Multiple Push URLs (Optional)

If you want a simple fallback without full filtering:

```bash
# Add public as secondary push target (no filtering)
git remote set-url --add --push origin git@github.com:gabrielrondon/defarm-rust-engine-public.git

# Now "git push origin" pushes to BOTH:
git push origin main

# Verify both have content
git fetch origin
git fetch public
```

**Note**: This doesn't filter files - it just pushes everything to both remotes. Use only if filtering isn't critical.

---

## Troubleshooting

### Workflow fails with "fatal: not a git repository"
**Cause**: Clone depth issue
**Fix**: Ensure `fetch-depth: 0` in checkout step

### Public repo is empty
**Cause**: Push failed silently
**Fix**: Check Actions logs, verify token has write access

### Sync is slow
**Cause**: Full history rewriting
**Fix**: This is expected first run. Subsequent runs are faster as they only update changed commits.

### Need to rewrite public repo history
```bash
# Delete and recreate public repo
# (Only recommended if you made a mistake)

# Then re-run workflow or manually push:
git push -f public main
```

### Want to exclude more files?
```yaml
# In .github/workflows/sync-to-public.yml, modify:
git filter-repo \
  --path-glob '*.md' \
  --path-glob 'tests/*' \
  --path-glob 'docs/*' \
  --path-glob '.env*' \
  --path-glob '*.log' \
  --invert-paths \
  --force
```

---

## Success Criteria

- [ ] Public repository created and accessible
- [ ] Workflow file `.github/workflows/sync-to-public.yml` created
- [ ] Initial workflow run completes successfully
- [ ] Public repo contains only `/src` and other non-excluded directories
- [ ] Public repo has NO .md, tests/, or docs/ files
- [ ] Can verify sync worked: `git fetch public && git rev-parse public/main`
- [ ] Subsequent commits auto-sync (verify in Actions tab)

---

## Timeline

| Task | Duration |
|------|----------|
| Create public repo | 2 min |
| Add remote + workflow | 5 min |
| Commit and push | 2 min |
| First workflow run | 2 min |
| Verification | 3 min |
| **Total** | **14 minutes** |

---

## Support Resources

- **git-filter-repo**: `man git-filter-repo` or https://github.com/newren/git-filter-repo
- **GitHub Actions**: https://docs.github.com/en/actions
- **Git Remotes**: `git remote --help`

