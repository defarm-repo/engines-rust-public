# 🚨 IMMEDIATE ACTIONS REQUIRED - CREDENTIAL EXPOSURE

## ⚠️ SEVERITY: CRITICAL
**Time Sensitive:** These credentials are publicly visible RIGHT NOW.

## 📍 WHAT WAS EXPOSED

### Primary Exposure (Commit f18fac8a)
**Location:** Commit message in public repository
**URL:** https://github.com/defarm-repo/engines-rust-public/commit/f18fac8a8d56503e26b4d1288ccf7bca507850e5

**Exposed Credentials:**
```
SMTP_USERNAME: MS_AVw21D@test-vz9dlem2ypp4kj50.mlsender.net
SMTP_PASSWORD: mssp.VZq5m3R.ynrw7gy3rdkl2k8e.4f81rCb
```

### Additional Credential Patterns Found
- GitHub Personal Access Tokens (ghp_*) in multiple commits
- MailerSend API key patterns (mlsn.*) in documentation

## ✅ ACTION CHECKLIST - DO IN ORDER

### [ ] STEP 1: REVOKE MAILERSEND CREDENTIALS (DO FIRST!)

**MailerSend Dashboard:** https://app.mailersend.com/

1. **Login to MailerSend**
   - Navigate to dashboard

2. **Revoke API Token**
   - Go to: Settings → API Tokens
   - Find any token starting with `mlsn.`
   - Click "Delete" or "Regenerate"
   - Confirm deletion

3. **Reset SMTP Password**
   - Go to: Settings → SMTP & API
   - Find SMTP user: `MS_AVw21D@test-vz9dlem2ypp4kj50.mlsender.net`
   - Click "Reset Password" or "Delete User"
   - Generate new SMTP credentials if needed

4. **Check Access Logs** (if available)
   - Go to: Activity → Logs
   - Look for suspicious activity since Nov 2, 2025
   - Check for unauthorized email sends

5. **Verify Account Status**
   - Confirm your account isn't paused
   - Check email sending quotas haven't been exceeded

---

### [ ] STEP 2: CHECK RAILWAY CREDENTIALS

Before generating new credentials, verify what Railway is currently using.

**Railway Dashboard:** https://railway.app/

1. Navigate to your project: `defarm-engines-api-production`
2. Go to: Variables tab
3. Check these variables:
   - `MAILERSEND_API_KEY` - Current value?
   - `SMTP_USERNAME` - Current value?
   - `SMTP_PASSWORD` - Current value?

**Question:** Are the exposed credentials the same as what Railway is using?
- If YES → Production is now broken (API needs immediate update)
- If NO → Production is safe (exposed credentials were test/old ones)

---

### [ ] STEP 3: GENERATE NEW MAILERSEND CREDENTIALS

Only after revoking the old ones:

**In MailerSend Dashboard:**

1. **Generate New API Token**
   - Go to: Settings → API Tokens
   - Click "Create Token"
   - Name: "DeFarm Production API - Nov 2025"
   - Permissions: Full access or Email sending only
   - Copy the token (starts with `mlsn.`)
   - **Save it securely** - you won't see it again!

2. **Generate New SMTP Credentials**
   - Go to: Settings → SMTP & API
   - Click "Add SMTP User" or use existing domain
   - Generate new password
   - Copy:
     - SMTP Host: `smtp.mailersend.net`
     - SMTP Port: `587` (or `2525`)
     - SMTP Username: `MS_xxx@your-domain.mlsender.net`
     - SMTP Password: `mssp.xxx`
   - **Save these securely!**

---

### [ ] STEP 4: UPDATE RAILWAY ENVIRONMENT VARIABLES

**In Railway Dashboard:**

1. Go to your project variables
2. Update these values with NEW credentials:

```bash
MAILERSEND_API_KEY=mlsn.YOUR_NEW_TOKEN_HERE
SMTP_HOST=smtp.mailersend.net
SMTP_PORT=587
SMTP_USERNAME=MS_YOUR_NEW_USERNAME@domain.mlsender.net
SMTP_PASSWORD=mssp.YOUR_NEW_PASSWORD_HERE
```

3. Save changes
4. Railway will automatically redeploy

**Wait for deployment to complete before testing.**

---

### [ ] STEP 5: TEST EMAIL FUNCTIONALITY

After Railway redeploys:

```bash
# Test password reset email
curl -X POST "https://defarm-engines-api-production.up.railway.app/api/auth/forgot-password" \
  -H "Content-Type: application/json" \
  -d '{"email":"your-test-email@example.com"}'
```

Expected: Email sent successfully
If fails: Check Railway logs for MailerSend API errors

---

### [ ] STEP 6: WAIT FOR GIT HISTORY CLEANUP

I will prepare a script to:
1. Rewrite git history to remove credentials from commit messages
2. Force push cleaned history to both repositories
3. Update security scanner to prevent this in future

**DO NOT** make any git commits until history cleanup is complete.

---

## 🔍 ADDITIONAL EXPOSURES FOUND

These may be false positives (example credentials in docs) but need verification:

1. **Commit 259f29d** - Contains github_pat_ pattern
2. **Commit 7ec9b6449** - Contains ghp_ pattern
3. **Commit 6088158** - Contains ghp_ pattern

**Action:** I will review these commits to determine if real tokens were exposed.

---

## 📞 NEXT STEPS

After completing Steps 1-5 above, reply with:
- ✅ MailerSend credentials revoked
- ✅ New credentials generated
- ✅ Railway updated and redeployed
- ✅ Email functionality tested

Then I will proceed with git history cleanup.

---

## ⏰ TIME ESTIMATE

- Steps 1-5: ~15-20 minutes
- Git history cleanup: ~10 minutes (automated)
- Total: ~30 minutes to full resolution

---

## 🆘 IF YOU NEED HELP

Stop and let me know which step you're stuck on. We'll work through it together.
