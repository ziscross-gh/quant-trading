//! Breakout Strategy for Gold/USD
//!
//! Trades breakouts above resistance or below support levels.
//! Uses volume confirmation and ATR for volatility filtering.

use super::{IndicatorData, Strategy};
use crate::config::Config;
use crate::indicators::{rsi, sma};
use crate::{Error, MarketData, Result, Signal};
use tracing::{debug, info};

/// Breakout Strategy
///
/// Strategy Logic:
/// - Identifies support/resistance from recent highs/lows
/// - BUY when: Price breaks above resistance + high volume
/// - SELL when: Price breaks below support + high volume
/// - Works best in trending markets with clear breakouts
pub struct BreakoutStrategy {
    config: Config,
    lookback_period: usize,
    breakout_threshold: f64, // % above/below to confirm breakout
    volume_multiplier: f64,   // Volume must be X times average
}

impl BreakoutStrategy {
    /// Create new breakout strategy
    pub fn new(config: Config) -> Self {
        let lookback_period = config.strategy.bb_period; // Reuse BB period for lookback
        let breakout_threshold = 0.5; // 0.5% breakout threshold
        let volume_multiplier = 1.5;   // 1.5x average volume

        info!("Initializing Breakout Strategy");
        info!("  Lookback Period: {}", lookback_period);
        info!("  Breakout Threshold: {}%", breakout_threshold);
        info!("  Volume Multiplier: {}x", volume_multiplier);

        Self {
            config,
            lookback_period,
            breakout_threshold,
            volume_multiplier,
        }
    }

    /// Calculate Average True Range (ATR) for volatility
    fn calculate_atr(&self, data: &MarketData, period: usize) -> Vec<f64> {
        let mut atr = Vec::new();
        let mut true_ranges = Vec::new();

        for i in 1..data.len() {
            let high = data.candles[i].high;
            let low = data.candles[i].low;
            let prev_close = data.candles[i - 1].close;

            let tr = (high - low)
                .max((high - prev_close).abs())
                .max((low - prev_close).abs());

            true_ranges.push(tr);
        }

        // Calculate SMA of true ranges
        for i in period..=true_ranges.len() {
            let sum: f64 = true_ranges[i - period..i].iter().sum();
            atr.push(sum / period as f64);
        }

        // Pad beginning with zeros
        let padding = data.len() - atr.len();
        let mut result = vec![0.0; padding];
        result.extend(atr);
        result
    }

    /// Find resistance level (recent high)
    fn find_resistance(&self, data: &MarketData) -> f64 {
        let start_idx = data.len().saturating_sub(self.lookback_period);
        data.candles[start_idx..]
            .iter()
            .map(|c| c.high)
            .fold(0.0, f64::max)
    }

    /// Find support level (recent low)
    fn find_support(&self, data: &MarketData) -> f64 {
        let start_idx = data.len().saturating_sub(self.lookback_period);
        data.candles[start_idx..]
            .iter()
            .map(|c| c.low)
            .fold(f64::INFINITY, f64::min)
    }
}

#[async_trait::async_trait]
impl Strategy for BreakoutStrategy {
    fn name(&self) -> &str {
        "Breakout Strategy"
    }

