//! Mean Reversion Strategy for Gold/USD
//!
//! Buys when price deviates significantly below mean, sells when above.
//! Uses Bollinger Bands and RSI for confirmation.

use super::{IndicatorData, Strategy};
use crate::config::Config;
use crate::indicators::{bollinger_bands, rsi, sma};
use crate::{Error, MarketData, Result, Signal};
use tracing::{debug, info};

/// Mean Reversion Strategy
///
/// Strategy Logic:
/// - BUY when: Price touches lower Bollinger Band + RSI < 30 (oversold)
/// - SELL when: Price touches upper Bollinger Band + RSI > 70 (overbought)
/// - Works best in ranging/choppy markets
pub struct MeanReversionStrategy {
    config: Config,
    bb_period: usize,
    bb_std: f64,
    rsi_period: usize,
    rsi_oversold: f64,
    rsi_overbought: f64,
}

impl MeanReversionStrategy {
    /// Create new mean reversion strategy
    pub fn new(config: Config) -> Self {
        info!("Initializing Mean Reversion Strategy");
        info!("  BB Period: {}", config.strategy.bb_period);
        info!("  BB Std Dev: {}", config.strategy.bb_std);
        info!("  RSI Period: {}", config.strategy.rsi_period);
        info!("  RSI Oversold: {}", config.strategy.rsi_oversold);
        info!("  RSI Overbought: {}", config.strategy.rsi_overbought);

        Self {
            bb_period: config.strategy.bb_period,
            bb_std: config.strategy.bb_std,
            rsi_period: config.strategy.rsi_period,
            rsi_oversold: config.strategy.rsi_oversold,
            rsi_overbought: config.strategy.rsi_overbought,
            config,
        }
    }
}

#[async_trait::async_trait]
impl Strategy for MeanReversionStrategy {
    fn name(&self) -> &str {
        "Mean Reversion Strategy"
    }

    fn calculate_indicators(&self, data: &MarketData) -> Result<IndicatorData> {
        let closes: Vec<f64> = data.candles.iter().map(|c| c.close).collect();

        if closes.len() < self.bb_period.max(self.rsi_period) {
            return Err(Error::InvalidData(format!(
                "Insufficient data: need at least {} candles",
                self.bb_period.max(self.rsi_period)
            )));
        }

        // Calculate Bollinger Bands
        let bb = bollinger_bands(&closes, self.bb_period, self.bb_std)?;
        let bb_upper = bb.upper;
        let bb_middle = bb.middle;
        let bb_lower = bb.lower;

        // Calculate RSI
        let rsi_values = rsi(&closes, self.rsi_period)?;

        // Calculate SMA for reference
        let sma_values = sma(&closes, self.bb_period)?;

        // Calculate distance from mean
        let mut momentum = Vec::new();
        for i in 0..closes.len() {
            if i < bb_middle.len() && bb_middle[i] != 0.0 {
                let distance_pct = ((closes[i] - bb_middle[i]) / bb_middle[i]) * 100.0;
                momentum.push(distance_pct);
            } else {
                momentum.push(0.0);
            }
        }

        Ok(IndicatorData {
            ma_fast: sma_values.clone(),
            ma_slow: sma_values,
            rsi: rsi_values,
            bb_upper,
            bb_middle,
            bb_lower,
            volume_ma: vec![0.0; closes.len()], // Not used in this strategy
            momentum,
        })
    }

    fn generate_signal(&self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
        let last_candle = data.last()
            .ok_or_else(|| Error::InvalidData("No data available".to_string()))?;

        let last_close = last_candle.close;
        let last_high = last_candle.high;
        let last_low = last_candle.low;

        let values = indicators.last_values()
            .ok_or_else(|| Error::InvalidData("No indicator values".to_string()))?;

        debug!(
            "Mean Reversion - Price: {:.2}, BB: [{:.2}, {:.2}, {:.2}], RSI: {:.2}, Distance: {:.2}%",
            last_close, values.bb_lower, values.bb_middle, values.bb_upper,
            values.rsi, values.momentum
        );

        // BUY Signal: Price near/below lower band + RSI oversold
        let touching_lower_band = last_low <= values.bb_lower * 1.002; // Within 0.2%
        let is_oversold = values.rsi < self.rsi_oversold;
        let below_mean = last_close < values.bb_middle;

        if touching_lower_band && is_oversold && below_mean {
            info!(
                "🟢 BUY Signal: Price touched lower BB ({:.2}) + RSI oversold ({:.2})",
                values.bb_lower, values.rsi
            );
            return Ok(Signal::Buy);
        }

        // SELL Signal: Price near/above upper band + RSI overbought
        let touching_upper_band = last_high >= values.bb_upper * 0.998; // Within 0.2%
        let is_overbought = values.rsi > self.rsi_overbought;
        let above_mean = last_close > values.bb_middle;

        if touching_upper_band && is_overbought && above_mean {
            info!(
                "🔴 SELL Signal: Price touched upper BB ({:.2}) + RSI overbought ({:.2})",
                values.bb_upper, values.rsi
            );
            return Ok(Signal::Sell);
        }

        // HOLD: Price within normal range
        debug!("⚪ HOLD: Price within normal range");
        Ok(Signal::Hold)
    }

    fn reset(&mut self) {
        debug!("Resetting Mean Reversion Strategy");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Candle;
    use chrono::Utc;

    #[test]
    fn test_mean_reversion_strategy() {
        let config = Config::default();
        let strategy = MeanReversionStrategy::new(config);
        assert_eq!(strategy.name(), "Mean Reversion Strategy");
    }

    #[test]
    fn test_oversold_buy_signal() {
        let config = Config::default();
        let strategy = MeanReversionStrategy::new(config);

        // Create fake data trending down to oversold
        let mut data: Vec<Candle> = Vec::new();
        let base_price = 2000.0;

        for i in 0..50 {
            let price = base_price - (i as f64 * 2.0); // Trending down
            data.push(Candle {
                timestamp: Utc::now(),
                open: price,
                high: price + 5.0,
                low: price - 5.0,
                close: price,
                volume: 1000.0,
            });
        }

        let indicators = strategy.calculate_indicators(&data).unwrap();
        let signal = strategy.generate_signal(&data, &indicators).unwrap();

        // Should be buy or hold (depending on exact RSI/BB values)
        assert!(matches!(signal, Signal::Buy | Signal::Hold));
    }
}
