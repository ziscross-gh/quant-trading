//! Autonomous trading execution module

use crate::config::Config;
use crate::data::DataFetcher;
use crate::monitoring::{AlertLevel, MonitoringConfig, MonitoringSystem};
use crate::news::{CalendarConfig, EconomicCalendar};
use crate::regime::{RegimeConfig, RegimeDetector};
use crate::risk_management::RiskManager;
use crate::strategies::Strategy;
use crate::{Result, Signal, Trade};
use chrono::{Datelike, Utc};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

/// Autonomous trader that runs continuously
pub struct AutonomousTrader {
    config: Config,
    data_fetcher: Arc<DataFetcher>,
    strategy: Arc<dyn Strategy>,
    risk_manager: RiskManager,
    economic_calendar: EconomicCalendar,
    regime_detector: RegimeDetector,
    monitoring: MonitoringSystem,
    is_running: bool,
    iteration_count: u64,
}

/// Configuration for production monitoring
#[derive(Clone)]
pub struct ProductionConfig {
    /// Telegram bot token for notifications
    pub telegram_bot_token: Option<String>,
    /// Telegram chat ID for notifications
    pub telegram_chat_id: Option<String>,
    /// Maximum drawdown percentage before alert (default: 10%)
    pub max_drawdown_alert_pct: f64,
    /// Maximum consecutive losses before alert (default: 3)
    pub max_consecutive_losses: u32,
    /// Minimum win rate before alert (default: 45%)
    pub min_win_rate_alert_pct: f64,
}

impl Default for ProductionConfig {
    fn default() -> Self {
        Self {
            telegram_bot_token: std::env::var("TELEGRAM_BOT_TOKEN").ok(),
            telegram_chat_id: std::env::var("TELEGRAM_CHAT_ID").ok(),
            max_drawdown_alert_pct: 10.0,
            max_consecutive_losses: 3,
            min_win_rate_alert_pct: 45.0,
        }
    }
}

impl AutonomousTrader {
    /// Create a new autonomous trader with default monitoring
    pub fn new(config: Config, strategy: Arc<dyn Strategy>) -> Self {
        Self::with_monitoring(config, strategy, ProductionConfig::default())
    }

    /// Create a new autonomous trader with custom monitoring configuration
    pub fn with_monitoring(
        config: Config,
        strategy: Arc<dyn Strategy>,
        production_config: ProductionConfig,
    ) -> Self {
        let data_fetcher = Arc::new(DataFetcher::new(
            config.trading.symbol.clone(),
            config.data.cache_enabled,
            config.data.cache_dir.clone(),
        ));

        let risk_manager = RiskManager::new(
            config.risk.clone(),
            config.trading.initial_capital,
            config.trading.max_positions,
        );

        // Initialize economic calendar
        let calendar_config = CalendarConfig::default();
        let mut economic_calendar = EconomicCalendar::new(calendar_config);

        // Load upcoming events for current and next month
        let now = Utc::now();
        economic_calendar.load_typical_events(now.year(), now.month());
        if now.month() == 12 {
            economic_calendar.load_typical_events(now.year() + 1, 1);
        } else {
            economic_calendar.load_typical_events(now.year(), now.month() + 1);
        }

        // Initialize regime detector
        let regime_config = RegimeConfig::default();
        let regime_detector = RegimeDetector::new(regime_config);

        // Initialize production monitoring system
        let telegram_enabled = production_config.telegram_bot_token.is_some();
        let monitoring_config = MonitoringConfig {
            telegram_token: production_config.telegram_bot_token,
            telegram_chat_id: production_config.telegram_chat_id,
            telegram_enabled,
            max_drawdown_alert_pct: production_config.max_drawdown_alert_pct,
            max_consecutive_losses: production_config.max_consecutive_losses as usize,
            min_win_rate_alert_pct: production_config.min_win_rate_alert_pct,
            alert_cooldown_secs: 300,
            daily_reports: true,
            daily_report_hour: 17, // 5 PM
        };
        let monitoring = MonitoringSystem::new(monitoring_config, config.trading.initial_capital);

        info!("{}", "=".repeat(60));
        info!("AUTONOMOUS GOLD/USD TRADING SYSTEM STARTED");
        info!("{}", "=".repeat(60));
        info!("Strategy: {}", strategy.name());
        info!("Mode: {:?}", config.trading.trading_mode);
        info!("Initial Capital: ${:.2}", config.trading.initial_capital);
        info!("Economic Calendar: {} events loaded", economic_calendar.get_upcoming_events(now, 720).len());
        info!("Market Regime Detection: ENABLED");
        info!("Production Monitoring: ENABLED");
        info!("{}", "=".repeat(60));

        Self {
            config,
            data_fetcher,
            strategy,
            risk_manager,
            economic_calendar,
            regime_detector,
            monitoring,
            is_running: false,
            iteration_count: 0,
        }
    }

