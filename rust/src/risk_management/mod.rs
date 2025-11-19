//! Risk management module

use crate::config::RiskConfig;
use crate::{Error, Position, Result, Signal, Trade, TradingMode};
use chrono::{DateTime, Datelike, Utc};
use std::collections::HashMap;
use tracing::{info, warn};

pub mod position_sizer;
pub mod trailing_stop;

pub use position_sizer::PositionSizer;
pub use trailing_stop::{TrailingStopConfig, TrailingStopManager};

/// Risk manager for controlling trading risk
pub struct RiskManager {
    config: RiskConfig,
    initial_capital: f64,
    current_capital: f64,
    peak_capital: f64,
    daily_start_capital: f64,
    max_positions: usize,
    pub open_positions: Vec<Position>,
    trades: Vec<Trade>,
    daily_trades: Vec<Trade>,
    last_reset_date: DateTime<Utc>,
    next_position_id: u64,
    trailing_stop_manager: TrailingStopManager,
}

impl RiskManager {
    /// Create a new risk manager
    pub fn new(config: RiskConfig, initial_capital: f64, max_positions: usize) -> Self {
        info!(
            "RiskManager initialized with capital: ${:.2}",
            initial_capital
        );
        info!(
            "Risk parameters: Max DD: {}%, Stop Loss: {}%, Risk per trade: {}%",
            config.max_drawdown_pct, config.stop_loss_pct, config.risk_per_trade_pct
        );

        // Create trailing stop manager with default config
        let trailing_config = TrailingStopConfig::default();
        let trailing_stop_manager = TrailingStopManager::new(trailing_config);

        info!(
            "ATR-based trailing stops enabled: activate at {}% profit",
            trailing_stop_manager.config.activation_threshold_pct
        );

        Self {
            config,
            initial_capital,
            current_capital: initial_capital,
            peak_capital: initial_capital,
            daily_start_capital: initial_capital,
            max_positions,
            open_positions: Vec::new(),
            trades: Vec::new(),
            daily_trades: Vec::new(),
            last_reset_date: Utc::now(),
            next_position_id: 1,
            trailing_stop_manager,
        }
    }

    /// Check if trading is allowed based on risk parameters
    pub fn can_trade(&mut self, signal: Signal) -> bool {
        // Reset daily tracking if new day
        self.check_daily_reset();

        // Check max drawdown
        if !self.check_max_drawdown() {
            warn!("Trading halted: Maximum drawdown limit reached");
            return false;
        }

        // Check max daily loss
        if !self.check_max_daily_loss() {
            warn!("Trading halted: Maximum daily loss limit reached");
            return false;
        }

        // Check position capacity
        if signal != Signal::Hold && self.open_positions.len() >= self.max_positions {
            warn!(
                "Trading halted: Maximum positions ({}) reached",
                self.max_positions
            );
            return false;
        }

        true
    }

    /// Calculate position size based on risk parameters
    pub fn calculate_position_size(&self, signal: Signal, entry_price: f64) -> f64 {
        if signal == Signal::Hold {
            return 0.0;
        }

        // Calculate risk amount
        let risk_amount = self.current_capital * (self.config.risk_per_trade_pct / 100.0);

        // Calculate position size based on stop loss
        let stop_loss_distance = entry_price * (self.config.stop_loss_pct / 100.0);
        let position_size = risk_amount / stop_loss_distance;

        // Also limit by maximum position value
        let max_position_value = self.current_capital * 0.1; // 10% max position size
        let max_position_units = max_position_value / entry_price;

        // Use the smaller of the two
        position_size.min(max_position_units)
    }

    /// Calculate stop loss price
    pub fn calculate_stop_loss(&self, entry_price: f64, signal: Signal) -> f64 {
        match signal {
            Signal::Buy => entry_price * (1.0 - self.config.stop_loss_pct / 100.0),
            Signal::Sell => entry_price * (1.0 + self.config.stop_loss_pct / 100.0),
            Signal::Hold => entry_price,
        }
    }

