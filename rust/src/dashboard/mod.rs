//! Real-time Web Dashboard Module
//!
//! Provides a comprehensive web interface for monitoring trading performance,
//! active positions, strategy comparison, and system health.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

use crate::{
    analytics::PerformanceMetrics,
    config::Config,
    types::{Position, Trade},
    Result,
};

mod handlers;
mod state;
mod websocket;

pub use state::DashboardState;

/// Real-time update message sent via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DashboardUpdate {
    /// New trade executed
    TradeExecuted {
        trade: Trade,
        total_pnl: f64,
        equity: f64,
    },
    /// Position opened/updated
    PositionUpdate {
        position: Position,
        unrealized_pnl: f64,
    },
    /// Position closed
    PositionClosed {
        position_id: String,
        pnl: f64,
    },
    /// Performance metrics updated
    MetricsUpdate {
        metrics: PerformanceSnapshot,
    },
    /// News sentiment update
    NewsUpdate {
        headline: String,
        sentiment: f64,
        timestamp: String,
    },
    /// System health update
    HealthUpdate {
        status: String,
        uptime_seconds: u64,
        errors_count: usize,
    },
}

/// Performance snapshot for dashboard display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub total_trades: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub current_equity: f64,
    pub daily_pnl: f64,
    pub weekly_pnl: f64,
    pub monthly_pnl: f64,
}

/// Strategy comparison data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyComparison {
    pub name: String,
    pub total_trades: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub profit_factor: f64,
}

/// Active position display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePositionDisplay {
    pub symbol: String,
    pub signal: String,
    pub entry_price: f64,
    pub current_price: f64,
    pub size: f64,
    pub unrealized_pnl: f64,
    pub unrealized_pnl_pct: f64,
    pub duration_hours: f64,
}

/// Dashboard configuration
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    pub host: String,
    pub port: u16,
    pub update_interval_ms: u64,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            update_interval_ms: 1000, // 1 second
        }
    }
}

/// Start the dashboard server
pub async fn start_dashboard(
    config: Config,
    dashboard_config: DashboardConfig,
) -> Result<()> {
    info!(
        "Starting dashboard server on {}:{}",
        dashboard_config.host, dashboard_config.port
    );

    let (tx, _rx) = broadcast::channel(100);
    let state = Arc::new(DashboardState::new(config, tx.clone()));

    let app = Router::new()
        .route("/", get(handlers::index))
        .route("/api/overview", get(handlers::get_overview))
        .route("/api/trades", get(handlers::get_trades))
        .route("/api/positions", get(handlers::get_active_positions))
        .route("/api/strategies", get(handlers::get_strategy_comparison))
        .route("/api/equity_curve", get(handlers::get_equity_curve))
        .route("/api/risk_metrics", get(handlers::get_risk_metrics))
        .route("/api/news", get(handlers::get_recent_news))
        .route("/api/health", get(handlers::get_system_health))
        .route("/ws", get(websocket_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", dashboard_config.host, dashboard_config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Dashboard running at http://{}", addr);
    info!("WebSocket available at ws://{}/ws", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// WebSocket handler for real-time updates
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<DashboardState>>,
) -> Response {
    ws.on_upgrade(|socket| websocket::handle_socket(socket, state))
}

/// Broadcast a dashboard update to all connected clients
pub async fn broadcast_update(
    tx: &broadcast::Sender<DashboardUpdate>,
    update: DashboardUpdate,
) {
    if let Err(e) = tx.send(update) {
        warn!("Failed to broadcast update: {}", e);
    }
}
