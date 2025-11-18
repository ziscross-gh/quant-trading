//! Trading strategies module

use crate::{Error, MarketData, Result, Signal, SignalResult};
use async_trait::async_trait;
use chrono::Utc;

pub mod gold_momentum;
pub mod news_enhanced;

pub use gold_momentum::GoldMomentumStrategy;
pub use news_enhanced::NewsEnhancedStrategy;

/// Base trait for trading strategies
#[async_trait]
pub trait Strategy: Send + Sync {
    /// Get strategy name
    fn name(&self) -> &str;

    /// Calculate technical indicators
    fn calculate_indicators(&self, data: &MarketData) -> Result<IndicatorData>;

    /// Generate trading signal
    fn generate_signal(&self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal>;

    /// Run complete strategy pipeline
    fn run(&self, data: &MarketData) -> Result<SignalResult> {
        let indicators = self.calculate_indicators(data)?;
        let signal = self.generate_signal(data, indicators)?;

        let last_candle = data.last()
            .ok_or_else(|| Error::InvalidData("No data available".to_string()))?;

        Ok(SignalResult {
            signal,
            timestamp: last_candle.timestamp,
            price: last_candle.close,
            strategy: self.name().to_string(),
        })
    }

    /// Reset strategy state
    fn reset(&mut self);

    /// Execute strategy with news sentiment (async version)
    /// Default implementation just calls generate_signal
    async fn execute(&mut self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
        self.generate_signal(data, indicators)
    }

    /// Generate signal with news enhancement (for news-aware strategies)
    /// Default implementation just calls generate_signal
    async fn generate_signal_with_news(&mut self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
        self.generate_signal(data, indicators)
    }
}

/// Container for calculated indicators
#[derive(Debug, Clone)]
pub struct IndicatorData {
    pub ma_fast: Vec<f64>,
    pub ma_slow: Vec<f64>,
    pub rsi: Vec<f64>,
    pub bb_upper: Vec<f64>,
    pub bb_middle: Vec<f64>,
    pub bb_lower: Vec<f64>,
    pub volume_ma: Vec<f64>,
    pub momentum: Vec<f64>,
}

impl IndicatorData {
    /// Get the last value for each indicator
    pub fn last_values(&self) -> Option<LastIndicatorValues> {
        Some(LastIndicatorValues {
            ma_fast: *self.ma_fast.last()?,
            ma_slow: *self.ma_slow.last()?,
            rsi: *self.rsi.last()?,
            bb_upper: *self.bb_upper.last()?,
            bb_middle: *self.bb_middle.last()?,
            bb_lower: *self.bb_lower.last()?,
            volume_ma: *self.volume_ma.last()?,
            momentum: *self.momentum.last()?,
        })
    }

    /// Get values at a specific index
    pub fn at(&self, index: usize) -> Option<LastIndicatorValues> {
        if index >= self.ma_fast.len() {
            return None;
        }

        Some(LastIndicatorValues {
            ma_fast: self.ma_fast[index],
            ma_slow: self.ma_slow[index],
            rsi: self.rsi[index],
            bb_upper: self.bb_upper[index],
            bb_middle: self.bb_middle[index],
            bb_lower: self.bb_lower[index],
            volume_ma: self.volume_ma[index],
            momentum: self.momentum[index],
        })
    }
}

/// Last indicator values snapshot
#[derive(Debug, Clone)]
pub struct LastIndicatorValues {
    pub ma_fast: f64,
    pub ma_slow: f64,
    pub rsi: f64,
    pub bb_upper: f64,
    pub bb_middle: f64,
    pub bb_lower: f64,
    pub volume_ma: f64,
    pub momentum: f64,
}
