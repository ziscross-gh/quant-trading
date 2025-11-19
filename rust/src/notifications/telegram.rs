//! Telegram bot notifications

use super::*;
use reqwest::Client;
use serde::Serialize;
use tracing::{debug, error, info};

pub struct TelegramNotifier {
    client: Client,
    bot_token: String,
    chat_id: String,
}

#[derive(Serialize)]
struct TelegramMessage {
    chat_id: String,
    text: String,
    parse_mode: String,
}

impl TelegramNotifier {
    pub fn new() -> Result<Self> {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| crate::Error::Config("TELEGRAM_BOT_TOKEN not set".to_string()))?;
        let chat_id = std::env::var("TELEGRAM_CHAT_ID")
            .map_err(|_| crate::Error::Config("TELEGRAM_CHAT_ID not set".to_string()))?;

        info!("Telegram notifier initialized (chat: {})", chat_id);
        Ok(Self {
            client: Client::new(),
            bot_token,
            chat_id,
        })
    }

    /// Format notification as Telegram message with emoji
    fn format_message(&self, notification: &Notification) -> String {
        let emoji = match notification.level {
            NotificationLevel::Info => "ℹ️",
            NotificationLevel::Warning => "⚠️",
            NotificationLevel::Error => "🔴",
            NotificationLevel::Critical => "🚨",
        };

        format!(
            "{} *{}*\n\n{}\n\n{}",
            emoji,
            notification.title,
            notification.message,
            notification.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
}

#[async_trait::async_trait]
impl Notifier for TelegramNotifier {
    async fn send(&self, notification: &Notification) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let message = TelegramMessage {
            chat_id: self.chat_id.clone(),
            text: self.format_message(notification),
            parse_mode: "Markdown".to_string(),
        };

        debug!("Sending Telegram notification: {}", notification.title);

        match self.client.post(&url).json(&message).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    debug!("Telegram notification sent successfully");
                    Ok(())
                } else {
                    let status = response.status();
                    let error_text = response.text().await.unwrap_or_default();
                    error!("Telegram API error {}: {}", status, error_text);
                    Err(crate::Error::Execution(format!(
                        "Telegram send failed: {} - {}",
                        status, error_text
                    )))
                }
            }
            Err(e) => {
                error!("Failed to send Telegram notification: {}", e);
                Err(crate::Error::Execution(format!(
                    "Telegram request failed: {}",
                    e
                )))
            }
        }
    }

    fn name(&self) -> &str {
        "Telegram"
    }
}
