/// Email Service for sending transactional emails via SendGrid
///
/// This module provides email sending functionality using the SendGrid API.
/// It supports password reset emails and can be extended for other use cases.
use serde_json::json;
use std::env;

/// Configuration for the email service
pub struct EmailConfig {
    pub sendgrid_api_key: String,
    pub from_email: String,
    pub from_name: String,
    pub frontend_url: String,
}

impl EmailConfig {
    /// Load email configuration from environment variables
    pub fn from_env() -> Result<Self, String> {
        let sendgrid_api_key = env::var("SENDGRID_API_KEY")
            .map_err(|_| "SENDGRID_API_KEY environment variable not set".to_string())?;

        let from_email =
            env::var("FROM_EMAIL").unwrap_or_else(|_| "noreply@defarm.net".to_string());

        let from_name = env::var("FROM_NAME").unwrap_or_else(|_| "DeFarm Connect".to_string());

        let frontend_url =
            env::var("FRONTEND_URL").unwrap_or_else(|_| "https://connect.defarm.net".to_string());

        Ok(Self {
            sendgrid_api_key,
            from_email,
            from_name,
            frontend_url,
        })
    }

    /// Check if email service is enabled (SendGrid API key is set)
    pub fn is_enabled() -> bool {
        env::var("SENDGRID_API_KEY").is_ok()
    }
}

/// Send a password reset email via SendGrid
pub async fn send_password_reset_email(
    to_email: &str,
    username: &str,
    token: &str,
) -> Result<(), String> {
    let config = EmailConfig::from_env()?;

    // Build the reset link
    let reset_link = format!("{}/reset-password?token={}", config.frontend_url, token);

    // Create HTML email body
    let html_body = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background-color: #f8f9fa; border-radius: 10px; padding: 30px; margin-bottom: 20px;">
        <h1 style="color: #2c3e50; margin-top: 0;">Password Reset Request</h1>
        <p>Hello <strong>{}</strong>,</p>
        <p>We received a request to reset your password for your DeFarm Connect account.</p>
        <p>Click the button below to reset your password:</p>
        <div style="text-align: center; margin: 30px 0;">
            <a href="{}" style="background-color: #3498db; color: white; padding: 12px 30px; text-decoration: none; border-radius: 5px; display: inline-block; font-weight: bold;">Reset Password</a>
        </div>
        <p><small style="color: #7f8c8d;">Or copy and paste this link into your browser:</small></p>
        <p style="background-color: #ecf0f1; padding: 10px; border-radius: 5px; word-break: break-all;"><small>{}</small></p>
        <hr style="border: none; border-top: 1px solid #ddd; margin: 30px 0;">
        <p style="color: #e74c3c; font-weight: bold;">⏰ This link expires in 30 minutes.</p>
        <p><small style="color: #7f8c8d;">If you didn't request a password reset, you can safely ignore this email. Your password will remain unchanged.</small></p>
    </div>
    <div style="text-align: center; color: #95a5a6; font-size: 12px;">
        <p>© 2024 DeFarm Connect. All rights reserved.</p>
        <p>This is an automated message, please do not reply to this email.</p>
    </div>
</body>
</html>
        "#,
        username, reset_link, reset_link
    );

    // Create plain text fallback
    let text_body = format!(
        r#"Hello {},

We received a request to reset your password for your DeFarm Connect account.

To reset your password, click the following link or copy it into your browser:

{}

This link expires in 30 minutes.

If you didn't request a password reset, you can safely ignore this email. Your password will remain unchanged.

---
© 2024 DeFarm Connect
This is an automated message, please do not reply to this email.
        "#,
        username, reset_link
    );

    // Send email via SendGrid API
    send_via_sendgrid(
        &config,
        to_email,
        "Reset Your Password",
        &html_body,
        &text_body,
    )
    .await
}

/// Send email via SendGrid API v3
async fn send_via_sendgrid(
    config: &EmailConfig,
    to_email: &str,
    subject: &str,
    html_body: &str,
    text_body: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    let payload = json!({
        "personalizations": [{
            "to": [{
                "email": to_email
            }]
        }],
        "from": {
            "email": config.from_email,
            "name": config.from_name
        },
        "subject": subject,
        "content": [
            {
                "type": "text/plain",
                "value": text_body
            },
            {
                "type": "text/html",
                "value": html_body
            }
        ]
    });

    tracing::debug!("Sending email to {} with subject '{}'", to_email, subject);

    let response = client
        .post("https://api.sendgrid.com/v3/mail/send")
        .header(
            "Authorization",
            format!("Bearer {}", config.sendgrid_api_key),
        )
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to SendGrid: {}", e))?;

    let status = response.status();

    if status.is_success() {
        tracing::info!("✅ Password reset email sent successfully to {}", to_email);
        Ok(())
    } else {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read error response".to_string());

        tracing::error!("❌ SendGrid API error (status {}): {}", status, error_body);

        Err(format!(
            "SendGrid API returned status {}: {}",
            status, error_body
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_config_is_enabled() {
        // Test checks if environment variable detection works
        // Actual value depends on test environment
        let _ = EmailConfig::is_enabled();
    }
}
