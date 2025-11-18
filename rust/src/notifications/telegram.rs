//! Telegram bot notifications

use super::*;
use tracing::info;

pub struct TelegramNotifier {
    bot_token: String,
    chat_id: String,
}

impl TelegramNotifier {
    pub fn new() -> Result<Self> {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| crate::Error::Config("TELEGRAM_BOT_TOKEN not set".to_string()))?;
        let chat_id = std::env::var("TELEGRAM_CHAT_ID")
            .map_err(|_| crate::Error::Config("TELEGRAM_CHAT_ID not set".to_string()))?;

        info!("Telegram notifier initialized");
        Ok(Self { bot_token, chat_id })
    }
}

#[async_trait::async_trait]
impl Notifier for TelegramNotifier {
    async fn send(&self, notification: &Notification) -> Result<()> {
        // TODO: Implement with teloxide or direct API call
        // let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        // Send message to chat_id
        info!("Telegram: {} - {}", notification.title, notification.message);
        Ok(())
    }

    fn name(&self) -> &str {
        "Telegram"
    }
}
