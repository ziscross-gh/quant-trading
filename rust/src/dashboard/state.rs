//! Dashboard state management

use std::sync::RwLock;
use std::time::Instant;
use tokio::sync::broadcast;

use crate::{
    config::Config,
    types::{Position, Trade},
};

use serde::{Deserialize, Serialize};

use super::{DashboardUpdate, PerformanceSnapshot, StrategyComparison};

/// Shared dashboard state
pub struct DashboardState {
    pub config: Config,
    pub tx: broadcast::Sender<DashboardUpdate>,
    pub data: RwLock<DashboardData>,
    pub start_time: Instant,
}

/// Mutable dashboard data
#[derive(Debug, Default)]
pub struct DashboardData {
    pub trades: Vec<Trade>,
    pub active_positions: Vec<Position>,
    pub current_equity: f64,
    pub total_pnl: f64,
    pub daily_pnl: f64,
    pub weekly_pnl: f64,
    pub monthly_pnl: f64,
    pub strategy_stats: Vec<StrategyComparison>,
    pub recent_news: Vec<NewsItem>,
    pub errors_count: usize,
    pub last_update: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub headline: String,
    pub sentiment: f64,
    pub timestamp: String,
    pub source: String,
}

impl DashboardState {
    pub fn new(config: Config, tx: broadcast::Sender<DashboardUpdate>) -> Self {
        let initial_equity = config.trading.initial_capital;

        Self {
            config,
            tx,
            data: RwLock::new(DashboardData {
                current_equity: initial_equity,
                ..Default::default()
            }),
            start_time: Instant::now(),
        }
    }

    /// Add a new trade to history
    pub fn add_trade(&self, trade: Trade) {
        let mut data = self.data.write().unwrap();
        data.total_pnl += trade.pnl;
        data.current_equity += trade.pnl;
        data.trades.push(trade.clone());
        data.last_update = Some(std::time::SystemTime::now());

        // Broadcast update
        let _ = self.tx.send(DashboardUpdate::TradeExecuted {
            trade,
            total_pnl: data.total_pnl,
            equity: data.current_equity,
        });
    }

    /// Update active position
    pub fn update_position(&self, position: Position, unrealized_pnl: f64) {
        let mut data = self.data.write().unwrap();

        // Update or add position
        if let Some(pos) = data.active_positions.iter_mut().find(|p| p.entry_time == position.entry_time) {
            *pos = position.clone();
        } else {
            data.active_positions.push(position.clone());
        }

        data.last_update = Some(std::time::SystemTime::now());

        // Broadcast update
        let _ = self.tx.send(DashboardUpdate::PositionUpdate {
            position,
            unrealized_pnl,
        });
    }

    /// Close a position
    pub fn close_position(&self, entry_time: chrono::DateTime<chrono::Utc>, pnl: f64) {
        let mut data = self.data.write().unwrap();
        data.active_positions.retain(|p| p.entry_time != entry_time);
        data.last_update = Some(std::time::SystemTime::now());

        // Broadcast update
        let _ = self.tx.send(DashboardUpdate::PositionClosed {
            position_id: entry_time.to_string(),
            pnl,
        });
    }

    /// Update performance metrics
    pub fn update_metrics(&self, snapshot: PerformanceSnapshot) {
        let mut data = self.data.write().unwrap();
        data.current_equity = snapshot.current_equity;
        data.total_pnl = snapshot.total_pnl;
        data.daily_pnl = snapshot.daily_pnl;
        data.weekly_pnl = snapshot.weekly_pnl;
        data.monthly_pnl = snapshot.monthly_pnl;
        data.last_update = Some(std::time::SystemTime::now());

        // Broadcast update
        let _ = self.tx.send(DashboardUpdate::MetricsUpdate {
            metrics: snapshot,
        });
    }

    /// Add news item
    pub fn add_news(&self, headline: String, sentiment: f64, source: String) {
        let mut data = self.data.write().unwrap();
        let timestamp = chrono::Utc::now().to_rfc3339();

        data.recent_news.insert(0, NewsItem {
            headline: headline.clone(),
            sentiment,
            timestamp: timestamp.clone(),
            source,
        });

        // Keep only last 50 news items
        if data.recent_news.len() > 50 {
            data.recent_news.truncate(50);
        }

        // Broadcast update
        let _ = self.tx.send(DashboardUpdate::NewsUpdate {
            headline,
            sentiment,
            timestamp,
        });
    }

    /// Increment error counter
    pub fn increment_errors(&self) {
        let mut data = self.data.write().unwrap();
        data.errors_count += 1;
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Update strategy statistics
    pub fn update_strategy_stats(&self, stats: Vec<StrategyComparison>) {
        let mut data = self.data.write().unwrap();
        data.strategy_stats = stats;
    }
}
