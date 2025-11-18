//! Performance metrics calculation

use crate::{EquityPoint, Trade};
use serde::{Deserialize, Serialize};

/// Performance metrics for backtest results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    // Basic metrics
    pub total_trades: usize,
    pub initial_capital: f64,
    pub final_capital: f64,
    pub net_profit: f64,
    pub total_return_pct: f64,

    // Return metrics
    pub annualized_return_pct: f64,
    pub avg_daily_return_pct: f64,

    // Risk metrics
    pub max_drawdown_pct: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub volatility_pct: f64,

    // Win/Loss metrics
    pub win_rate_pct: f64,
    pub num_winning_trades: usize,
    pub num_losing_trades: usize,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub largest_win: f64,
    pub largest_loss: f64,
    pub profit_factor: f64,

    // Additional metrics
    pub avg_trade_duration_hours: f64,
}

impl PerformanceMetrics {
    /// Calculate all performance metrics
    pub fn calculate(
        trades: &[Trade],
        equity_curve: &[EquityPoint],
        initial_capital: f64,
    ) -> Self {
        let total_trades = trades.len();
        let final_capital = equity_curve.last().map(|e| e.equity).unwrap_or(initial_capital);
        let net_profit = final_capital - initial_capital;
        let total_return_pct = (net_profit / initial_capital) * 100.0;

        // Calculate returns
        let mut daily_returns = Vec::new();
        for i in 1..equity_curve.len() {
            let ret = (equity_curve[i].equity - equity_curve[i - 1].equity)
                / equity_curve[i - 1].equity;
            daily_returns.push(ret);
        }

        let avg_daily_return_pct = if !daily_returns.is_empty() {
            daily_returns.iter().sum::<f64>() / daily_returns.len() as f64 * 100.0
        } else {
            0.0
        };

        // Annualized return
        let total_days = if equity_curve.len() >= 2 {
            (equity_curve.last().unwrap().timestamp - equity_curve.first().unwrap().timestamp)
                .num_days() as f64
        } else {
            365.0
        };
        let years = total_days / 365.25;
        let annualized_return_pct = if years > 0.0 {
            ((final_capital / initial_capital).powf(1.0 / years) - 1.0) * 100.0
        } else {
            0.0
        };

        // Max drawdown
        let mut peak = initial_capital;
        let mut max_dd = 0.0;
        for point in equity_curve {
            if point.equity > peak {
                peak = point.equity;
            }
            let dd = ((peak - point.equity) / peak) * 100.0;
            if dd > max_dd {
                max_dd = dd;
            }
        }

        // Sharpe ratio
        let risk_free_rate = 0.02 / 252.0;
        let std_dev = if daily_returns.len() > 1 {
            let mean = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
            let variance = daily_returns
                .iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>()
                / (daily_returns.len() - 1) as f64;
            variance.sqrt()
        } else {
            0.0
        };

        let sharpe_ratio = if std_dev > 0.0 {
            let mean_return = if !daily_returns.is_empty() {
                daily_returns.iter().sum::<f64>() / daily_returns.len() as f64
            } else {
                0.0
            };
            ((mean_return - risk_free_rate) / std_dev) * (252.0_f64).sqrt()
        } else {
            0.0
        };

        // Sortino ratio
        let downside_returns: Vec<f64> = daily_returns.iter().filter(|&&r| r < 0.0).copied().collect();
        let downside_std = if downside_returns.len() > 1 {
            let mean = downside_returns.iter().sum::<f64>() / downside_returns.len() as f64;
            let variance = downside_returns
                .iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>()
                / (downside_returns.len() - 1) as f64;
            variance.sqrt()
        } else {
            0.0
        };

        let sortino_ratio = if downside_std > 0.0 {
            let mean_return = if !daily_returns.is_empty() {
                daily_returns.iter().sum::<f64>() / daily_returns.len() as f64
            } else {
                0.0
            };
            ((mean_return - risk_free_rate) / downside_std) * (252.0_f64).sqrt()
        } else {
            0.0
        };

        let volatility_pct = std_dev * (252.0_f64).sqrt() * 100.0;

        // Win/Loss metrics
        let winning_trades: Vec<&Trade> = trades.iter().filter(|t| t.is_winner()).collect();
        let losing_trades: Vec<&Trade> = trades.iter().filter(|t| !t.is_winner()).collect();

        let num_winning_trades = winning_trades.len();
        let num_losing_trades = losing_trades.len();
        let win_rate_pct = if total_trades > 0 {
            (num_winning_trades as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };

        let avg_win = if !winning_trades.is_empty() {
            winning_trades.iter().map(|t| t.pnl).sum::<f64>() / winning_trades.len() as f64
        } else {
            0.0
        };

        let avg_loss = if !losing_trades.is_empty() {
            losing_trades.iter().map(|t| t.pnl).sum::<f64>() / losing_trades.len() as f64
        } else {
            0.0
        };

        let largest_win = winning_trades.iter().map(|t| t.pnl).fold(0.0, f64::max);
        let largest_loss = losing_trades.iter().map(|t| t.pnl).fold(0.0, f64::min);

        let total_wins: f64 = winning_trades.iter().map(|t| t.pnl).sum();
        let total_losses: f64 = losing_trades.iter().map(|t| t.pnl).sum();
        let profit_factor = if total_losses != 0.0 {
            (total_wins / total_losses.abs())
        } else {
            0.0
        };

        let avg_trade_duration_hours = if !trades.is_empty() {
            trades.iter().map(|t| t.duration_hours).sum::<f64>() / trades.len() as f64
        } else {
            0.0
        };

        Self {
            total_trades,
            initial_capital,
            final_capital,
            net_profit,
            total_return_pct,
            annualized_return_pct,
            avg_daily_return_pct,
            max_drawdown_pct: max_dd,
            sharpe_ratio,
            sortino_ratio,
            volatility_pct,
            win_rate_pct,
            num_winning_trades,
            num_losing_trades,
            avg_win,
            avg_loss,
            largest_win,
            largest_loss,
            profit_factor,
            avg_trade_duration_hours,
        }
    }

    /// Print formatted summary
    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(60));
        println!("BACKTEST PERFORMANCE SUMMARY");
        println!("{}", "=".repeat(60));

