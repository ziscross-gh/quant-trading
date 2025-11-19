//! Database layer for persistent storage
//!
//! PostgreSQL integration for storing:
//! - Trade history
//! - Performance metrics
//! - Configuration versions
//! - Audit logs
//!
//! ## Setup
//!
//! 1. Install PostgreSQL
//! 2. Create database: `createdb trading_db`
//! 3. Set DATABASE_URL: `postgresql://user:password@localhost/trading_db`
//! 4. Run migrations: `sqlx migrate run`
//!
//! ## Migrations
//!
//! See `rust/migrations/` directory for SQL schemas.

use crate::{Result, Trade, EquityPoint};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;

/// Database connection pool
pub struct Database {
    pool: Arc<PgPool>,
}

impl Database {
    /// Create new database connection
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| crate::Error::Config(format!("Database connection failed: {}", e)))?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Save trade to database
    pub async fn save_trade(&self, trade: &Trade) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO trades (
                entry_time, exit_time, signal, entry_price, exit_price,
                size, pnl, pnl_pct, duration_hours, strategy
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            trade.entry_time,
            trade.exit_time,
            format!("{:?}", trade.signal),
            trade.entry_price,
            trade.exit_price,
            trade.size,
            trade.pnl,
            trade.pnl_pct,
            trade.duration_hours,
            trade.strategy.as_deref()
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| crate::Error::Execution(format!("Failed to save trade: {}", e)))?;

        Ok(())
    }

    /// Get trade history
    pub async fn get_trades(&self, limit: i64) -> Result<Vec<Trade>> {
        let records = sqlx::query!(
            r#"
            SELECT
                entry_time, exit_time, signal, entry_price, exit_price,
                size, pnl, pnl_pct, duration_hours, strategy
            FROM trades
            ORDER BY entry_time DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| crate::Error::Execution(format!("Failed to fetch trades: {}", e)))?;

        let trades = records
            .into_iter()
            .map(|record| {
                let signal = match record.signal.as_str() {
                    "Buy" => crate::Signal::Buy,
                    "Sell" => crate::Signal::Sell,
                    _ => crate::Signal::Hold,
                };

                Trade {
                    entry_time: record.entry_time,
                    exit_time: record.exit_time,
                    signal,
                    entry_price: record.entry_price.to_string().parse().unwrap_or(0.0),
                    exit_price: record.exit_price.to_string().parse().unwrap_or(0.0),
                    size: record.size.to_string().parse().unwrap_or(0.0),
                    pnl: record.pnl.to_string().parse().unwrap_or(0.0),
                    pnl_pct: record.pnl_pct.to_string().parse().unwrap_or(0.0),
                    duration_hours: record.duration_hours.to_string().parse().unwrap_or(0.0),
                    strategy: record.strategy,
                }
            })
            .collect();

        Ok(trades)
    }

    /// Save equity point
    pub async fn save_equity_point(&self, point: &EquityPoint) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO equity_curve (timestamp, equity, price)
            VALUES ($1, $2, $3)
            "#,
            point.timestamp,
            point.equity,
            point.price
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| crate::Error::Execution(format!("Failed to save equity point: {}", e)))?;

        Ok(())
    }

    /// Get equity curve
    pub async fn get_equity_curve(&self, limit: i64) -> Result<Vec<EquityPoint>> {
        let records = sqlx::query!(
            r#"
            SELECT timestamp, equity, price
            FROM equity_curve
            ORDER BY timestamp DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| crate::Error::Execution(format!("Failed to fetch equity curve: {}", e)))?;

        let points = records
            .into_iter()
            .map(|record| EquityPoint {
                timestamp: record.timestamp,
                equity: record.equity.to_string().parse().unwrap_or(0.0),
                price: record.price.to_string().parse().unwrap_or(0.0),
            })
            .collect();

        Ok(points)
    }

    /// Get performance summary
    pub async fn get_summary(&self) -> Result<PerformanceSummary> {
        let record = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_trades,
                SUM(CASE WHEN pnl > 0 THEN 1 ELSE 0 END) as winning_trades,
                SUM(pnl) as total_pnl,
                AVG(pnl) as avg_pnl,
                MAX(pnl) as max_profit,
                MIN(pnl) as max_loss
            FROM trades
            "#
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| crate::Error::Execution(format!("Failed to fetch summary: {}", e)))?;

        let total_trades = record.total_trades.unwrap_or(0);
        let winning_trades = record.winning_trades.unwrap_or(0);
        let win_rate = if total_trades > 0 {
            (winning_trades as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };

        Ok(PerformanceSummary {
            total_trades: total_trades as usize,
            winning_trades: winning_trades as usize,
            losing_trades: (total_trades - winning_trades) as usize,
            win_rate,
            total_pnl: record
                .total_pnl
                .map(|d| d.to_string().parse().unwrap_or(0.0))
                .unwrap_or(0.0),
            avg_pnl: record
                .avg_pnl
                .map(|d| d.to_string().parse().unwrap_or(0.0))
                .unwrap_or(0.0),
            max_profit: record
                .max_profit
                .map(|d| d.to_string().parse().unwrap_or(0.0))
                .unwrap_or(0.0),
            max_loss: record
                .max_loss
                .map(|d| d.to_string().parse().unwrap_or(0.0))
                .unwrap_or(0.0),
        })
    }
}

/// Performance summary statistics
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub avg_pnl: f64,
    pub max_profit: f64,
    pub max_loss: f64,
}
}

// SQL MIGRATION TEMPLATES (create in rust/migrations/):
//
// 001_create_trades.sql:
// ```sql
// CREATE TABLE trades (
//     id SERIAL PRIMARY KEY,
//     entry_time TIMESTAMPTZ NOT NULL,
//     exit_time TIMESTAMPTZ NOT NULL,
//     signal VARCHAR(10) NOT NULL,
//     entry_price DECIMAL(10, 2) NOT NULL,
//     exit_price DECIMAL(10, 2) NOT NULL,
//     size DECIMAL(10, 4) NOT NULL,
//     pnl DECIMAL(10, 2) NOT NULL,
//     pnl_pct DECIMAL(6, 2) NOT NULL,
//     duration_hours DECIMAL(10, 2) NOT NULL,
//     strategy VARCHAR(50),
//     created_at TIMESTAMPTZ DEFAULT NOW()
// );
//
// CREATE INDEX idx_trades_entry_time ON trades(entry_time);
// CREATE INDEX idx_trades_strategy ON trades(strategy);
// ```
//
// 002_create_equity_curve.sql:
// ```sql
// CREATE TABLE equity_curve (
//     id SERIAL PRIMARY KEY,
//     timestamp TIMESTAMPTZ NOT NULL,
//     equity DECIMAL(12, 2) NOT NULL,
//     price DECIMAL(10, 2) NOT NULL,
//     created_at TIMESTAMPTZ DEFAULT NOW()
// );
//
// CREATE INDEX idx_equity_timestamp ON equity_curve(timestamp);
// ```
