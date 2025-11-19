//! Trade Analysis and Learning Module
//!
//! Systematically analyzes losing trades to identify patterns and improve strategies.

use crate::{
    types::{Signal, Trade},
    Result,
};
use chrono::{DateTime, Utc, Timelike, Datelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Analysis of losing trades to identify patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeAnalysis {
    pub trade_id: usize,
    pub timestamp: DateTime<Utc>,
    pub signal: Signal,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub duration_hours: f64,
    pub strategy: String,

    // Failure classification
    pub failure_type: FailureType,
    pub failure_reason: String,
    pub lessons_learned: Vec<String>,

    // Market context at time of trade
    pub market_regime: String,
    pub volatility_percentile: f64,
    pub news_sentiment: Option<f64>,
    pub time_of_day: u32,
    pub day_of_week: String,
}

/// Classification of why a trade failed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FailureType {
    /// False breakout - price reversed immediately
    FalseBreakout,

    /// Stopped out but direction was correct
    PrematureStopLoss,

    /// Market regime changed mid-trade
    RegimeChange,

    /// High volatility whipsaw
    Whipsaw,

    /// News event caused unexpected move
    NewsShock,

    /// Entry timing was poor (too early/late)
    PoorTiming,

    /// Position size was too large
    Overleveraged,

    /// Strategy not suited for current conditions
    WrongStrategy,

    /// Technical indicator gave false signal
    FalseSignal,

    /// Normal loss (stop loss working as intended)
    NormalLoss,
}