    /// Calculate take profit price
    pub fn calculate_take_profit(&self, entry_price: f64, signal: Signal) -> f64 {
        match signal {
            Signal::Buy => entry_price * (1.0 + self.config.take_profit_pct / 100.0),
            Signal::Sell => entry_price * (1.0 - self.config.take_profit_pct / 100.0),
            Signal::Hold => entry_price,
        }
    }

    /// Open a new position
    pub fn open_position(
        &mut self,
        signal: Signal,
        entry_price: f64,
        size: f64,
        timestamp: DateTime<Utc>,
    ) -> Position {
        let stop_loss = self.calculate_stop_loss(entry_price, signal);
        let take_profit = self.calculate_take_profit(entry_price, signal);

        let position = Position::new(
            self.next_position_id,
            signal,
            entry_price,
            size,
            stop_loss,
            take_profit,
            timestamp,
        );

        self.next_position_id += 1;
        self.open_positions.push(position.clone());

        info!(
            "Position opened: {:?} {:.4} units @ ${:.2}",
            signal, size, entry_price
        );
        info!(
            "  Stop Loss: ${:.2}, Take Profit: ${:.2}",
            stop_loss, take_profit
        );

        position
    }

    /// Update positions and check for exit triggers
    ///
    /// # Arguments
    /// * `current_price` - Current market price
    /// * `timestamp` - Current timestamp
    /// * `current_atr` - Optional ATR value for dynamic trailing stops
    pub fn update_positions(
        &mut self,
        current_price: f64,
        timestamp: DateTime<Utc>,
        current_atr: Option<f64>,
    ) -> Vec<Position> {
        let mut positions_to_close = Vec::new();

        let trailing_stop_pct = self.config.trailing_stop_pct;

        for position in &mut self.open_positions {
            // Update trailing stop using ATR-based manager if ATR is available
            if let Some(atr) = current_atr {
                let stop_adjusted = self.trailing_stop_manager.update_position_trailing_stop(
                    position,
                    current_price,
                    atr,
                );

                if stop_adjusted {
                    let status = self.trailing_stop_manager.get_status_description(position, current_price);
                    info!("Position #{} trailing stop updated: {}", position.id, status);
                }

                // Check if trailing stop was hit
                if self.trailing_stop_manager.is_trailing_stop_hit(position, current_price) {
                    info!(
                        "Trailing stop triggered at ${:.2} (stop: ${:.2})",
                        current_price,
                        position.trailing_stop.unwrap()
                    );
                    positions_to_close.push(position.clone());
                    continue;
                }
            } else {
                // Fallback to percentage-based trailing stop
                Self::update_percentage_trailing_stop_static(position, current_price, trailing_stop_pct);

                // Check if trailing stop was hit
                if let Some(trailing_stop) = position.trailing_stop {
                    let hit = match position.signal {
                        Signal::Buy => current_price <= trailing_stop,
                        Signal::Sell => current_price >= trailing_stop,
                        Signal::Hold => false,
                    };

                    if hit {
                        info!("Trailing stop triggered at ${:.2}", current_price);
                        positions_to_close.push(position.clone());
                        continue;
                    }
                }
            }

            // Check fixed stop loss and take profit
            match position.signal {
                Signal::Buy => {
                    if current_price <= position.stop_loss {
                        info!("Stop loss triggered at ${:.2}", current_price);
                        positions_to_close.push(position.clone());
                    } else if current_price >= position.take_profit {
                        info!("Take profit triggered at ${:.2}", current_price);
                        positions_to_close.push(position.clone());
                    }
                }
                Signal::Sell => {
                    if current_price >= position.stop_loss {
                        info!("Stop loss triggered at ${:.2}", current_price);
                        positions_to_close.push(position.clone());
                    } else if current_price <= position.take_profit {
                        info!("Take profit triggered at ${:.2}", current_price);
                        positions_to_close.push(position.clone());
                    }
                }
                Signal::Hold => {}
            }
        }

        positions_to_close
    }

