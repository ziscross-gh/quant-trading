//! ATR Volatility Breakout Strategy for Gold/USD
//!
//! Exploits volatility expansion - a proven Gold trading edge.
//! Gold tends to have explosive moves after periods of low volatility.
//!
//! Research basis:
//! - Gold volatility clustering (low vol → high vol transitions)
//! - Breakouts work best when volatility expands
//! - ATR filter prevents false signals in choppy markets

use super::{IndicatorData, Strategy};
use crate::config::Config;
use crate::indicators::{ema, rsi};
use crate::{Error, MarketData, Result, Signal};
use tracing::{debug, info};

/// ATR Volatility Breakout Strategy
///
/// Entry Rules:
/// 1. Measure ATR (Average True Range) for volatility
/// 2. Calculate Bollinger-style bands using ATR
/// 3. BUY when: Price breaks above upper ATR band + expanding volatility
/// 4. SELL when: Price breaks below lower ATR band + expanding volatility
///
/// This captures explosive Gold moves after consolidation periods.
pub struct VolatilityBreakoutStrategy {
    config: Config,
    atr_period: usize,
    atr_multiplier: f64,
    volatility_lookback: usize,
    min_volatility_expansion: f64, // Minimum % increase in ATR to trigger
}

impl VolatilityBreakoutStrategy {
    pub fn new(config: Config) -> Self {
        let atr_period = 14;
        let atr_multiplier = 2.0;
        let volatility_lookback = 20;
        let min_volatility_expansion = 1.2; // ATR must be 1.2x its MA

        info!("Initializing ATR Volatility Breakout Strategy");
        info!("  ATR Period: {}", atr_period);
        info!("  ATR Multiplier: {}x", atr_multiplier);
        info!("  Volatility Expansion Threshold: {}x", min_volatility_expansion);

        Self {
            config,
            atr_period,
            atr_multiplier,
            volatility_lookback,
            min_volatility_expansion,
        }
    }

    /// Calculate ATR (Average True Range)
    fn calculate_atr(&self, data: &MarketData) -> Vec<f64> {
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

        // EMA of true ranges (more responsive than SMA)
        let mut atr = Vec::new();
        if true_ranges.len() >= self.atr_period {
            // Initial SMA
            let initial_sum: f64 = true_ranges[..self.atr_period].iter().sum();
            let mut current_atr = initial_sum / self.atr_period as f64;
            atr.push(current_atr);

            // EMA smoothing
            let multiplier = 2.0 / (self.atr_period as f64 + 1.0);
            for i in self.atr_period..true_ranges.len() {
                current_atr = (true_ranges[i] - current_atr) * multiplier + current_atr;
                atr.push(current_atr);
            }
        }

        // Pad beginning
        let padding = data.len() - atr.len();
        let mut result = vec![0.0; padding];
        result.extend(atr);
        result
    }

    /// Check if volatility is expanding
    fn is_volatility_expanding(&self, atr_values: &[f64]) -> bool {
        if atr_values.len() < self.volatility_lookback + 1 {
            return false;
        }

        let current_atr = *atr_values.last().unwrap();
        let lookback_start = atr_values.len() - self.volatility_lookback;
        let avg_atr: f64 = atr_values[lookback_start..atr_values.len() - 1]
            .iter()
            .sum::<f64>()
            / (self.volatility_lookback - 1) as f64;

        if avg_atr == 0.0 {
            return false;
        }

        let expansion_ratio = current_atr / avg_atr;
        expansion_ratio >= self.min_volatility_expansion
    }

    /// Calculate Keltner-style channels using ATR
    fn calculate_atr_bands(&self, closes: &[f64], atr: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let ema_values = ema(closes, 20).unwrap_or_else(|_| vec![0.0; closes.len()]);

        let mut upper = Vec::new();
        let mut lower = Vec::new();

        for i in 0..closes.len() {
            upper.push(ema_values[i] + atr[i] * self.atr_multiplier);
            lower.push(ema_values[i] - atr[i] * self.atr_multiplier);
        }

        (upper, ema_values, lower)
    }
}

#[async_trait::async_trait]
impl Strategy for VolatilityBreakoutStrategy {
    fn name(&self) -> &str {
        "ATR Volatility Breakout Strategy"
    }