    fn calculate_indicators(&self, data: &MarketData) -> Result<IndicatorData> {
        let closes: Vec<f64> = data.candles.iter().map(|c| c.close).collect();
        let volumes: Vec<f64> = data.candles.iter().map(|c| c.volume).collect();

        if closes.len() < self.lookback_period {
            return Err(Error::InvalidData(format!(
                "Insufficient data: need at least {} candles",
                self.lookback_period
            )));
        }

        // Calculate volume MA
        let volume_ma = sma(&volumes, self.config.strategy.volume_ma)?;

        // Calculate ATR for volatility
        let atr = self.calculate_atr(data, 14);

        // Calculate RSI for overbought/oversold confirmation
        let rsi_values = rsi(&closes, self.config.strategy.rsi_period)?;

        // Calculate price momentum (rate of change)
        let mut momentum = Vec::new();
        for i in 0..closes.len() {
            if i >= 10 && closes[i - 10] != 0.0 {
                let roc = ((closes[i] - closes[i - 10]) / closes[i - 10]) * 100.0;
                momentum.push(roc);
            } else {
                momentum.push(0.0);
            }
        }

        // Use ATR as upper/lower bounds
        let mut bb_upper = Vec::new();
        let mut bb_middle = closes.clone();
        let mut bb_lower = Vec::new();

        for i in 0..closes.len() {
            let current_atr = if i < atr.len() { atr[i] } else { 0.0 };
            bb_upper.push(closes[i] + current_atr * 2.0);
            bb_lower.push(closes[i] - current_atr * 2.0);
        }

        Ok(IndicatorData {
            ma_fast: closes.clone(),
            ma_slow: closes,
            rsi: rsi_values,
            bb_upper,
            bb_middle,
            bb_lower,
            volume_ma,
            momentum,
        })
    }

    fn generate_signal(&self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
        let last_candle = data.last()
            .ok_or_else(|| Error::InvalidData("No data available".to_string()))?;

        let last_close = last_candle.close;
        let last_high = last_candle.high;
        let last_low = last_candle.low;
        let last_volume = last_candle.volume;

        let values = indicators.last_values()
            .ok_or_else(|| Error::InvalidData("No indicator values".to_string()))?;

        // Find support and resistance
        let resistance = self.find_resistance(data);
        let support = self.find_support(data);
        let range = resistance - support;

        // Check volume confirmation
        let volume_confirmed = last_volume > values.volume_ma * self.volume_multiplier;

        debug!(
            "Breakout - Price: {:.2}, Support: {:.2}, Resistance: {:.2}, Volume: {:.0} (Avg: {:.0}), Momentum: {:.2}%",
            last_close, support, resistance, last_volume, values.volume_ma, values.momentum
        );

        // BUY Signal: Breakout above resistance
        let breakout_price = resistance * (1.0 + self.breakout_threshold / 100.0);
        let upward_breakout = last_high >= breakout_price;
        let strong_momentum = values.momentum > 1.0; // Positive momentum

        if upward_breakout && volume_confirmed && strong_momentum {
            info!(
                "🟢 BUY Signal: Breakout above resistance ({:.2}) with volume confirmation",
                resistance
            );
            return Ok(Signal::Buy);
        }

        // SELL Signal: Breakdown below support
        let breakdown_price = support * (1.0 - self.breakout_threshold / 100.0);
        let downward_breakout = last_low <= breakdown_price;
        let weak_momentum = values.momentum < -1.0; // Negative momentum

        if downward_breakout && volume_confirmed && weak_momentum {
            info!(
                "🔴 SELL Signal: Breakdown below support ({:.2}) with volume confirmation",
                support
            );
            return Ok(Signal::Sell);
        }

        // HOLD: Price within range or no volume confirmation
        debug!("⚪ HOLD: Price within range {:.2} - {:.2}", support, resistance);
        Ok(Signal::Hold)
    }

    fn reset(&mut self) {
        debug!("Resetting Breakout Strategy");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Candle;
    use chrono::Utc;

    #[test]
    fn test_breakout_strategy() {
        let config = Config::default();
        let strategy = BreakoutStrategy::new(config);
        assert_eq!(strategy.name(), "Breakout Strategy");
    }

    #[test]
    fn test_support_resistance() {
        let config = Config::default();
        let strategy = BreakoutStrategy::new(config);

        let mut data: Vec<Candle> = Vec::new();
        for i in 0..30 {
            data.push(Candle {
                timestamp: Utc::now(),
                open: 2000.0,
                high: 2010.0 + i as f64,
                low: 1990.0,
                close: 2000.0,
                volume: 1000.0,
            });
        }

        let resistance = strategy.find_resistance(&data);
        let support = strategy.find_support(&data);

        assert!(resistance > support);
        assert!(resistance >= 2010.0);
    }
}