    /// Start the autonomous trading loop
    pub async fn start(&mut self) -> Result<()> {
        self.is_running = true;

        // Send startup notification
        self.monitoring
            .send_alert(crate::monitoring::Alert::new(
                AlertLevel::Info,
                "Trading System Started".to_string(),
                format!(
                    "Strategy: {}\nMode: {:?}\nCapital: ${:.2}",
                    self.strategy.name(),
                    self.config.trading.trading_mode,
                    self.config.trading.initial_capital
                ),
            ))
            .await;

        // Update health status - data feed connected
        self.monitoring.update_health_data_feed(true).await;

        while self.is_running {
            match self.trading_loop().await {
                Ok(_) => {
                    // Update health status on successful iteration
                    self.monitoring.update_health_data_feed(true).await;
                }
                Err(e) => {
                    error!("Trading loop error: {}", e);
                    self.monitoring
                        .send_alert(crate::monitoring::Alert::new(
                            AlertLevel::Error,
                            "Trading Loop Error".to_string(),
                            format!("Error: {}", e),
                        ))
                        .await;
                    // Continue running despite error
                }
            }

            // Sleep for check interval
            let interval = Duration::from_secs(self.config.execution.check_interval);
            sleep(interval).await;
        }

        Ok(())
    }

    /// Stop the autonomous trader
    pub async fn stop(&mut self) {
        info!("Stopping autonomous trader...");
        self.is_running = false;

        // Close all open positions
        if !self.risk_manager.open_positions.is_empty() {
            info!(
                "Closing {} open positions...",
                self.risk_manager.open_positions.len()
            );

            if let Ok((current_price, _)) = self.data_fetcher.get_current_price().await {
                for position in self.risk_manager.open_positions.clone() {
                    let _exit_signal = match position.signal {
                        Signal::Buy => Signal::Sell,
                        Signal::Sell => Signal::Buy,
                        Signal::Hold => Signal::Hold,
                    };

                    let trade = self.risk_manager.close_position(
                        &position,
                        current_price,
                        Utc::now(),
                        0.0,
                        0.0,
                    );
                    // Record trade with monitoring system
                    self.monitoring.record_trade(&trade).await;
                }
            }
        }

        // Get final metrics for shutdown report
        let metrics = self.monitoring.get_metrics().await;
        let summary = self.risk_manager.get_risk_summary();
        let initial = self.config.trading.initial_capital;
        let total_pnl = summary.current_capital - initial;
        let total_pnl_pct = (total_pnl / initial) * 100.0;

        // Send shutdown notification with session summary
        self.monitoring
            .send_alert(crate::monitoring::Alert::new(
                AlertLevel::Info,
                "Trading System Stopped".to_string(),
                format!(
                    "Session Summary:\n\
                     Total Trades: {}\n\
                     Win Rate: {:.1}%\n\
                     Total P&L: ${:.2} ({:.2}%)\n\
                     Max Drawdown: {:.2}%",
                    metrics.total_trades,
                    metrics.win_rate_pct,
                    total_pnl,
                    total_pnl_pct,
                    summary.drawdown_pct
                ),
            ))
            .await;

        self.print_session_summary();
        info!("Autonomous trader stopped");
    }

