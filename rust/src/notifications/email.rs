//! Email notifications

use super::*;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use tracing::{debug, error, info};

pub struct EmailNotifier {
    smtp_server: String,
    smtp_username: String,
    smtp_password: String,
    from_address: String,
    to_address: String,
}

impl EmailNotifier {
    pub fn new() -> Result<Self> {
        let smtp_server =
            std::env::var("SMTP_SERVER").unwrap_or_else(|_| "smtp.gmail.com".to_string());
        let smtp_username = std::env::var("SMTP_USERNAME")
            .map_err(|_| crate::Error::Config("SMTP_USERNAME not set".to_string()))?;
        let smtp_password = std::env::var("SMTP_PASSWORD")
            .map_err(|_| crate::Error::Config("SMTP_PASSWORD not set".to_string()))?;
        let from_address = std::env::var("EMAIL_FROM")
            .map_err(|_| crate::Error::Config("EMAIL_FROM not set".to_string()))?;
        let to_address = std::env::var("EMAIL_TO")
            .map_err(|_| crate::Error::Config("EMAIL_TO not set".to_string()))?;

        info!(
            "Email notifier initialized (server: {}, from: {}, to: {})",
            smtp_server, from_address, to_address
        );
        Ok(Self {
            smtp_server,
            smtp_username,
            smtp_password,
            from_address,
            to_address,
        })
    }

    /// Format notification as HTML email
    fn format_html(&self, notification: &Notification) -> String {
        let color = match notification.level {
            NotificationLevel::Info => "#17a2b8",
            NotificationLevel::Warning => "#ffc107",
            NotificationLevel::Error => "#dc3545",
            NotificationLevel::Critical => "#721c24",
        };

        format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; }}
        .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background-color: {}; color: white; padding: 15px; border-radius: 5px; }}
        .content {{ background-color: #f8f9fa; padding: 20px; margin-top: 10px; border-radius: 5px; }}
        .footer {{ color: #6c757d; font-size: 12px; margin-top: 20px; text-align: center; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h2>{}</h2>
        </div>
        <div class="content">
            <p>{}</p>
        </div>
        <div class="footer">
            <p>{}</p>
        </div>
    </div>
</body>
</html>
"#,
            color,
            notification.title,
            notification.message.replace('\n', "<br>"),
            notification.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
}

#[async_trait::async_trait]
impl Notifier for EmailNotifier {
    async fn send(&self, notification: &Notification) -> Result<()> {
        let email = Message::builder()
            .from(
                self.from_address
                    .parse()
                    .map_err(|e| crate::Error::Config(format!("Invalid from address: {}", e)))?,
            )
            .to(self
                .to_address
                .parse()
                .map_err(|e| crate::Error::Config(format!("Invalid to address: {}", e)))?)
            .subject(format!("[Trading Bot] {}", notification.title))
            .header(ContentType::TEXT_HTML)
            .body(self.format_html(notification))
            .map_err(|e| crate::Error::Execution(format!("Failed to build email: {}", e)))?;

        let creds = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

        let mailer = SmtpTransport::relay(&self.smtp_server)
            .map_err(|e| crate::Error::Config(format!("Invalid SMTP server: {}", e)))?
            .credentials(creds)
            .build();

        debug!("Sending email notification: {}", notification.title);

        match mailer.send(&email) {
            Ok(_) => {
                debug!("Email notification sent successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to send email: {}", e);
                Err(crate::Error::Execution(format!(
                    "Email send failed: {}",
                    e
                )))
            }
        }
    }

    fn name(&self) -> &str {
        "Email"
    }
}