/// Monthly performance review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyReview {
    pub month: String,  // "2024-02"
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,

    // Failure analysis
    pub failure_breakdown: HashMap<FailureType, usize>,
    pub most_common_failure: FailureType,
    pub most_expensive_failure: FailureType,

    // Temporal patterns
    pub best_trading_hours: Vec<u32>,
    pub worst_trading_hours: Vec<u32>,
    pub best_day_of_week: String,
    pub worst_day_of_week: String,

    // Strategy performance
    pub best_strategy: String,
    pub worst_strategy: String,
    pub strategy_performance: HashMap<String, StrategyMetrics>,

    // Market regime performance
    pub regime_performance: HashMap<String, f64>,

    // Actionable insights
    pub key_insights: Vec<String>,
    pub action_items: Vec<ActionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyMetrics {
    pub trades: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub avg_pnl: f64,
    pub sharpe_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub priority: Priority,
    pub category: ActionCategory,
    pub description: String,
    pub expected_impact: String,
    pub implementation_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Critical,  // Fix immediately
    High,      // Fix this week
    Medium,    // Fix this month
    Low,       // Nice to have
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionCategory {
    ParameterAdjustment,
    StrategyDisable,
    RiskReduction,
    TimeFilter,
    RegimeFilter,
    StopLossOptimization,
    PositionSizing,
    Other,
}

/// Trade analyzer
pub struct TradeAnalyzer {
    trades: Vec<Trade>,
}

impl TradeAnalyzer {
    pub fn new() -> Self {
        Self {
            trades: Vec::new(),
        }
    }

    pub fn add_trade(&mut self, trade: Trade) {
        self.trades.push(trade);
    }

    /// Analyze a losing trade to classify the failure
    pub fn analyze_losing_trade(
        &self,
        trade: &Trade,
        market_data: &crate::types::MarketData,
        news_sentiment: Option<f64>,
    ) -> TradeAnalysis {
        let failure_type = self.classify_failure(trade, market_data);
        let failure_reason = self.explain_failure(&failure_type, trade);
        let lessons_learned = self.extract_lessons(&failure_type, trade);

        TradeAnalysis {
            trade_id: 0, // Would be set from database
            timestamp: trade.exit_time,
            signal: trade.signal,
            entry_price: trade.entry_price,
            exit_price: trade.exit_price,
            pnl: trade.pnl,
            pnl_pct: trade.pnl_pct,
            duration_hours: trade.duration_hours,
            strategy: trade.strategy.clone(),
            failure_type,
            failure_reason,
            lessons_learned,
            market_regime: self.detect_market_regime(market_data),
            volatility_percentile: self.calculate_volatility_percentile(market_data),
            news_sentiment,
            time_of_day: trade.entry_time.hour(),
            day_of_week: format!("{:?}", trade.entry_time.weekday()),
        }
    }

    /// Classify why a trade failed
    fn classify_failure(
        &self,
        trade: &Trade,
        market_data: &crate::types::MarketData,
    ) -> FailureType {
        // Check for false breakout
        if self.is_false_breakout(trade, market_data) {
            return FailureType::FalseBreakout;
        }

        // Check if stopped out but direction was right
        if self.is_premature_stop(trade, market_data) {
            return FailureType::PrematureStopLoss;
        }

        // Check for high volatility whipsaw
        if self.is_whipsaw(trade, market_data) {
            return FailureType::Whipsaw;
        }

        // Check for regime change
        if self.is_regime_change(trade, market_data) {
            return FailureType::RegimeChange;
        }

        // Check if news caused the loss
        if self.is_news_shock(trade) {
            return FailureType::NewsShock;
        }

        // Check entry timing
        if self.is_poor_timing(trade, market_data) {
            return FailureType::PoorTiming;
        }

        // Default: normal loss
        FailureType::NormalLoss
    }

    /// Check if trade was a false breakout
    fn is_false_breakout(
        &self,
        trade: &Trade,
        market_data: &crate::types::MarketData,
    ) -> bool {
        // If price reversed within 2 hours and moved against us > 0.3%
        if trade.duration_hours < 2.0 && trade.pnl_pct < -0.3 {
            return true;
        }
        false
    }

    /// Check if stop was too tight (direction was right but stopped out)
    fn is_premature_stop(
        &self,
        trade: &Trade,
        market_data: &crate::types::MarketData,
    ) -> bool {
        // Check if price later moved in our direction significantly
        // Would need to check market data after exit
        // For now, check if loss was small and duration was short
        if trade.pnl_pct > -1.0 && trade.pnl_pct < -0.2 && trade.duration_hours < 4.0 {
            return true;
        }
        false
    }

    /// Check if loss was due to volatility whipsaw
    fn is_whipsaw(&self, trade: &Trade, market_data: &crate::types::MarketData) -> bool {
        // High volatility + quick reversal
        let volatility = self.calculate_volatility_percentile(market_data);
        if volatility > 80.0 && trade.duration_hours < 3.0 {
            return true;
        }
        false
    }

    /// Check if market regime changed during trade
    fn is_regime_change(
        &self,
        trade: &Trade,
        market_data: &crate::types::MarketData,
    ) -> bool {
        // Would compare regime at entry vs exit
        // For now, check if trade lasted long and had reversal
        if trade.duration_hours > 12.0 && trade.pnl_pct < -1.5 {
            return true;
        }
        false
    }

    /// Check if news event caused the loss
    fn is_news_shock(&self, trade: &Trade) -> bool {
        // Check if high impact news occurred during trade
        // Would need news event calendar integration
        // For now, check for sudden large moves
        if trade.pnl_pct < -2.0 && trade.duration_hours < 1.0 {
            return true;
        }
        false
    }

    /// Check if entry timing was poor
    fn is_poor_timing(&self, trade: &Trade, market_data: &crate::types::MarketData) -> bool {
        // Check if entered during low liquidity hours
        let hour = trade.entry_time.hour();

        // Low liquidity hours: 22:00-2:00 UTC
        if hour >= 22 || hour <= 2 {
            return true;
        }
        false
    }

    /// Explain the failure in human-readable form
    fn explain_failure(&self, failure_type: &FailureType, trade: &Trade) -> String {
        match failure_type {
            FailureType::FalseBreakout => {
                format!("False breakout: Price reversed within {} hours, losing {:.2}%. \
                        Breakouts need confirmation with volume or multiple timeframe alignment.",
                        trade.duration_hours, -trade.pnl_pct)
            }
            FailureType::PrematureStopLoss => {
                format!("Stop loss too tight: Lost {:.2}% in {} hours. \
                        Consider wider stops based on ATR (current stop may be < 1.5x ATR).",
                        -trade.pnl_pct, trade.duration_hours)
            }
            FailureType::Whipsaw => {
                format!("Volatility whipsaw: High volatility period caused quick reversal. \
                        Avoid trading during extreme volatility or use wider stops.")
            }
            FailureType::RegimeChange => {
                format!("Market regime changed during trade. \
                        Consider shorter holding periods or trailing stops to adapt faster.")
            }
            FailureType::NewsShock => {
                format!("Unexpected news event caused {:.2}% loss. \
                        Implement news calendar filter to avoid trading around high-impact events.",
                        -trade.pnl_pct)
            }
            FailureType::PoorTiming => {
                format!("Entry during low liquidity hours ({}:00 UTC). \
                        Avoid trading during 22:00-02:00 UTC when spreads are wider.",
                        trade.entry_time.hour())
            }
            FailureType::WrongStrategy => {
                format!("Strategy '{}' not suited for current market conditions. \
                        Consider using ensemble or regime-based strategy selection.",
                        trade.strategy)
            }
            FailureType::FalseSignal => {
                format!("Technical indicator gave false signal. \
                        Add confirmation from multiple indicators before entering.")
            }
            FailureType::Overleveraged => {
                format!("Position size too large. Reduce to maximum 2% risk per trade.")
            }
            FailureType::NormalLoss => {
                format!("Normal loss - stop loss working as intended. No action needed.")
            }
        }
    }

    /// Extract actionable lessons from the failure
    fn extract_lessons(&self, failure_type: &FailureType, trade: &Trade) -> Vec<String> {
        let mut lessons = Vec::new();

        match failure_type {
            FailureType::FalseBreakout => {
                lessons.push("Wait for breakout confirmation with volume surge".to_string());
                lessons.push("Use multiple timeframe analysis before entering breakouts".to_string());
                lessons.push("Consider reducing position size on first breakout attempt".to_string());
            }
            FailureType::PrematureStopLoss => {
                lessons.push(format!("Increase stop loss to at least 2x ATR (currently may be ~{}%)",
                    -trade.pnl_pct));
                lessons.push("Use volatility-adjusted stops instead of fixed percentage".to_string());
            }
            FailureType::Whipsaw => {
                lessons.push("Avoid trading when ATR is > 90th percentile".to_string());
                lessons.push("Use wider stops during high volatility periods".to_string());
                lessons.push("Reduce position size by 50% during extreme volatility".to_string());
            }
            FailureType::RegimeChange => {
                lessons.push("Implement trailing stops to lock in profits".to_string());
                lessons.push("Set maximum hold time based on strategy (e.g., 24 hours)".to_string());
                lessons.push("Monitor ADX for trend strength changes".to_string());
            }
            FailureType::NewsShock => {
                lessons.push("Close all positions 30 minutes before high-impact news".to_string());
                lessons.push("Avoid entering new trades 1 hour before/after news".to_string());
                lessons.push("Use tighter stops during news-heavy periods".to_string());
            }
            FailureType::PoorTiming => {
                lessons.push(format!("Disable trading during {}:00-{}:00 UTC",
                    if trade.entry_time.hour() >= 22 { 22 } else { 0 },
                    if trade.entry_time.hour() >= 22 { 2 } else { trade.entry_time.hour() + 1 }));
                lessons.push("Best Gold trading hours: 13:00-21:00 UTC (London-NY overlap)".to_string());
            }
            FailureType::WrongStrategy => {
                lessons.push(format!("Disable '{}' strategy during current market regime", trade.strategy));
                lessons.push("Use ensemble strategy for better regime adaptation".to_string());
            }
            FailureType::FalseSignal => {
                lessons.push("Require 2+ indicator confirmations before entry".to_string());
                lessons.push("Add RSI divergence as confirmation filter".to_string());
            }
            FailureType::Overleveraged => {
                lessons.push("Reduce position size to 1-2% risk per trade".to_string());
                lessons.push("Implement maximum 3 concurrent positions limit".to_string());
            }
            FailureType::NormalLoss => {
                lessons.push("Loss within expected parameters - no changes needed".to_string());
            }
        }

        lessons
    }

    /// Detect current market regime
    fn detect_market_regime(&self, market_data: &crate::types::MarketData) -> String {
        // Simplified regime detection
        // In production, would use ADX, ATR, BB width, etc.
        "Normal".to_string()
    }

    /// Calculate where current volatility stands historically (0-100 percentile)
    fn calculate_volatility_percentile(&self, market_data: &crate::types::MarketData) -> f64 {
        // Simplified calculation
        // In production, would calculate ATR percentile over rolling window
        50.0
    }

    /// Generate monthly review with actionable insights
    pub fn generate_monthly_review(&self, month: &str) -> MonthlyReview {
        let month_trades: Vec<&Trade> = self.trades
            .iter()
            .filter(|t| t.exit_time.format("%Y-%m").to_string() == month)
            .collect();

        if month_trades.is_empty() {
            return self.empty_review(month);
        }

        let total_trades = month_trades.len();
        let winning_trades = month_trades.iter().filter(|t| t.pnl > 0.0).count();
        let losing_trades = total_trades - winning_trades;
        let win_rate = (winning_trades as f64 / total_trades as f64) * 100.0;
        let total_pnl: f64 = month_trades.iter().map(|t| t.pnl).sum();

        // Analyze failures
        let mut failure_breakdown = HashMap::new();
        for trade in month_trades.iter().filter(|t| t.pnl < 0.0) {
            // Simplified - in production would use full analysis
            let failure_type = FailureType::NormalLoss; // Placeholder
            *failure_breakdown.entry(failure_type).or_insert(0) += 1;
        }

        // Temporal analysis
        let hourly_pnl = self.analyze_by_hour(&month_trades);
        let daily_pnl = self.analyze_by_day(&month_trades);

        // Strategy performance
        let strategy_performance = self.analyze_by_strategy(&month_trades);

        // Generate insights and action items
        let key_insights = self.generate_insights(
            win_rate,
            &strategy_performance,
            &hourly_pnl,
            &daily_pnl,
        );
        let action_items = self.generate_action_items(&key_insights, win_rate, total_pnl);

        MonthlyReview {
            month: month.to_string(),
            total_trades,
            winning_trades,
            losing_trades,
            win_rate,
            total_pnl,
            sharpe_ratio: 0.0, // Would calculate properly
            max_drawdown: 0.0, // Would calculate properly
            failure_breakdown,
            most_common_failure: FailureType::NormalLoss, // Placeholder
            most_expensive_failure: FailureType::NormalLoss, // Placeholder
            best_trading_hours: hourly_pnl.iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|(h, _)| vec![*h])
                .unwrap_or_default(),
            worst_trading_hours: hourly_pnl.iter()
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|(h, _)| vec![*h])
                .unwrap_or_default(),
            best_day_of_week: daily_pnl.iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|(d, _)| d.clone())
                .unwrap_or_default(),
            worst_day_of_week: daily_pnl.iter()
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|(d, _)| d.clone())
                .unwrap_or_default(),
            best_strategy: strategy_performance.iter()
                .max_by(|a, b| a.1.total_pnl.partial_cmp(&b.1.total_pnl).unwrap())
                .map(|(s, _)| s.clone())
                .unwrap_or_default(),
            worst_strategy: strategy_performance.iter()
                .min_by(|a, b| a.1.total_pnl.partial_cmp(&b.1.total_pnl).unwrap())
                .map(|(s, _)| s.clone())
                .unwrap_or_default(),
            strategy_performance,
            regime_performance: HashMap::new(), // Would populate
            key_insights,
            action_items,
        }
    }

    fn empty_review(&self, month: &str) -> MonthlyReview {
        MonthlyReview {
            month: month.to_string(),
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: 0.0,
            total_pnl: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown: 0.0,
            failure_breakdown: HashMap::new(),
            most_common_failure: FailureType::NormalLoss,
            most_expensive_failure: FailureType::NormalLoss,
            best_trading_hours: vec![],
            worst_trading_hours: vec![],
            best_day_of_week: String::new(),
            worst_day_of_week: String::new(),
            best_strategy: String::new(),
            worst_strategy: String::new(),
            strategy_performance: HashMap::new(),
            regime_performance: HashMap::new(),
            key_insights: vec!["No trades this month".to_string()],
            action_items: vec![],
        }
    }

    fn analyze_by_hour(&self, trades: &[&Trade]) -> HashMap<u32, f64> {
        let mut hourly_pnl = HashMap::new();
        for trade in trades {
            let hour = trade.entry_time.hour();
            *hourly_pnl.entry(hour).or_insert(0.0) += trade.pnl;
        }
        hourly_pnl
    }

    fn analyze_by_day(&self, trades: &[&Trade]) -> HashMap<String, f64> {
        let mut daily_pnl = HashMap::new();
        for trade in trades {
            let day = format!("{:?}", trade.entry_time.weekday());
            *daily_pnl.entry(day).or_insert(0.0) += trade.pnl;
        }
        daily_pnl
    }

    fn analyze_by_strategy(&self, trades: &[&Trade]) -> HashMap<String, StrategyMetrics> {
        let mut strategy_stats: HashMap<String, Vec<&Trade>> = HashMap::new();
        for trade in trades {
            strategy_stats.entry(trade.strategy.clone())
                .or_insert_with(Vec::new)
                .push(trade);
        }

        let mut strategy_performance = HashMap::new();
        for (strategy, strat_trades) in strategy_stats {
            let total = strat_trades.len();
            let wins = strat_trades.iter().filter(|t| t.pnl > 0.0).count();
            let win_rate = (wins as f64 / total as f64) * 100.0;
            let total_pnl: f64 = strat_trades.iter().map(|t| t.pnl).sum();
            let avg_pnl = total_pnl / total as f64;

            strategy_performance.insert(strategy, StrategyMetrics {
                trades: total,
                win_rate,
                total_pnl,
                avg_pnl,
                sharpe_ratio: 0.0, // Would calculate
            });
        }

        strategy_performance
    }

    fn generate_insights(
        &self,
        win_rate: f64,
        strategy_performance: &HashMap<String, StrategyMetrics>,
        hourly_pnl: &HashMap<u32, f64>,
        daily_pnl: &HashMap<String, f64>,
    ) -> Vec<String> {
        let mut insights = Vec::new();

        // Win rate insights
        if win_rate >= 60.0 {
            insights.push(format!("✅ Excellent win rate of {:.1}% - system performing well", win_rate));
        } else if win_rate >= 52.0 {
            insights.push(format!("✓ Win rate of {:.1}% is acceptable but has room for improvement", win_rate));
        } else {
            insights.push(format!("⚠️ Win rate of {:.1}% is below target - review strategy selection", win_rate));
        }

        // Strategy insights
        if let Some((best_strat, metrics)) = strategy_performance.iter()
            .max_by(|a, b| a.1.total_pnl.partial_cmp(&b.1.total_pnl).unwrap())
        {
            insights.push(format!("💰 '{}' was most profitable: ${:.2} ({:.1}% win rate)",
                best_strat, metrics.total_pnl, metrics.win_rate));
        }

        if let Some((worst_strat, metrics)) = strategy_performance.iter()
            .min_by(|a, b| a.1.total_pnl.partial_cmp(&b.1.total_pnl).unwrap())
        {
            if metrics.total_pnl < -50.0 {
                insights.push(format!("❌ '{}' lost ${:.2} - consider disabling temporarily",
                    worst_strat, -metrics.total_pnl));
            }
        }

        // Temporal insights
        if let Some((best_hour, pnl)) = hourly_pnl.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        {
            if *pnl > 100.0 {
                insights.push(format!("⏰ Hour {}:00 UTC was most profitable (${:.2})",
                    best_hour, pnl));
            }
        }

        insights
    }

    fn generate_action_items(
        &self,
        insights: &[String],
        win_rate: f64,
        total_pnl: f64,
    ) -> Vec<ActionItem> {
        let mut actions = Vec::new();

        // Critical actions for poor performance
        if win_rate < 45.0 {
            actions.push(ActionItem {
                priority: Priority::Critical,
                category: ActionCategory::StrategyDisable,
                description: "Win rate critically low - halt live trading immediately".to_string(),
                expected_impact: "Prevent further losses while system is re-evaluated".to_string(),
                implementation_steps: vec![
                    "Switch to paper trading mode".to_string(),
                    "Run comprehensive backtest on recent data".to_string(),
                    "Review and re-optimize parameters".to_string(),
                ],
            });
        }

        // High priority for negative month
        if total_pnl < -100.0 {
            actions.push(ActionItem {
                priority: Priority::High,
                category: ActionCategory::RiskReduction,
                description: "Significant monthly loss - reduce risk immediately".to_string(),
                expected_impact: "Limit losses while maintaining learning".to_string(),
                implementation_steps: vec![
                    "Reduce position sizes by 50%".to_string(),
                    "Tighten stop losses to 1% max".to_string(),
                    "Limit to maximum 2 concurrent positions".to_string(),
                ],
            });
        }

        // Medium priority for optimization
        if win_rate < 55.0 && win_rate >= 45.0 {
            actions.push(ActionItem {
                priority: Priority::Medium,
                category: ActionCategory::ParameterAdjustment,
                description: "Win rate below optimal - optimize parameters".to_string(),
                expected_impact: "Improve win rate by 3-5%".to_string(),
                implementation_steps: vec![
                    "Run walk-forward analysis".to_string(),
                    "Test wider stop losses (2x ATR vs 1.5x)".to_string(),
                    "Add confirmation filters to reduce false signals".to_string(),
                ],
            });
        }

        actions
    }
}