    /// Main trading loop iteration
    async fn trading_loop(&mut self) -> Result<()> {
        self.iteration_count += 1;
        let now = Utc::now();

        info!("\n{}", "=".repeat(60));
        info!("Iteration #{} - {}", self.iteration_count, now);
        info!("{}", "=".repeat(60));

        // Check for upcoming high-impact economic events
        let upcoming_events = self.economic_calendar.get_upcoming_events(now, 24);
        if !upcoming_events.is_empty() {
            info!("Upcoming economic events in next 24h:");
            for event in &upcoming_events {
                let hours_until = event.scheduled_time.signed_duration_since(now).num_hours();
                info!(
                    "  - {} in {}h ({:?})",
                    event.event_type.name(),
                    hours_until,
                    event.impact
                );
            }
        }

        // Fetch latest data
        info!("Fetching latest market data...");
        let data = self
            .data_fetcher
            .fetch_live("5d", &self.config.data.interval)
            .await?;

        if data.is_empty() {
            warn!("No data received, skipping iteration");
            return Ok(());
        }

        let current_price = data.last().unwrap().close;
        info!("Current Gold price: ${:.2}", current_price);

        // Detect current market regime
        let regime_analysis = self.regime_detector.analyze_regime(&data)?;
        info!("Market Regime: {}", regime_analysis.summary());

        // Log recommended strategies for current regime
        let recommended_strategies = regime_analysis.regime.recommended_strategies();
        info!("  Recommended strategies: {:?}", recommended_strategies);

        // Get position sizing multiplier based on regime
        let regime_position_multiplier = regime_analysis.regime.position_size_multiplier();
        info!("  Position size multiplier: {:.2}x", regime_position_multiplier);

        // Check if positions should be closed before high-impact events
        if let Some(event) = self.economic_calendar.should_close_positions(now) {
            if !self.risk_manager.open_positions.is_empty() {
                let minutes_until = event.scheduled_time.signed_duration_since(now).num_minutes();
                warn!(
                    "🚨 HIGH-IMPACT EVENT ALERT: {} in {} minutes",
                    event.event_type.name(),
                    minutes_until
                );
                warn!("Closing all positions as precaution...");

                // Send alert about event-based closure
                self.monitoring
                    .send_alert(crate::monitoring::Alert::new(
                        AlertLevel::Warning,
                        "Positions Closed - Economic Event".to_string(),
                        format!(
                            "{} in {} minutes\nClosing {} positions as precaution",
                            event.event_type.name(),
                            minutes_until,
                            self.risk_manager.open_positions.len()
                        ),
                    ))
                    .await;

                for position in self.risk_manager.open_positions.clone() {
                    info!("Closing position #{} before event", position.id);
                    let trade = self.risk_manager.close_position(
                        &position,
                        current_price,
                        now,
                        0.0,
                        0.0,
                    );
                    // Record trade with monitoring system
                    self.monitoring.record_trade(&trade).await;
                }
            }
        }

        // Update existing positions (with ATR for dynamic trailing stops)
        self.update_positions(&data, current_price).await;

        // Check if trading should be restricted due to economic events
        let trading_allowed = if let Some(event) = self.economic_calendar.should_restrict_trading(now) {
            let time_until = event.scheduled_time.signed_duration_since(now);
            if time_until.num_minutes() > 0 {
                warn!(
                    "⚠️  Trading restricted: {} event in {} minutes",
                    event.event_type.name(),
                    time_until.num_minutes()
                );
            } else {
                warn!(
                    "⚠️  Trading restricted: {} event ended {} minutes ago",
                    event.event_type.name(),
                    -time_until.num_minutes()
                );
            }
            false
        } else {
            true
        };

        // Generate trading signal
        info!("Evaluating strategy...");
        let result = self.strategy.run(&data)?;

        info!("Signal: {}", result.signal.description());

        // Execute trade if signal and allowed
        if result.signal != Signal::Hold && trading_allowed && self.risk_manager.can_trade(result.signal) {
            self.execute_trade(result.signal, current_price, regime_position_multiplier).await?;
        } else if result.signal != Signal::Hold {
            if !trading_allowed {
                warn!("Trading signal generated but trading restricted (economic event window)");
            } else {
                warn!("Trading signal generated but trading not allowed (risk limits)");
            }
        }

        // Print status
        self.print_status(current_price);

        // Clean up old events periodically
        if self.iteration_count % 100 == 0 {
            self.economic_calendar.clean_old_events(now);
        }

        Ok(())
    }