    fn calculate_indicators(&self, data: &MarketData) -> Result<IndicatorData> {
        let closes: Vec<f64> = data.candles.iter().map(|c| c.close).collect();
        let volumes: Vec<f64> = data.candles.iter().map(|c| c.volume).collect();

        if closes.len() < self.atr_period + self.volatility_lookback {
            return Err(Error::InvalidData(format!(
                "Need {} candles, have {}",
                self.atr_period + self.volatility_lookback,
                closes.len()
            )));
        }

        // Calculate ATR
        let atr_values = self.calculate_atr(data);

        // Calculate ATR-based bands (Keltner Channels)
        let (upper_band, middle_band, lower_band) = self.calculate_atr_bands(&closes, &atr_values);

        // Calculate RSI for confluence
        let rsi_values = rsi(&closes, 14)?;

        // Calculate momentum (rate of ATR change)
        let mut momentum = Vec::new();
        for i in 0..atr_values.len() {
            if i >= 10 && atr_values[i - 10] > 0.0 {
                let atr_roc = ((atr_values[i] - atr_values[i - 10]) / atr_values[i - 10]) * 100.0;
                momentum.push(atr_roc);
            } else {
                momentum.push(0.0);
            }
        }

        Ok(IndicatorData {
            ma_fast: middle_band.clone(),
            ma_slow: middle_band,
            rsi: rsi_values,
            bb_upper: upper_band,
            bb_middle: atr_values.clone(), // Store ATR in bb_middle
            bb_lower: lower_band,
            volume_ma: volumes,
            momentum,
        })
    }

    fn generate_signal(&self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
        let last_candle = data.last()
            .ok_or_else(|| Error::InvalidData("No data".to_string()))?;

        let values = indicators.last_values()
            .ok_or_else(|| Error::InvalidData("No indicators".to_string()))?;

        let current_atr = values.bb_middle; // We stored ATR here
        let is_expanding = self.is_volatility_expanding(&indicators.bb_middle);

        debug!(
            "Volatility Breakout - Price: {:.2}, Upper: {:.2}, Lower: {:.2}, ATR: {:.2}, Expanding: {}",
            last_candle.close, values.bb_upper, values.bb_lower, current_atr, is_expanding
        );

        // BUY Signal: Price breaks above upper band + volatility expanding
        let price_above_upper = last_candle.close > values.bb_upper;
        let strong_breakout = last_candle.high > values.bb_upper * 1.001; // 0.1% clearance
        let not_overbought = values.rsi < 75.0; // Some room to run

        if price_above_upper && is_expanding && strong_breakout && not_overbought {
            info!(
                "🟢 BUY: Volatility breakout! Price: {:.2} > Upper: {:.2}, ATR expanding: {:.2}%",
                last_candle.close, values.bb_upper, values.momentum
            );
            return Ok(Signal::Buy);
        }

        // SELL Signal: Price breaks below lower band + volatility expanding
        let price_below_lower = last_candle.close < values.bb_lower;
        let strong_breakdown = last_candle.low < values.bb_lower * 0.999; // 0.1% clearance
        let not_oversold = values.rsi > 25.0; // Some room to fall

        if price_below_lower && is_expanding && strong_breakdown && not_oversold {
            info!(
                "🔴 SELL: Volatility breakdown! Price: {:.2} < Lower: {:.2}, ATR expanding: {:.2}%",
                last_candle.close, values.bb_lower, values.momentum
            );
            return Ok(Signal::Sell);
        }

        // HOLD: No volatility breakout or consolidating
        if is_expanding {
            debug!("⚪ HOLD: Volatility expanding but no clear breakout");
        } else {
            debug!("⚪ HOLD: Low volatility consolidation (ATR: {:.2})", current_atr);
        }

        Ok(Signal::Hold)
    }

    fn reset(&mut self) {
        debug!("Resetting ATR Volatility Breakout Strategy");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Candle;
    use chrono::Utc;

    #[test]
    fn test_volatility_strategy() {
        let config = Config::default();
        let strategy = VolatilityBreakoutStrategy::new(config);
        assert_eq!(strategy.name(), "ATR Volatility Breakout Strategy");
    }

    #[test]
    fn test_atr_calculation() {
        let config = Config::default();
        let strategy = VolatilityBreakoutStrategy::new(config);

        let mut data = Vec::new();
        for i in 0..50 {
            data.push(Candle {
                timestamp: Utc::now(),
                open: 2000.0,
                high: 2020.0 + (i as f64 % 10.0),
                low: 1980.0,
                close: 2000.0,
                volume: 1000.0,
            });
        }

        let atr = strategy.calculate_atr(&data);
        assert_eq!(atr.len(), data.len());
        assert!(atr.iter().skip(20).all(|&x| x > 0.0));
    }
}
