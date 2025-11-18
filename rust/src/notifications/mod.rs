//! Notification system for alerts and updates
//!
//! Supports multiple channels:
//! - Telegram bot
//! - Email
//! - Discord webhooks
//! - SMS (via Twilio)

use crate::Result;
use serde::{Deserialize, Serialize};

pub mod telegram;
pub mod email;

pub use telegram::TelegramNotifier;
pub use email::EmailNotifier;

/// Notification severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub level: NotificationLevel,
    pub title: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Notifier trait
#[async_trait::async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, notification: &Notification) -> Result<()>;
    fn name(&self) -> &str;
}

/// Notification manager (sends to multiple channels)
pub struct NotificationManager {
    notifiers: Vec<Box<dyn Notifier>>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            notifiers: Vec::new(),
        }
    }

    pub fn add_notifier(&mut self, notifier: Box<dyn Notifier>) {
        self.notifiers.push(notifier);
    }

    pub async fn send(&self, notification: &Notification) -> Result<()> {
        for notifier in &self.notifiers {
            notifier.send(notification).await?;
        }
        Ok(())
    }

    /// Send trade alert
    pub async fn trade_alert(&self, symbol: &str, side: &str, price: f64, pnl: f64) {
        let notification = Notification {
            level: if pnl >= 0.0 { NotificationLevel::Info } else { NotificationLevel::Warning },
            title: format!("Trade Executed: {}", symbol),
            message: format!("{} @ ${:.2}\nP&L: ${:.2}", side, price, pnl),
            timestamp: chrono::Utc::now(),
        };
        let _ = self.send(&notification).await;
    }

    /// Send error alert
    pub async fn error_alert(&self, error: &str) {
        let notification = Notification {
            level: NotificationLevel::Error,
            title: "Trading Error".to_string(),
            message: error.to_string(),
            timestamp: chrono::Utc::now(),
        };
        let _ = self.send(&notification).await;
    }
}
