// Production Monitoring and Alerting Module
//
// Comprehensive monitoring system for live trading operations.
// Tracks performance, errors, health status, and sends alerts via Telegram.
//
// Key Features:
// - Real-time performance metrics collection
// - Telegram notifications for trades, alerts, and errors
// - Health checks and system status monitoring
// - Trade execution tracking
// - Error detection and alerting
// - Daily/weekly performance reports

use crate::{
    types::{Position, Signal, Trade},
    Error, Result,
};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Configuration for monitoring and alerting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Telegram bot token
    pub telegram_token: Option<String>,

    /// Telegram chat ID for alerts
    pub telegram_chat_id: Option<String>,

    /// Enable Telegram notifications
    pub telegram_enabled: bool,

    /// Minimum time between alerts (seconds) to avoid spam
    pub alert_cooldown_secs: u64,

    /// Enable daily performance reports
    pub daily_reports: bool,

    /// Time for daily report (UTC hour)
    pub daily_report_hour: u8,

    /// Alert on drawdown exceeding threshold
    pub max_drawdown_alert_pct: f64,

    /// Alert on consecutive losses
    pub max_consecutive_losses: usize,

    /// Alert on win rate dropping below threshold
    pub min_win_rate_alert_pct: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            telegram_token: None,
            telegram_chat_id: None,
            telegram_enabled: false,
            alert_cooldown_secs: 300, // 5 minutes
            daily_reports: true,
            daily_report_hour: 0, // Midnight UTC
            max_drawdown_alert_pct: 10.0,
            max_consecutive_losses: 5,
            min_win_rate_alert_pct: 45.0,
        }
    }
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

impl AlertLevel {
    pub fn emoji(&self) -> &'static str {
        match self {
            AlertLevel::Info => "ℹ️",
            AlertLevel::Warning => "⚠️",
            AlertLevel::Error => "❌",
            AlertLevel::Critical => "🚨",
        }
    }
}

/// Alert message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub level: AlertLevel,
    pub title: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

impl Alert {
    pub fn new(level: AlertLevel, title: String, message: String) -> Self {
        Self {
            level,
            title,
            message,
            timestamp: Utc::now(),
        }
    }

    pub fn format_telegram(&self) -> String {
        format!(
            "{} *{}*\n\n{}\n\n_{}",
            self.level.emoji(),
            self.title,
            self.message,
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
}

/// Real-time performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMetrics {
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate_pct: f64,
    pub total_pnl: f64,
    pub total_return_pct: f64,
    pub current_drawdown_pct: f64,
    pub max_drawdown_pct: f64,
    pub consecutive_wins: usize,
    pub consecutive_losses: usize,
    pub sharpe_ratio: f64,
    pub last_trade_time: Option<DateTime<Utc>>,
    pub last_updated: DateTime<Utc>,
}

impl Default for LiveMetrics {
    fn default() -> Self {
        Self {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate_pct: 0.0,
            total_pnl: 0.0,
            total_return_pct: 0.0,
            current_drawdown_pct: 0.0,
            max_drawdown_pct: 0.0,
            consecutive_wins: 0,
            consecutive_losses: 0,
            sharpe_ratio: 0.0,
            last_trade_time: None,
            last_updated: Utc::now(),
        }
    }
}

impl LiveMetrics {
    pub fn summary(&self) -> String {
        format!(
            "📊 *Performance Summary*\n\n\
            Total Trades: {}\n\
            Win Rate: {:.1}%\n\
            Total PnL: ${:.2}\n\
            Return: {:.2}%\n\
            Current DD: {:.2}%\n\
            Max DD: {:.2}%\n\
            Sharpe: {:.2}\n\
            Consecutive: {}W / {}L",
            self.total_trades,
            self.win_rate_pct,
            self.total_pnl,
            self.total_return_pct,
            self.current_drawdown_pct,
            self.max_drawdown_pct,
            self.sharpe_ratio,
            self.consecutive_wins,
            self.consecutive_losses
        )
    }
}

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: SystemStatus,
    pub uptime_hours: f64,
    pub data_connection: bool,
    pub broker_connection: bool,
    pub last_data_update: Option<DateTime<Utc>>,
    pub last_trade_attempt: Option<DateTime<Utc>>,
    pub errors_last_hour: usize,
    pub warnings_last_hour: usize,
    pub memory_usage_mb: f64,
    pub cpu_usage_pct: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemStatus {
    Healthy,
    Degraded,
    Critical,
    Offline,
}

impl SystemStatus {
    pub fn emoji(&self) -> &'static str {
        match self {
            SystemStatus::Healthy => "✅",
            SystemStatus::Degraded => "⚠️",
            SystemStatus::Critical => "❌",
            SystemStatus::Offline => "🔴",
        }
    }
}

