//! HTTP handlers for dashboard API endpoints

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{
    state::DashboardState, ActivePositionDisplay, PerformanceSnapshot, StrategyComparison,
};
use crate::types::{Signal, Trade};

/// Dashboard overview response
#[derive(Debug, Serialize)]
pub struct OverviewResponse {
    pub current_equity: f64,
    pub total_pnl: f64,
    pub daily_pnl: f64,
    pub weekly_pnl: f64,
    pub monthly_pnl: f64,
    pub active_positions_count: usize,
    pub total_trades: usize,
    pub uptime_seconds: u64,
}

/// Equity curve data point
#[derive(Debug, Serialize)]
pub struct EquityPoint {
    pub timestamp: String,
    pub equity: f64,
    pub drawdown_pct: f64,
}

/// Risk metrics response
#[derive(Debug, Serialize)]
pub struct RiskMetricsResponse {
    pub total_exposure: f64,
    pub largest_position_size: f64,
    pub current_drawdown_pct: f64,
    pub max_drawdown_pct: f64,
    pub risk_limit_used_pct: f64,
    pub position_count: usize,
}

/// System health response
#[derive(Debug, Serialize)]
pub struct SystemHealthResponse {
    pub status: String,
    pub uptime_seconds: u64,
    pub uptime_human: String,
    pub total_errors: usize,
    pub last_update: Option<String>,
    pub memory_usage_mb: f64,
}

/// Serve the main dashboard HTML page
pub async fn index() -> Html<String> {
    Html(include_str!("../../static/dashboard.html").to_string())
}

/// Get dashboard overview
pub async fn get_overview(
    State(state): State<Arc<DashboardState>>,
) -> Json<OverviewResponse> {
    let data = state.data.read().unwrap();

    Json(OverviewResponse {
        current_equity: data.current_equity,
        total_pnl: data.total_pnl,
        daily_pnl: data.daily_pnl,
        weekly_pnl: data.weekly_pnl,
        monthly_pnl: data.monthly_pnl,
        active_positions_count: data.active_positions.len(),
        total_trades: data.trades.len(),
        uptime_seconds: state.uptime_seconds(),
    })
}

/// Get recent trades
pub async fn get_trades(State(state): State<Arc<DashboardState>>) -> Json<Vec<Trade>> {
    let data = state.data.read().unwrap();

    // Return last 100 trades
    let trades: Vec<Trade> = data
        .trades
        .iter()
        .rev()
        .take(100)
        .cloned()
        .collect();

    Json(trades)
}

/// Get active positions
pub async fn get_active_positions(
    State(state): State<Arc<DashboardState>>,
) -> Json<Vec<ActivePositionDisplay>> {
    let data = state.data.read().unwrap();

    let positions: Vec<ActivePositionDisplay> = data
        .active_positions
        .iter()
        .map(|p| {
            let duration = chrono::Utc::now()
                .signed_duration_since(p.entry_time)
                .num_seconds() as f64
                / 3600.0;

            // For display, we'll assume current price = entry price (would be updated with live data)
            let current_price = p.entry_price;
            let unrealized_pnl = match p.signal {
                Signal::Buy => (current_price - p.entry_price) * p.size,
                Signal::Sell => (p.entry_price - current_price) * p.size,
                Signal::Hold => 0.0,
            };
            let unrealized_pnl_pct = (unrealized_pnl / (p.entry_price * p.size)) * 100.0;

            ActivePositionDisplay {
                symbol: "XAUUSD".to_string(),
                signal: format!("{:?}", p.signal),
                entry_price: p.entry_price,
                current_price,
                size: p.size,
                unrealized_pnl,
                unrealized_pnl_pct,
                duration_hours: duration,
            }
        })
        .collect();

    Json(positions)
}

/// Get strategy comparison data
pub async fn get_strategy_comparison(
    State(state): State<Arc<DashboardState>>,
) -> Json<Vec<StrategyComparison>> {
    let data = state.data.read().unwrap();
    Json(data.strategy_stats.clone())
}

/// Get equity curve data
pub async fn get_equity_curve(
    State(state): State<Arc<DashboardState>>,
) -> Json<Vec<EquityPoint>> {
    let data = state.data.read().unwrap();
    let initial_equity = state.config.trading.initial_capital;

    let mut equity = initial_equity;
    let mut peak_equity = initial_equity;
    let mut points = vec![EquityPoint {
        timestamp: chrono::Utc::now().to_rfc3339(),
        equity: initial_equity,
        drawdown_pct: 0.0,
    }];

    for trade in data.trades.iter() {
        equity += trade.pnl;

        if equity > peak_equity {
            peak_equity = equity;
        }

        let drawdown_pct = if peak_equity > 0.0 {
            ((peak_equity - equity) / peak_equity) * 100.0
        } else {
            0.0
        };

        points.push(EquityPoint {
            timestamp: trade.exit_time.to_rfc3339(),
            equity,
            drawdown_pct,
        });
    }

    Json(points)
}

/// Get risk metrics
pub async fn get_risk_metrics(
    State(state): State<Arc<DashboardState>>,
) -> Json<RiskMetricsResponse> {
    let data = state.data.read().unwrap();

    let total_exposure: f64 = data
        .active_positions
        .iter()
        .map(|p| p.entry_price * p.size)
        .sum();

    let largest_position = data
        .active_positions
        .iter()
        .map(|p| p.size)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    let initial_capital = state.config.trading.initial_capital;
    let current_drawdown_pct = if initial_capital > 0.0 {
        ((initial_capital - data.current_equity) / initial_capital) * 100.0
    } else {
        0.0
    };

    // Calculate risk limit used as a percentage of equity (using 50% max exposure as baseline)
    let max_exposure = data.current_equity * 0.5; // 50% of equity max
    let risk_limit_used_pct = if max_exposure > 0.0 {
        (total_exposure / max_exposure) * 100.0
    } else {
        0.0
    };

    Json(RiskMetricsResponse {
        total_exposure,
        largest_position_size: largest_position,
        current_drawdown_pct: current_drawdown_pct.max(0.0),
        max_drawdown_pct: state.config.risk.max_drawdown_pct,
        risk_limit_used_pct: risk_limit_used_pct.min(100.0),
        position_count: data.active_positions.len(),
    })
}

/// Get recent news
pub async fn get_recent_news(
    State(state): State<Arc<DashboardState>>,
) -> Json<Vec<super::state::NewsItem>> {
    let data = state.data.read().unwrap();
    Json(data.recent_news.clone())
}

/// Get system health
pub async fn get_system_health(
    State(state): State<Arc<DashboardState>>,
) -> Json<SystemHealthResponse> {
    let data = state.data.read().unwrap();
    let uptime_secs = state.uptime_seconds();

    let hours = uptime_secs / 3600;
    let minutes = (uptime_secs % 3600) / 60;
    let seconds = uptime_secs % 60;
    let uptime_human = format!("{}h {}m {}s", hours, minutes, seconds);

    let status = if data.errors_count > 10 {
        "Warning"
    } else if data.errors_count > 0 {
        "OK"
    } else {
        "Healthy"
    };

    let last_update = data
        .last_update
        .map(|t| {
            chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()
        });

    // Simple memory estimation (would use a proper crate in production)
    let memory_usage_mb = 0.0; // Placeholder

    Json(SystemHealthResponse {
        status: status.to_string(),
        uptime_seconds: uptime_secs,
        uptime_human,
        total_errors: data.errors_count,
        last_update,
        memory_usage_mb,
    })
}