        println!("\n--- Basic Metrics ---");
        println!("Total Trades: {}", self.total_trades);
        println!("Initial Capital: ${:,.2}", self.initial_capital);
        println!("Final Capital: ${:,.2}", self.final_capital);
        println!("Net Profit: ${:,.2}", self.net_profit);
        println!("Total Return: {:.2}%", self.total_return_pct);

        println!("\n--- Return Metrics ---");
        println!("Annualized Return: {:.2}%", self.annualized_return_pct);
        println!("Avg Daily Return: {:.3}%", self.avg_daily_return_pct);

        println!("\n--- Risk Metrics ---");
        println!("Max Drawdown: {:.2}%", self.max_drawdown_pct);
        println!("Sharpe Ratio: {:.2}", self.sharpe_ratio);
        println!("Sortino Ratio: {:.2}", self.sortino_ratio);
        println!("Volatility: {:.2}%", self.volatility_pct);

        println!("\n--- Win/Loss Metrics ---");
        println!("Win Rate: {:.2}%", self.win_rate_pct);
        println!("Winning Trades: {}", self.num_winning_trades);
        println!("Losing Trades: {}", self.num_losing_trades);
        println!("Avg Win: ${:,.2}", self.avg_win);
        println!("Avg Loss: ${:,.2}", self.avg_loss);
        println!("Largest Win: ${:,.2}", self.largest_win);
        println!("Largest Loss: ${:,.2}", self.largest_loss);
        println!("Profit Factor: {:.2}", self.profit_factor);

        println!("\n--- Additional Metrics ---");
        println!("Avg Trade Duration: {:.1} hours", self.avg_trade_duration_hours);

        println!("\n{}", "=".repeat(60));
    }
}