/// Trade notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeNotification {
    pub signal: Signal,
    pub entry_price: f64,
    pub size: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

impl TradeNotification {
    pub fn format_telegram(&self) -> String {
        let signal_emoji = match self.signal {
            Signal::Buy => "🟢 LONG",
            Signal::Sell => "🔴 SHORT",
            Signal::Hold => "⏸️ HOLD",
        };

        let mut msg = format!(
            "{} *Trade Executed*\n\n\
            Signal: {}\n\
            Entry: ${:.2}\n\
            Size: {:.4} oz\n\
            Value: ${:.2}",
            signal_emoji,
            format!("{:?}", self.signal),
            self.entry_price,
            self.size,
            self.entry_price * self.size
        );

        if let Some(sl) = self.stop_loss {
            msg.push_str(&format!("\nStop Loss: ${:.2}", sl));
        }

        if let Some(tp) = self.take_profit {
            msg.push_str(&format!("\nTake Profit: ${:.2}", tp));
        }

        msg.push_str(&format!("\nReason: {}", self.reason));
        msg.push_str(&format!("\n\n_{}_", self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));

        msg
    }
}

/// Production monitoring system
pub struct MonitoringSystem {
    config: MonitoringConfig,
    metrics: Arc<RwLock<LiveMetrics>>,
    health: Arc<RwLock<HealthStatus>>,
    telegram_client: Option<TelegramNotifier>,
    alert_history: Arc<RwLock<Vec<Alert>>>,
    last_alert_time: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    start_time: DateTime<Utc>,
    initial_capital: f64,
}

impl MonitoringSystem {
    pub fn new(config: MonitoringConfig, initial_capital: f64) -> Self {
        let telegram_client = if config.telegram_enabled {
            config
                .telegram_token
                .as_ref()
                .and_then(|token| config.telegram_chat_id.as_ref().map(|chat_id| {
                    TelegramNotifier::new(token.clone(), chat_id.clone())
                }))
        } else {
            None
        };

        let health = HealthStatus {
            status: SystemStatus::Healthy,
            uptime_hours: 0.0,
            data_connection: true,
            broker_connection: true,
            last_data_update: None,
            last_trade_attempt: None,
            errors_last_hour: 0,
            warnings_last_hour: 0,
            memory_usage_mb: 0.0,
            cpu_usage_pct: 0.0,
        };

        Self {
            config,
            metrics: Arc::new(RwLock::new(LiveMetrics::default())),
            health: Arc::new(RwLock::new(health)),
            telegram_client,
            alert_history: Arc::new(RwLock::new(Vec::new())),
            last_alert_time: Arc::new(RwLock::new(HashMap::new())),
            start_time: Utc::now(),
            initial_capital,
        }
    }

    /// Record a completed trade
    pub async fn record_trade(&self, trade: &Trade) {
        let mut metrics = self.metrics.write().await;

        metrics.total_trades += 1;
        metrics.total_pnl += trade.pnl;
        metrics.total_return_pct = (metrics.total_pnl / self.initial_capital) * 100.0;
        metrics.last_trade_time = Some(trade.exit_time);

        if trade.pnl > 0.0 {
            metrics.winning_trades += 1;
            metrics.consecutive_wins += 1;
            metrics.consecutive_losses = 0;
        } else {
            metrics.losing_trades += 1;
            metrics.consecutive_losses += 1;
            metrics.consecutive_wins = 0;
        }

        metrics.win_rate_pct = if metrics.total_trades > 0 {
            (metrics.winning_trades as f64 / metrics.total_trades as f64) * 100.0
        } else {
            0.0
        };

        metrics.last_updated = Utc::now();

        drop(metrics);

        // Send trade notification
        if let Some(telegram) = &self.telegram_client {
            let notification = TradeNotification {
                signal: trade.signal,
                entry_price: trade.entry_price,
                size: trade.size,
                stop_loss: None,
                take_profit: None,
                reason: format!("PnL: ${:.2} ({:.2}%)", trade.pnl, trade.pnl_pct),
                timestamp: trade.exit_time,
            };

            if let Err(e) = telegram.send_trade_notification(&notification).await {
                error!("Failed to send trade notification: {}", e);
            }
        }

        // Check for alert conditions
        self.check_alert_conditions().await;
    }