    /// Fallback: Update trailing stop using percentage-based approach
    fn update_percentage_trailing_stop_static(position: &mut Position, current_price: f64, trailing_stop_pct: f64) {
        match position.signal {
            Signal::Buy => {
                if current_price > position.peak_price {
                    position.peak_price = current_price;
                    let trailing_stop =
                        current_price * (1.0 - trailing_stop_pct / 100.0);

                    if position.trailing_stop.is_none()
                        || trailing_stop > position.trailing_stop.unwrap()
                    {
                        position.trailing_stop = Some(trailing_stop);
                    }
                }
            }
            Signal::Sell => {
                if current_price < position.peak_price {
                    position.peak_price = current_price;
                    let trailing_stop =
                        current_price * (1.0 + trailing_stop_pct / 100.0);

                    if position.trailing_stop.is_none()
                        || trailing_stop < position.trailing_stop.unwrap()
                    {
                        position.trailing_stop = Some(trailing_stop);
                    }
                }
            }
            Signal::Hold => {}
        }
    }

    /// Close a position
    pub fn close_position(
        &mut self,
        position: &Position,
        exit_price: f64,
        timestamp: DateTime<Utc>,
        commission: f64,
        slippage: f64,
    ) -> Trade {
        let trade = Trade::new(position, exit_price, timestamp, commission, slippage, "Unknown".to_string());

        // Update capital
        self.current_capital += trade.pnl;
        if self.current_capital > self.peak_capital {
            self.peak_capital = self.current_capital;
        }

        // Record trade
        self.trades.push(trade.clone());
        self.daily_trades.push(trade.clone());

        // Remove from open positions
        self.open_positions.retain(|p| p.id != position.id);

        info!(
            "Position closed: P&L ${:.2} ({:.2}%)",
            trade.pnl, trade.pnl_pct
        );
        info!("Current capital: ${:.2}", self.current_capital);

        trade
    }

    /// Check maximum drawdown
    fn check_max_drawdown(&self) -> bool {
        let current_drawdown = ((self.peak_capital - self.current_capital) / self.peak_capital) * 100.0;
        current_drawdown < self.config.max_drawdown_pct
    }

    /// Check maximum daily loss
    fn check_max_daily_loss(&self) -> bool {
        let daily_loss =
            ((self.daily_start_capital - self.current_capital) / self.daily_start_capital) * 100.0;
        daily_loss < self.config.max_daily_loss_pct
    }

    /// Reset daily tracking if new day
    fn check_daily_reset(&mut self) {
        let now = Utc::now();
        if now.date_naive() > self.last_reset_date.date_naive() {
            self.daily_start_capital = self.current_capital;
            self.daily_trades.clear();
            self.last_reset_date = now;
            info!(
                "Daily reset: Starting capital ${:.2}",
                self.current_capital
            );
        }
    }

    /// Get risk summary
    pub fn get_risk_summary(&self) -> RiskSummary {
        let current_drawdown =
            ((self.peak_capital - self.current_capital) / self.peak_capital) * 100.0;
        let daily_pnl = self.current_capital - self.daily_start_capital;
        let daily_pnl_pct = (daily_pnl / self.daily_start_capital) * 100.0;

        RiskSummary {
            current_capital: self.current_capital,
            peak_capital: self.peak_capital,
            drawdown_pct: current_drawdown,
            daily_pnl,
            daily_pnl_pct,
            open_positions: self.open_positions.len(),
            daily_trades: self.daily_trades.len(),
        }
    }

    /// Get all trades
    pub fn get_trades(&self) -> &[Trade] {
        &self.trades
    }

    /// Get current capital
    pub fn capital(&self) -> f64 {
        self.current_capital
    }
}

/// Risk summary information
#[derive(Debug, Clone)]
pub struct RiskSummary {
    pub current_capital: f64,
    pub peak_capital: f64,
    pub drawdown_pct: f64,
    pub daily_pnl: f64,
    pub daily_pnl_pct: f64,
    pub open_positions: usize,
    pub daily_trades: usize,
}
