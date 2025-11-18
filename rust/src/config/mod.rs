//! Configuration management module

use crate::{Error, Result, Interval, TradingMode};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub trading: TradingConfig,
    pub data: DataConfig,
    pub strategy: StrategyConfig,
    pub risk: RiskConfig,
    pub backtest: BacktestConfig,
    pub execution: ExecutionConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub news: NewsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub symbol: String,
    #[serde(default = "default_symbol_alt")]
    pub symbol_alt: String,
    #[serde(default = "default_initial_capital")]
    pub initial_capital: f64,
    #[serde(default = "default_position_size")]
    pub position_size: f64,
    #[serde(default = "default_max_positions")]
    pub max_positions: usize,
    #[serde(default = "default_trading_mode")]
    pub trading_mode: TradingMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_interval")]
    pub interval: String,
    #[serde(default = "default_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_cache_enabled")]
    pub cache_enabled: bool,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    #[serde(default = "default_strategy_name")]
    pub name: String,
    #[serde(default = "default_fast_ma")]
    pub fast_ma: usize,
    #[serde(default = "default_slow_ma")]
    pub slow_ma: usize,
    #[serde(default = "default_rsi_period")]
    pub rsi_period: usize,
    #[serde(default = "default_rsi_oversold")]
    pub rsi_oversold: f64,
    #[serde(default = "default_rsi_overbought")]
    pub rsi_overbought: f64,
    #[serde(default = "default_bb_period")]
    pub bb_period: usize,
    #[serde(default = "default_bb_std")]
    pub bb_std: f64,
    #[serde(default = "default_volume_ma")]
    pub volume_ma: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    #[serde(default = "default_max_drawdown_pct")]
    pub max_drawdown_pct: f64,
    #[serde(default = "default_stop_loss_pct")]
    pub stop_loss_pct: f64,
    #[serde(default = "default_take_profit_pct")]
    pub take_profit_pct: f64,
    #[serde(default = "default_risk_per_trade_pct")]
    pub risk_per_trade_pct: f64,
    #[serde(default = "default_max_daily_loss_pct")]
    pub max_daily_loss_pct: f64,
    #[serde(default = "default_trailing_stop_pct")]
    pub trailing_stop_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    #[serde(default = "default_start_date")]
    pub start_date: String,
    #[serde(default = "default_end_date")]
    pub end_date: String,
    #[serde(default = "default_commission")]
    pub commission: f64,
    #[serde(default = "default_slippage")]
    pub slippage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    #[serde(default = "default_check_interval")]
    pub check_interval: u64,
    #[serde(default = "default_order_type")]
    pub order_type: String,
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: usize,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_dir")]
    pub log_dir: String,
    #[serde(default = "default_log_to_file")]
    pub log_to_file: bool,
    #[serde(default = "default_log_to_console")]
    pub log_to_console: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsConfig {
    #[serde(default = "default_news_enabled")]
    pub enabled: bool,
    #[serde(default = "default_news_cache_ttl")]
    pub cache_ttl_seconds: i64,
    #[serde(default = "default_news_lookback_hours")]
    pub lookback_hours: i64,
    #[serde(default = "default_news_sentiment_threshold")]
    pub sentiment_threshold: f64,
    #[serde(default = "default_news_weight")]
    pub sentiment_weight: f64,
    #[serde(default = "default_news_limit")]
    pub article_limit: usize,
}

// Default value functions
fn default_symbol_alt() -> String { "XAUUSD".to_string() }
fn default_initial_capital() -> f64 { 100000.0 }
fn default_position_size() -> f64 { 0.1 }
fn default_max_positions() -> usize { 3 }
fn default_trading_mode() -> TradingMode { TradingMode::Paper }
fn default_provider() -> String { "yfinance".to_string() }
fn default_interval() -> String { "1h".to_string() }
fn default_lookback_days() -> usize { 365 }
fn default_cache_enabled() -> bool { true }
fn default_cache_dir() -> String { "data/raw".to_string() }
fn default_strategy_name() -> String { "GoldMomentumStrategy".to_string() }
fn default_fast_ma() -> usize { 20 }
fn default_slow_ma() -> usize { 50 }
fn default_rsi_period() -> usize { 14 }
fn default_rsi_oversold() -> f64 { 30.0 }
fn default_rsi_overbought() -> f64 { 70.0 }
fn default_bb_period() -> usize { 20 }
fn default_bb_std() -> f64 { 2.0 }
fn default_volume_ma() -> usize { 20 }
fn default_max_drawdown_pct() -> f64 { 15.0 }
fn default_stop_loss_pct() -> f64 { 2.0 }
fn default_take_profit_pct() -> f64 { 5.0 }
fn default_risk_per_trade_pct() -> f64 { 1.0 }
fn default_max_daily_loss_pct() -> f64 { 5.0 }
fn default_trailing_stop_pct() -> f64 { 1.5 }
fn default_start_date() -> String { "2020-01-01".to_string() }
fn default_end_date() -> String { "2024-12-31".to_string() }
fn default_commission() -> f64 { 0.001 }
fn default_slippage() -> f64 { 0.0005 }
fn default_check_interval() -> u64 { 60 }
fn default_order_type() -> String { "market".to_string() }
fn default_retry_attempts() -> usize { 3 }
fn default_timeout() -> u64 { 30 }
fn default_log_level() -> String { "INFO".to_string() }
fn default_log_dir() -> String { "logs".to_string() }
fn default_log_to_file() -> bool { true }
fn default_log_to_console() -> bool { true }
fn default_news_enabled() -> bool { true }
fn default_news_cache_ttl() -> i64 { 300 } // 5 minutes
fn default_news_lookback_hours() -> i64 { 24 } // 24 hours
fn default_news_sentiment_threshold() -> f64 { 0.05 } // -0.05 to 0.05 is neutral
fn default_news_weight() -> f64 { 0.3 } // 30% weight in signal
fn default_news_limit() -> usize { 20 } // Fetch up to 20 articles