    /// Record a trade entry
    pub async fn record_trade_entry(
        &self,
        signal: Signal,
        entry_price: f64,
        size: f64,
        stop_loss: Option<f64>,
        take_profit: Option<f64>,
        reason: String,
    ) {
        let mut health = self.health.write().await;
        health.last_trade_attempt = Some(Utc::now());
        drop(health);

        if let Some(telegram) = &self.telegram_client {
            let notification = TradeNotification {
                signal,
                entry_price,
                size,
                stop_loss,
                take_profit,
                reason,
                timestamp: Utc::now(),
            };

            if let Err(e) = telegram.send_trade_notification(&notification).await {
                error!("Failed to send trade entry notification: {}", e);
            }
        }
    }

    /// Send trade entry notification (convenience wrapper)
    pub async fn send_trade_entry(
        &self,
        signal: Signal,
        entry_price: f64,
        size: f64,
        stop_loss: f64,
        take_profit: f64,
    ) {
        self.record_trade_entry(
            signal,
            entry_price,
            size,
            Some(stop_loss),
            Some(take_profit),
            "Position opened".to_string(),
        )
        .await;
    }

    /// Update health status for data feed connection
    pub async fn update_health_data_feed(&self, connected: bool) {
        let mut health = self.health.write().await;
        health.data_connection = connected;
        if !connected {
            health.status = SystemStatus::Degraded;
        }
    }

    /// Update health status for broker connection
    pub async fn update_health_broker(&self, connected: bool) {
        let mut health = self.health.write().await;
        health.broker_connection = connected;
        if !connected {
            health.status = SystemStatus::Degraded;
        }
    }

    /// Send alert
    pub async fn send_alert(&self, alert: Alert) {
        // Check cooldown
        if !self.should_send_alert(&alert.title).await {
            return;
        }

        info!(
            "Alert [{}]: {} - {}",
            match alert.level {
                AlertLevel::Info => "INFO",
                AlertLevel::Warning => "WARN",
                AlertLevel::Error => "ERROR",
                AlertLevel::Critical => "CRITICAL",
            },
            alert.title,
            alert.message
        );

        // Store alert
        let mut history = self.alert_history.write().await;
        history.push(alert.clone());
        drop(history);

        // Update last alert time
        let mut last_times = self.last_alert_time.write().await;
        last_times.insert(alert.title.clone(), Utc::now());
        drop(last_times);

        // Send to Telegram
        if let Some(telegram) = &self.telegram_client {
            if let Err(e) = telegram.send_alert(&alert).await {
                error!("Failed to send alert to Telegram: {}", e);
            }
        }
    }

    /// Check if alert should be sent (cooldown logic)
    async fn should_send_alert(&self, title: &str) -> bool {
        let last_times = self.last_alert_time.read().await;

        if let Some(&last_time) = last_times.get(title) {
            let elapsed = Utc::now().signed_duration_since(last_time);
            elapsed.num_seconds() >= self.config.alert_cooldown_secs as i64
        } else {
            true
        }
    }