    /// Update existing positions
    async fn update_positions(&mut self, data: &crate::MarketData, current_price: f64) {
        if self.risk_manager.open_positions.is_empty() {
            return;
        }

        info!(
            "Updating {} open positions...",
            self.risk_manager.open_positions.len()
        );

        // Calculate ATR for dynamic trailing stops
        let atr_period = 14;
        let current_atr = if data.len() >= atr_period + 1 {
            match crate::indicators::atr(
                &data.highs(),
                &data.lows(),
                &data.closes(),
                atr_period,
            ) {
                Ok(atr_values) => {
                    if let Some(atr) = atr_values.last() {
                        if !atr.is_nan() {
                            Some(*atr)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        } else {
            None
        };

        let positions_to_close = self
            .risk_manager
            .update_positions(current_price, Utc::now(), current_atr);

        for position in positions_to_close {
            info!("Closing position due to trigger: {}", position.id);
            let trade = self.risk_manager.close_position(
                &position,
                current_price,
                Utc::now(),
                0.0,
                0.0,
            );
            // Record trade with monitoring system
            self.monitoring.record_trade(&trade).await;
        }
    }

    /// Execute a trade with regime-adjusted position sizing
    async fn execute_trade(&mut self, signal: Signal, current_price: f64, regime_multiplier: f64) -> Result<()> {
        info!("Executing {} order...", signal.description());

        // Calculate base position size
        let base_position_size = self.risk_manager.calculate_position_size(signal, current_price);

        // Apply regime-based adjustment
        let position_size = base_position_size * regime_multiplier;

        if position_size <= 0.0 {
            warn!("Position size is zero, skipping trade");
            return Ok(());
        }

        info!(
            "Position sizing: Base {:.4} oz × Regime {:.2}x = {:.4} oz",
            base_position_size, regime_multiplier, position_size
        );

        let position = self.risk_manager.open_position(
            signal,
            current_price,
            position_size,
            Utc::now(),
        );

        info!(
            "Position opened: {:?} {:.4} units @ ${:.2}",
            signal, position_size, current_price
        );
        info!(
            "  Stop Loss: ${:.2}, Take Profit: ${:.2}",
            position.stop_loss, position.take_profit
        );

        // Send trade entry notification via monitoring system
        self.monitoring
            .send_trade_entry(
                signal,
                current_price,
                position_size,
                position.stop_loss,
                position.take_profit,
            )
            .await;

        Ok(())
    }

    /// Print current status
    fn print_status(&self, current_price: f64) {
        let summary = self.risk_manager.get_risk_summary();

        info!("\n--- Current Status ---");
        info!("Current Capital: ${:.2}", summary.current_capital);
        info!("Peak Capital: ${:.2}", summary.peak_capital);
        info!("Drawdown: {:.2}%", summary.drawdown_pct);
        info!(
            "Daily P&L: ${:.2} ({:.2}%)",
            summary.daily_pnl, summary.daily_pnl_pct
        );
        info!("Open Positions: {}", summary.open_positions);
        info!("Daily Trades: {}", summary.daily_trades);

        // Print open positions
        if !self.risk_manager.open_positions.is_empty() {
            info!("\n--- Open Positions ---");
            for pos in &self.risk_manager.open_positions {
                let unrealized_pnl = pos.unrealized_pnl(current_price);
                let unrealized_pnl_pct = pos.unrealized_pnl_pct(current_price);

                info!(
                    "Position #{}: {:?} {:.4} units @ ${:.2}",
                    pos.id, pos.signal, pos.size, pos.entry_price
                );
                info!(
                    "  Unrealized P&L: ${:.2} ({:.2}%)",
                    unrealized_pnl, unrealized_pnl_pct
                );
                info!(
                    "  Stop Loss: ${:.2}, Take Profit: ${:.2}",
                    pos.stop_loss, pos.take_profit
                );
            }
        }
    }

    /// Print session summary
    fn print_session_summary(&self) {
        let summary = self.risk_manager.get_risk_summary();
        let initial = self.config.trading.initial_capital;
        let total_pnl = summary.current_capital - initial;
        let total_pnl_pct = (total_pnl / initial) * 100.0;

        info!("\n{}", "=".repeat(60));
        info!("TRADING SESSION SUMMARY");
        info!("{}", "=".repeat(60));
        info!("Initial Capital: ${:.2}", initial);
        info!("Final Capital: ${:.2}", summary.current_capital);
        info!("Total P&L: ${:.2} ({:.2}%)", total_pnl, total_pnl_pct);
        info!("Max Drawdown: {:.2}%", summary.drawdown_pct);
        info!("Total Iterations: {}", self.iteration_count);
        info!("{}", "=".repeat(60));
    }
}
