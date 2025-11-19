//! Performance Analytics Module
//!
//! Advanced trading performance metrics and analysis.

use crate::{EquityPoint, Result, Trade};
use serde::{Deserialize, Serialize};

/// Comprehensive performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    // Basic metrics
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: f64,

    // P&L metrics
    pub total_pnl: f64,
    pub total_pnl_pct: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub largest_win: f64,
    pub largest_loss: f64,

    // Risk metrics
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub max_drawdown: f64,
    pub max_drawdown_pct: f64,
    pub calmar_ratio: f64,

    // Trade quality
    pub profit_factor: f64,
    pub avg_trade_duration_hours: f64,
    pub avg_bars_in_trade: f64,

    // Consecutive stats
    pub max_consecutive_wins: usize,
    pub max_consecutive_losses: usize,
    pub current_streak: i32, // Positive for wins, negative for losses

    // Expectancy
    pub expectancy: f64,
    pub expectancy_pct: f64,

    // Recovery metrics
    pub recovery_factor: f64,
}

impl PerformanceMetrics {
    /// Calculate metrics from trade history
    pub fn from_trades(trades: &[Trade], initial_capital: f64) -> Self {
        if trades.is_empty() {
            return Self::default();
        }

        let total_trades = trades.len();
        let winning_trades = trades.iter().filter(|t| t.pnl > 0.0).count();
        let losing_trades = trades.iter().filter(|t| t.pnl < 0.0).count();
        let win_rate = if total_trades > 0 {
            (winning_trades as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };

        // P&L metrics
        let total_pnl: f64 = trades.iter().map(|t| t.pnl).sum();
        let total_pnl_pct = (total_pnl / initial_capital) * 100.0;

        let winning_pnl: f64 = trades.iter().filter(|t| t.pnl > 0.0).map(|t| t.pnl).sum();
        let losing_pnl: f64 = trades.iter().filter(|t| t.pnl < 0.0).map(|t| t.pnl).sum();

        let avg_win = if winning_trades > 0 {
            winning_pnl / winning_trades as f64
        } else {
            0.0
        };

        let avg_loss = if losing_trades > 0 {
            losing_pnl / losing_trades as f64
        } else {
            0.0
        };

        let largest_win = trades.iter().map(|t| t.pnl).fold(0.0, f64::max);
        let largest_loss = trades.iter().map(|t| t.pnl).fold(0.0, f64::min);

        // Risk metrics
        let returns: Vec<f64> = trades.iter().map(|t| t.pnl_pct).collect();
        let sharpe_ratio = Self::calculate_sharpe_ratio(&returns);
        let sortino_ratio = Self::calculate_sortino_ratio(&returns);

        let (max_drawdown, max_drawdown_pct) = Self::calculate_max_drawdown(trades, initial_capital);

        let calmar_ratio = if max_drawdown_pct.abs() > 0.01 {
            (total_pnl_pct / 365.0) / max_drawdown_pct.abs() // Annualized return / max DD
        } else {
            0.0
        };

        // Profit factor
        let profit_factor = if losing_pnl.abs() > 0.01 {
            winning_pnl / losing_pnl.abs()
        } else {
            winning_pnl
        };

        // Trade duration
        let avg_trade_duration_hours: f64 = trades.iter().map(|t| t.duration_hours).sum::<f64>()
            / total_trades as f64;

        // Consecutive wins/losses
        let (max_consecutive_wins, max_consecutive_losses, current_streak) =
            Self::calculate_consecutive_stats(trades);

        // Expectancy
        let expectancy = if total_trades > 0 {
            (win_rate / 100.0 * avg_win) + ((1.0 - win_rate / 100.0) * avg_loss)
        } else {
            0.0
        };

        let expectancy_pct = if initial_capital > 0.0 {
            (expectancy / initial_capital) * 100.0
        } else {
            0.0
        };

        // Recovery factor
        let recovery_factor = if max_drawdown.abs() > 0.01 {
            total_pnl / max_drawdown.abs()
        } else {
            total_pnl
        };

        Self {
            total_trades,
            winning_trades,
            losing_trades,
            win_rate,
            total_pnl,
            total_pnl_pct,
            avg_win,
            avg_loss,
            largest_win,
            largest_loss,
            sharpe_ratio,
            sortino_ratio,
            max_drawdown,
            max_drawdown_pct,
            calmar_ratio,
            profit_factor,
            avg_trade_duration_hours,
            avg_bars_in_trade: 0.0, // Would need candle data
            max_consecutive_wins,
            max_consecutive_losses,
            current_streak,
            expectancy,
            expectancy_pct,
            recovery_factor,
        }
    }

    /// Calculate Sharpe Ratio (risk-adjusted return)
    fn calculate_sharpe_ratio(returns: &[f64]) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }

        let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance: f64 = returns
            .iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>()
            / returns.len() as f64;

        let std_dev = variance.sqrt();

