//! Backtesting framework module

use crate::config::{BacktestConfig, Config};
use crate::risk_management::RiskManager;
use crate::strategies::Strategy;
use crate::{EquityPoint, Error, MarketData, Result, Signal, Trade};
use tracing::{info, warn};

pub mod metrics;

pub use metrics::PerformanceMetrics;

/// Backtesting engine
pub struct BacktestEngine {
    config: Config,
}

impl BacktestEngine {
    /// Create a new backtest engine
    pub fn new(config: Config) -> Self {
        info!("BacktestEngine initialized");
        info!(
            "Commission: {:.3}%, Slippage: {:.3}%",
            config.backtest.commission * 100.0,
            config.backtest.slippage * 100.0
        );

        Self { config }
    }

    /// Run backtest on historical data
    pub fn run(
        &self,
        strategy: &dyn Strategy,
        data: &MarketData,
    ) -> Result<BacktestResults> {
        info!(
            "Starting backtest from {} to {}",
            data.candles.first().map(|c| c.timestamp.to_string()).unwrap_or_default(),
            data.candles.last().map(|c| c.timestamp.to_string()).unwrap_or_default()
        );
        info!("Total data points: {}", data.len());

        let mut risk_manager = RiskManager::new(
            self.config.risk.clone(),
            self.config.trading.initial_capital,
            self.config.trading.max_positions,
        );

        let mut equity_curve = Vec::new();
        let mut all_trades = Vec::new();

        // Calculate indicators once for the entire dataset
        let indicators = strategy.calculate_indicators(data)?;

        let lookback = self.config.strategy.slow_ma.max(self.config.strategy.rsi_period);

        for i in lookback..data.len() {
            // Create slice of data up to current point
            let current_data = MarketData {
                symbol: data.symbol.clone(),
                candles: data.candles[..=i].to_vec(),
            };

            let current_candle = &data.candles[i];
            let current_price = current_candle.close;
            let current_time = current_candle.timestamp;

            // Update existing positions
            let positions_to_close = risk_manager.update_positions(current_price, current_time);

            // Close triggered positions
            for position in positions_to_close {
                let exit_price = self.apply_slippage(current_price, Signal::from_i8(-position.signal.to_i8()).unwrap());
                let commission = self.calculate_commission(exit_price, position.size);
                let slippage = (exit_price - current_price).abs();

                let trade = risk_manager.close_position(
                    &position,
                    exit_price,
                    current_time,
                    commission,
                    slippage,
                );
                all_trades.push(trade);
            }

            // Generate signal
            let signal = strategy.generate_signal(&current_data, &indicators)?;

            // Check if we can trade
            if signal != Signal::Hold && risk_manager.can_trade(signal) {
                let entry_price = self.apply_slippage(current_price, signal);
                let position_size = risk_manager.calculate_position_size(signal, entry_price);

                if position_size > 0.0 {
                    risk_manager.open_position(signal, entry_price, position_size, current_time);
                }
            }

            // Record equity
            let equity = self.calculate_equity(&risk_manager, current_price);
            equity_curve.push(EquityPoint {
                timestamp: current_time,
                equity,
                price: current_price,
            });
        }

        // Close any remaining positions at the end
        let final_candle = data.candles.last().unwrap();
        let final_price = final_candle.close;
        let final_time = final_candle.timestamp;

        for position in risk_manager.open_positions.clone() {
            let exit_price = self.apply_slippage(final_price, Signal::from_i8(-position.signal.to_i8()).unwrap());
            let commission = self.calculate_commission(exit_price, position.size);
            let slippage = (exit_price - final_price).abs();

            let trade = risk_manager.close_position(
                &position,
                exit_price,
                final_time,
                commission,
                slippage,
            );
            all_trades.push(trade);
            info!("Position closed at end of backtest: ${:.2}", trade.pnl);
        }

        // Calculate performance metrics
        let metrics = PerformanceMetrics::calculate(
            &all_trades,
            &equity_curve,
            self.config.trading.initial_capital,
        );

        info!("Backtest completed. Total trades: {}", all_trades.len());
        info!("Final capital: ${:,.2}", risk_manager.capital());

        Ok(BacktestResults {
            trades: all_trades,
            equity_curve,
            metrics,
            final_capital: risk_manager.capital(),
        })
    }

    /// Apply slippage to price
    fn apply_slippage(&self, price: f64, signal: Signal) -> f64 {
        let slippage = self.config.backtest.slippage;
        match signal {
            Signal::Buy => price * (1.0 + slippage),
            Signal::Sell => price * (1.0 - slippage),
            Signal::Hold => price,
        }
    }

    /// Calculate commission
    fn calculate_commission(&self, price: f64, size: f64) -> f64 {
        price * size * self.config.backtest.commission
    }

    /// Calculate current equity including unrealized P&L
    fn calculate_equity(&self, risk_manager: &RiskManager, current_price: f64) -> f64 {
        let mut equity = risk_manager.capital();

        for position in &risk_manager.open_positions {
            equity += position.unrealized_pnl(current_price);
        }

        equity
    }
}

/// Backtest results
#[derive(Debug, Clone)]
pub struct BacktestResults {
    pub trades: Vec<Trade>,
    pub equity_curve: Vec<EquityPoint>,
    pub metrics: PerformanceMetrics,
    pub final_capital: f64,
}