impl Config {
    /// Load configuration from a YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        let config: Config = serde_yaml::from_str(&contents)
            .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    /// Load configuration with environment variable overrides
    pub fn load() -> Result<Self> {
        dotenv::dotenv().ok(); // Load .env file if it exists

        // Try to load from config file
        let config_path = std::env::var("CONFIG_PATH")
            .unwrap_or_else(|_| "config/config.yaml".to_string());

        Self::from_file(config_path)
    }

    /// Get the data interval as an enum
    pub fn get_interval(&self) -> Result<Interval> {
        self.data.interval.parse()
            .map_err(|e: String| Error::Config(e))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trading: TradingConfig {
                symbol: "GC=F".to_string(),
                symbol_alt: default_symbol_alt(),
                initial_capital: default_initial_capital(),
                position_size: default_position_size(),
                max_positions: default_max_positions(),
                trading_mode: default_trading_mode(),
            },
            data: DataConfig {
                provider: default_provider(),
                interval: default_interval(),
                lookback_days: default_lookback_days(),
                cache_enabled: default_cache_enabled(),
                cache_dir: default_cache_dir(),
            },
            strategy: StrategyConfig {
                name: default_strategy_name(),
                fast_ma: default_fast_ma(),
                slow_ma: default_slow_ma(),
                rsi_period: default_rsi_period(),
                rsi_oversold: default_rsi_oversold(),
                rsi_overbought: default_rsi_overbought(),
                bb_period: default_bb_period(),
                bb_std: default_bb_std(),
                volume_ma: default_volume_ma(),
            },
            risk: RiskConfig {
                max_drawdown_pct: default_max_drawdown_pct(),
                stop_loss_pct: default_stop_loss_pct(),
                take_profit_pct: default_take_profit_pct(),
                risk_per_trade_pct: default_risk_per_trade_pct(),
                max_daily_loss_pct: default_max_daily_loss_pct(),
                trailing_stop_pct: default_trailing_stop_pct(),
            },
            backtest: BacktestConfig {
                start_date: default_start_date(),
                end_date: default_end_date(),
                commission: default_commission(),
                slippage: default_slippage(),
            },
            execution: ExecutionConfig {
                check_interval: default_check_interval(),
                order_type: default_order_type(),
                retry_attempts: default_retry_attempts(),
                timeout: default_timeout(),
            },
            logging: LoggingConfig {
                level: default_log_level(),
                log_dir: default_log_dir(),
                log_to_file: default_log_to_file(),
                log_to_console: default_log_to_console(),
            },
            news: NewsConfig {
                enabled: default_news_enabled(),
                cache_ttl_seconds: default_news_cache_ttl(),
                lookback_hours: default_news_lookback_hours(),
                sentiment_threshold: default_news_sentiment_threshold(),
                sentiment_weight: default_news_weight(),
                article_limit: default_news_limit(),
            },
        }
    }
}

impl Default for NewsConfig {
    fn default() -> Self {
        Self {
            enabled: default_news_enabled(),
            cache_ttl_seconds: default_news_cache_ttl(),
            lookback_hours: default_news_lookback_hours(),
            sentiment_threshold: default_news_sentiment_threshold(),
            sentiment_weight: default_news_weight(),
            article_limit: default_news_limit(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.trading.symbol, "GC=F");
        assert_eq!(config.trading.initial_capital, 100000.0);
        assert_eq!(config.strategy.fast_ma, 20);
        assert_eq!(config.strategy.slow_ma, 50);
    }
}