        if std_dev > 0.001 {
            // Annualized Sharpe (assuming ~250 trading days/year)
            (mean_return / std_dev) * (250.0_f64).sqrt()
        } else {
            0.0
        }
    }

    /// Calculate Sortino Ratio (downside risk-adjusted return)
    fn calculate_sortino_ratio(returns: &[f64]) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }

        let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;

        // Only use negative returns for downside deviation
        let downside_returns: Vec<f64> = returns.iter()
            .filter(|&&r| r < 0.0)
            .copied()
            .collect();

        if downside_returns.is_empty() {
            return mean_return; // No downside, return is the ratio
        }

        let downside_variance: f64 = downside_returns
            .iter()
            .map(|r| r.powi(2))
            .sum::<f64>()
            / downside_returns.len() as f64;

        let downside_deviation = downside_variance.sqrt();

        if downside_deviation > 0.001 {
            // Annualized Sortino
            (mean_return / downside_deviation) * (250.0_f64).sqrt()
        } else {
            0.0
        }
    }

    /// Calculate maximum drawdown
    fn calculate_max_drawdown(trades: &[Trade], initial_capital: f64) -> (f64, f64) {
        if trades.is_empty() {
            return (0.0, 0.0);
        }

        let mut equity = initial_capital;
        let mut peak = equity;
        let mut max_dd = 0.0;
        let mut max_dd_pct = 0.0;

        for trade in trades {
            equity += trade.pnl;

            if equity > peak {
                peak = equity;
            }

            let drawdown = peak - equity;
            let drawdown_pct = if peak > 0.0 {
                (drawdown / peak) * 100.0
            } else {
                0.0
            };

            if drawdown > max_dd {
                max_dd = drawdown;
            }

            if drawdown_pct > max_dd_pct {
                max_dd_pct = drawdown_pct;
            }
        }

        (max_dd, max_dd_pct)
    }

    /// Calculate consecutive win/loss streaks
    fn calculate_consecutive_stats(trades: &[Trade]) -> (usize, usize, i32) {
        if trades.is_empty() {
            return (0, 0, 0);
        }

        let mut max_wins = 0;
        let mut max_losses = 0;
        let mut current_wins = 0;
        let mut current_losses = 0;

        for trade in trades {
            if trade.pnl > 0.0 {
                current_wins += 1;
                current_losses = 0;
                max_wins = max_wins.max(current_wins);
            } else if trade.pnl < 0.0 {
                current_losses += 1;
                current_wins = 0;
                max_losses = max_losses.max(current_losses);
            }
        }

        let current_streak = if current_wins > 0 {
            current_wins as i32
        } else {
            -(current_losses as i32)
        };

        (max_wins, max_losses, current_streak)
    }

    /// Generate performance report
    pub fn report(&self) -> String {
        format!(
            r#"
═══════════════════════════════════════════════════════════
                  PERFORMANCE METRICS REPORT
═══════════════════════════════════════════════════════════

📊 BASIC STATISTICS
  Total Trades:        {}
  Winning Trades:      {} ({:.1}%)
  Losing Trades:       {} ({:.1}%)
  Win Rate:            {:.1}%

💰 PROFIT & LOSS
  Total P&L:           ${:.2} ({:.2}%)
  Average Win:         ${:.2}
  Average Loss:        ${:.2}
  Largest Win:         ${:.2}
  Largest Loss:        ${:.2}
  Profit Factor:       {:.2}x

📈 RISK METRICS
  Sharpe Ratio:        {:.2}
  Sortino Ratio:       {:.2}
  Max Drawdown:        ${:.2} ({:.2}%)
  Calmar Ratio:        {:.2}
  Recovery Factor:     {:.2}

🎯 TRADE QUALITY
  Expectancy:          ${:.2} ({:.2}% per trade)
  Avg Duration:        {:.1} hours
  Max Consecutive Wins: {}
  Max Consecutive Loss: {}
  Current Streak:      {}

═══════════════════════════════════════════════════════════
"#,
            self.total_trades,
            self.winning_trades,
            self.win_rate,
            self.losing_trades,
            100.0 - self.win_rate,
            self.win_rate,
            self.total_pnl,
            self.total_pnl_pct,
            self.avg_win,
            self.avg_loss,
            self.largest_win,
            self.largest_loss,
            self.profit_factor,
            self.sharpe_ratio,
            self.sortino_ratio,
            self.max_drawdown,
            self.max_drawdown_pct,
            self.calmar_ratio,
            self.recovery_factor,
            self.expectancy,
            self.expectancy_pct,
            self.avg_trade_duration_hours,
            self.max_consecutive_wins,
            self.max_consecutive_losses,
            if self.current_streak >= 0 {
                format!("+{} wins", self.current_streak)
            } else {
                format!("{} losses", self.current_streak.abs())
            }
        )
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: 0.0,
            total_pnl: 0.0,
            total_pnl_pct: 0.0,
            avg_win: 0.0,
            avg_loss: 0.0,
            largest_win: 0.0,
            largest_loss: 0.0,
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
            max_drawdown: 0.0,
            max_drawdown_pct: 0.0,
            calmar_ratio: 0.0,
            profit_factor: 0.0,
            avg_trade_duration_hours: 0.0,
            avg_bars_in_trade: 0.0,
            max_consecutive_wins: 0,
            max_consecutive_losses: 0,
            current_streak: 0,
            expectancy: 0.0,
            expectancy_pct: 0.0,
            recovery_factor: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_performance_metrics() {
        let trades = vec![
            Trade {
                entry_time: Utc::now(),
                exit_time: Utc::now(),
                signal: crate::Signal::Buy,
                entry_price: 2000.0,
                exit_price: 2100.0,
                size: 1.0,
                pnl: 100.0,
                pnl_pct: 5.0,
                duration_hours: 24.0,
                strategy: Some("test".to_string()),
            },
            Trade {
                entry_time: Utc::now(),
                exit_time: Utc::now(),
                signal: crate::Signal::Sell,
                entry_price: 2100.0,
                exit_price: 2050.0,
                size: 1.0,
                pnl: -50.0,
                pnl_pct: -2.5,
                duration_hours: 12.0,
                strategy: Some("test".to_string()),
            },
        ];

        let metrics = PerformanceMetrics::from_trades(&trades, 10000.0);

        assert_eq!(metrics.total_trades, 2);
        assert_eq!(metrics.winning_trades, 1);
        assert_eq!(metrics.losing_trades, 1);
        assert_eq!(metrics.win_rate, 50.0);
        assert_eq!(metrics.total_pnl, 50.0);
    }
}