    /// Check for alert conditions
    async fn check_alert_conditions(&self) {
        let metrics = self.metrics.read().await;

        // Check drawdown
        if metrics.current_drawdown_pct > self.config.max_drawdown_alert_pct {
            let current_dd = metrics.current_drawdown_pct;
            let threshold = self.config.max_drawdown_alert_pct;
            drop(metrics);
            self.send_alert(Alert::new(
                AlertLevel::Warning,
                "High Drawdown".to_string(),
                format!(
                    "Current drawdown {:.2}% exceeds threshold {:.2}%",
                    current_dd, threshold
                ),
            ))
            .await;
            return;
        }

        // Check consecutive losses
        if metrics.consecutive_losses >= self.config.max_consecutive_losses {
            let consecutive = metrics.consecutive_losses;
            drop(metrics);
            self.send_alert(Alert::new(
                AlertLevel::Warning,
                "Consecutive Losses".to_string(),
                format!(
                    "{} consecutive losing trades",
                    consecutive
                ),
            ))
            .await;
            return;
        }

        // Check win rate
        if metrics.total_trades >= 20
            && metrics.win_rate_pct < self.config.min_win_rate_alert_pct
        {
            let win_rate = metrics.win_rate_pct;
            let threshold = self.config.min_win_rate_alert_pct;
            drop(metrics);
            self.send_alert(Alert::new(
                AlertLevel::Warning,
                "Low Win Rate".to_string(),
                format!(
                    "Win rate {:.1}% below threshold {:.1}%",
                    win_rate, threshold
                ),
            ))
            .await;
        }
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> LiveMetrics {
        self.metrics.read().await.clone()
    }

    /// Get health status
    pub async fn get_health(&self) -> HealthStatus {
        let mut health = self.health.write().await;

        // Update uptime
        let uptime = Utc::now().signed_duration_since(self.start_time);
        health.uptime_hours = uptime.num_seconds() as f64 / 3600.0;

        health.clone()
    }

    /// Update health status
    pub async fn update_health(&self, updates: impl FnOnce(&mut HealthStatus)) {
        let mut health = self.health.write().await;
        updates(&mut health);
    }

    /// Send daily performance report
    pub async fn send_daily_report(&self) {
        if !self.config.daily_reports {
            return;
        }

        let metrics = self.metrics.read().await;
        let report = metrics.summary();
        drop(metrics);

        let alert = Alert::new(
            AlertLevel::Info,
            "Daily Performance Report".to_string(),
            report,
        );

        if let Some(telegram) = &self.telegram_client {
            if let Err(e) = telegram.send_alert(&alert).await {
                error!("Failed to send daily report: {}", e);
            }
        }
    }

    /// Send system startup notification
    pub async fn send_startup_notification(&self) {
        let alert = Alert::new(
            AlertLevel::Info,
            "System Started".to_string(),
            format!(
                "🚀 Autonomous trading system is now online\n\
                Initial Capital: ${:.2}\n\
                Time: {}",
                self.initial_capital,
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            ),
        );

        self.send_alert(alert).await;
    }

    /// Send system shutdown notification
    pub async fn send_shutdown_notification(&self) {
        let metrics = self.metrics.read().await;
        let alert = Alert::new(
            AlertLevel::Info,
            "System Shutdown".to_string(),
            format!(
                "🛑 Autonomous trading system is shutting down\n\n{}",
                metrics.summary()
            ),
        );
        drop(metrics);

        self.send_alert(alert).await;
    }
}

/// Telegram notification client
struct TelegramNotifier {
    bot_token: String,
    chat_id: String,
    client: Client,
}

impl TelegramNotifier {
    fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            bot_token,
            chat_id,
            client: Client::new(),
        }
    }

    async fn send_message(&self, text: &str) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let params = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "Markdown",
        });

        let response = self
            .client
            .post(&url)
            .json(&params)
            .send()
            .await
            .map_err(|e| Error::Http(e))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Execution(format!(
                "Telegram API error: {}",
                error_text
            )));
        }

        Ok(())
    }

    async fn send_alert(&self, alert: &Alert) -> Result<()> {
        self.send_message(&alert.format_telegram()).await
    }

    async fn send_trade_notification(&self, notification: &TradeNotification) -> Result<()> {
        self.send_message(&notification.format_telegram()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_formatting() {
        let alert = Alert::new(
            AlertLevel::Warning,
            "Test Alert".to_string(),
            "This is a test message".to_string(),
        );

        let formatted = alert.format_telegram();
        assert!(formatted.contains("⚠️"));
        assert!(formatted.contains("Test Alert"));
        assert!(formatted.contains("This is a test message"));
    }

    #[tokio::test]
    async fn test_metrics_update() {
        let config = MonitoringConfig::default();
        let monitoring = MonitoringSystem::new(config, 100000.0);

        let trade = Trade {
            signal: Signal::Buy,
            entry_price: 1850.0,
            entry_time: Utc::now(),
            exit_price: 1860.0,
            exit_time: Utc::now(),
            size: 1.0,
            pnl: 10.0,
            pnl_pct: 0.54,
            duration_hours: 2.5,
            commission: 0.1,
            slippage: 0.05,
            strategy: "test".to_string(),
        };

        monitoring.record_trade(&trade).await;

        let metrics = monitoring.get_metrics().await;
        assert_eq!(metrics.total_trades, 1);
        assert_eq!(metrics.winning_trades, 1);
        assert_eq!(metrics.consecutive_wins, 1);
    }
}
