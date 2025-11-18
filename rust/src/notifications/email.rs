//! Email notifications

use super::*;
use tracing::info;

pub struct EmailNotifier {
    smtp_server: String,
    from_address: String,
    to_address: String,
}

impl EmailNotifier {
    pub fn new() -> Result<Self> {
        let smtp_server = std::env::var("SMTP_SERVER").unwrap_or_else(|_| "smtp.gmail.com".to_string());
        let from_address = std::env::var("EMAIL_FROM")
            .map_err(|_| crate::Error::Config("EMAIL_FROM not set".to_string()))?;
        let to_address = std::env::var("EMAIL_TO")
            .map_err(|_| crate::Error::Config("EMAIL_TO not set".to_string()))?;

        info!("Email notifier initialized");
        Ok(Self {
            smtp_server,
            from_address,
            to_address,
        })
    }
}

#[async_trait::async_trait]
impl Notifier for EmailNotifier {
    async fn send(&self, notification: &Notification) -> Result<()> {
        // TODO: Implement with lettre crate
        // Build email with SMTP transport
        info!("Email: {} - {}", notification.title, notification.message);
        Ok(())
    }

    fn name(&self) -> &str {
        "Email"
    }
}
