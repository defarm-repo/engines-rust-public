# Email Configuration Guide

This document explains how to configure email sending for password reset functionality using SendGrid.

## Overview

The DeFarm API supports password reset emails via SendGrid. When configured, users will receive professional HTML emails with password reset links. Without configuration, reset tokens are logged to the console for development purposes.

## Environment Variables

### Required for Email Functionality

```bash
# SendGrid API Key (get from https://app.sendgrid.com/settings/api_keys)
SENDGRID_API_KEY=SG.xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

### Optional Configuration

```bash
# Sender email address (default: noreply@defarm.net)
FROM_EMAIL=noreply@your-domain.com

# Sender display name (default: DeFarm Connect)
FROM_NAME="Your Company Name"

# Frontend URL for reset links (default: https://connect.defarm.net)
FRONTEND_URL=https://your-app.com
```

### Development Mode

```bash
# Enable debug logging of reset tokens (default: disabled)
# WARNING: Only use in development! Logs plaintext tokens to console
PASSWORD_RESET_DEBUG_LOG=1
```

## Setup Steps

### 1. Create SendGrid Account

1. Go to [https://sendgrid.com](https://sendgrid.com)
2. Sign up for a free account (100 emails/day free tier)
3. Verify your email address

### 2. Generate API Key

1. Log in to SendGrid dashboard
2. Go to Settings → API Keys
3. Click "Create API Key"
4. Name: "DeFarm Password Reset"
5. Permissions: Select "Full Access" or "Mail Send" only
6. Click "Create & View"
7. **Copy the API key immediately** (you won't see it again!)

### 3. Configure Environment

#### Railway Deployment

1. Go to your Railway project dashboard
2. Navigate to Variables tab
3. Add the following variables:
   ```
   SENDGRID_API_KEY=SG.your_api_key_here
   FROM_EMAIL=noreply@defarm.net
   FRONTEND_URL=https://connect.defarm.net
   ```
4. Save and redeploy

#### Local Development

1. Create `.env` file in project root:
   ```bash
   # Database
   DATABASE_URL=postgresql://localhost/defarm_dev
   JWT_SECRET=your-local-secret-key-for-development-only

   # Email (optional for local dev)
   SENDGRID_API_KEY=SG.your_api_key_here
   FROM_EMAIL=noreply@defarm.net
   FRONTEND_URL=http://localhost:3000

   # Development mode: log tokens instead of sending email
   PASSWORD_RESET_DEBUG_LOG=1
   ```

2. The API will automatically load `.env` on startup

### 4. Verify Sender Domain (Recommended for Production)

For better deliverability in production:

1. Go to SendGrid → Settings → Sender Authentication
2. Click "Authenticate Your Domain"
3. Follow the wizard to add DNS records
4. SendGrid will verify your domain (can take up to 48 hours)

Benefits:
- Higher deliverability rates
- Better spam filter performance
- Professional sender reputation
- DKIM and SPF authentication

## How It Works

### With SendGrid Configured (Production)

1. User requests password reset via `/api/auth/forgot-password`
2. System generates secure token and stores it (hashed)
3. Email sent to user's registered email address
4. Email contains clickable link: `https://your-app.com/reset-password?token=xxx`
5. User clicks link, enters new password
6. System validates token and updates password

### Without SendGrid (Development)

1. User requests password reset
2. System generates token and stores it
3. **Token logged to console/Railway logs**
4. Developer copies token from logs
5. Developer uses token to test reset endpoint

## Email Template

The password reset email includes:

- **HTML Version**: Professionally styled with:
  - Company branding
  - Clear call-to-action button
  - Expiration notice (30 minutes)
  - Security notice
  - Plain text link fallback

- **Plain Text Version**: Simple fallback for email clients that don't support HTML

Example email content:
```
Hello [username],

We received a request to reset your password for your DeFarm Connect account.

Click the button below to reset your password:

[Reset Password Button]

Or copy and paste this link into your browser:
https://connect.defarm.net/reset-password?token=xxx

⏰ This link expires in 30 minutes.

If you didn't request a password reset, you can safely ignore this email.
```

## Testing

### Test Email Sending

```bash
# Make password reset request
curl -X POST https://connect.defarm.net/api/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser"}'

# Expected response
{
  "message": "If the account exists, password reset instructions have been sent."
}
```

### Check SendGrid Activity

1. Go to SendGrid → Activity
2. View email delivery status
3. Check for bounces or errors
4. View email content preview

### Development Testing

```bash
# Set debug mode
export PASSWORD_RESET_DEBUG_LOG=1

# Request reset
curl -X POST http://localhost:3000/api/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser"}'

# Check server logs for token
# Look for: "Password reset token generated for user testuser: [token]"

# Use token to reset password
curl -X POST http://localhost:3000/api/auth/reset-password \
  -H "Content-Type: application/json" \
  -d '{"token":"[token-from-logs]","new_password":"NewPass123!@#"}'
```

## Troubleshooting

### Email Not Received

1. **Check SendGrid Activity**
   - Log in to SendGrid dashboard
   - Go to Activity tab
   - Search for recipient email
   - Check delivery status and errors

2. **Check Spam Folder**
   - Email may be in spam/junk folder
   - Add sender to whitelist

3. **Verify User Email**
   - Ensure user account has valid email address
   - Check for typos in email field

4. **Check API Key**
   - Ensure `SENDGRID_API_KEY` is set correctly
   - Verify API key hasn't expired
   - Check API key permissions include "Mail Send"

### Email Sending Fails

Check server logs for error messages:

```bash
# Railway logs
railway logs

# Look for errors like:
# "Failed to send password reset email to user@example.com: [error]"
```

Common errors:
- `401 Unauthorized`: Invalid API key
- `403 Forbidden`: API key lacks permissions
- `413 Payload Too Large`: Email content too large (unlikely)
- `500 Server Error`: SendGrid service issue

### Development Mode Not Working

If tokens aren't appearing in logs:

1. Verify `PASSWORD_RESET_DEBUG_LOG=1` is set
2. Check log level is INFO or higher
3. Ensure `SENDGRID_API_KEY` is NOT set (otherwise email mode is used)

## Cost and Limits

### SendGrid Free Tier
- 100 emails per day
- Forever free
- Full email template support
- Email Activity Feed (30 days)

### SendGrid Paid Tiers
- **Essentials**: $19.95/month - 50,000 emails/month
- **Pro**: $89.95/month - 100,000 emails/month
- **Premier**: Custom pricing

### Rate Limits
- Password reset: 3 requests per hour per user (enforced by API)
- SendGrid: 600 requests per minute (API limit)

## Security Considerations

1. **Never log tokens in production** (disable `PASSWORD_RESET_DEBUG_LOG`)
2. **Keep SendGrid API key secret** (never commit to git)
3. **Use environment variables** for all sensitive config
4. **Monitor SendGrid activity** for suspicious patterns
5. **Verify sender domain** to prevent spoofing
6. **Token expires in 30 minutes** (enforced by database)
7. **One-time use tokens** (enforced by database)

## Support

- **SendGrid Documentation**: https://docs.sendgrid.com
- **SendGrid Support**: https://support.sendgrid.com
- **DeFarm Issues**: https://github.com/your-org/defarm/issues

## See Also

- [PASSWORD_RESET_IMPLEMENTATION_PLAN.md](./PASSWORD_RESET_IMPLEMENTATION_PLAN.md) - Technical implementation details
- [PASSWORD_RESET_STATUS.md](./PASSWORD_RESET_STATUS.md) - Implementation status
- [src/email_service.rs](./src/email_service.rs) - Email service source code
