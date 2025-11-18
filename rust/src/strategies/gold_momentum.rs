//! Gold/USD Momentum Strategy

use super::{IndicatorData, Strategy};
use crate::config::StrategyConfig;
use crate::indicators::*;
use crate::{Error, MarketData, Result, Signal};
use async_trait::async_trait;
use tracing::{debug, info, warn};

/// Gold/USD momentum-based trading strategy
pub struct GoldMomentumStrategy {
    name: String,
    config: StrategyConfig,
    position: Option<Signal>,
}

impl GoldMomentumStrategy {
    /// Create a new Gold momentum strategy
    pub fn new(config: StrategyConfig) -> Self {
        info!(
            "GoldMomentumStrategy initialized: MA({}/{}), RSI({}), BB({},{})",
            config.fast_ma, config.slow_ma, config.rsi_period, config.bb_period, config.bb_std
        );

        Self {
            name: config.name.clone(),
            config,
            position: None,
        }
    }
}

#[async_trait]
impl Strategy for GoldMomentumStrategy {
    fn name(&self) -> &str {
        &self.name
    }

    fn calculate_indicators(&self, data: &MarketData) -> Result<IndicatorData> {
        if data.is_empty() {
            return Err(Error::InvalidData("No market data provided".to_string()));
        }

        let closes = data.closes();
        let volumes = data.volumes();

        // Moving Averages
        let ma_fast = sma(&closes, self.config.fast_ma)?;
        let ma_slow = sma(&closes, self.config.slow_ma)?;

        // RSI
        let rsi = rsi(&closes, self.config.rsi_period)?;

        // Bollinger Bands
        let bb = bollinger_bands(&closes, self.config.bb_period, self.config.bb_std)?;

        // Volume MA
        let volume_ma = sma(&volumes, self.config.volume_ma)?;

        // Momentum
        let momentum = momentum(&closes, 10);

        Ok(IndicatorData {
            ma_fast,
            ma_slow,
            rsi,
            bb_upper: bb.upper,
            bb_middle: bb.middle,
            bb_lower: bb.lower,
            volume_ma,
            momentum,
        })
    }

    fn generate_signal(&self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
        let len = data.len();
        if len < self.config.slow_ma {
            warn!("Insufficient data for signal generation");
            return Ok(Signal::Hold);
        }

        // Get latest values
        let latest = indicators.last_values()
            .ok_or_else(|| Error::Strategy("Failed to get latest indicator values".to_string()))?;

        let prev = indicators.at(len - 2)
            .ok_or_else(|| Error::Strategy("Failed to get previous indicator values".to_string()))?;

        let last_candle = data.last().unwrap();
        let prev_candle = &data.candles[len - 2];

        // Check for NaN values
        if latest.ma_fast.is_nan() || latest.ma_slow.is_nan() || latest.rsi.is_nan() {
            warn!("NaN values in indicators, skipping signal");
            return Ok(Signal::Hold);
        }

        // Initialize signal counters
        let mut buy_signals = 0;
        let mut sell_signals = 0;

        // 1. Moving Average Crossover
        if latest.ma_fast > latest.ma_slow && prev.ma_fast <= prev.ma_slow {
            buy_signals += 2;
            debug!("Buy signal: MA crossover (fast > slow)");
        } else if latest.ma_fast < latest.ma_slow && prev.ma_fast >= prev.ma_slow {
            sell_signals += 2;
            debug!("Sell signal: MA crossover (fast < slow)");
        }

        // 2. RSI Conditions
        if latest.rsi < self.config.rsi_oversold {
            buy_signals += 1;
            debug!("Buy signal: RSI oversold ({:.2})", latest.rsi);
        } else if latest.rsi > self.config.rsi_overbought {
            sell_signals += 1;
            debug!("Sell signal: RSI overbought ({:.2})", latest.rsi);
        }

        // 3. Bollinger Bands
        if last_candle.close < latest.bb_lower && last_candle.close > prev_candle.close {
            buy_signals += 1;
            debug!("Buy signal: Price bouncing from lower BB");
        } else if last_candle.close > latest.bb_upper && last_candle.close < prev_candle.close {
            sell_signals += 1;
            debug!("Sell signal: Price rejecting at upper BB");
        }

        // 4. Volume Confirmation
        let volume_ratio = if latest.volume_ma > 0.0 {
            last_candle.volume / latest.volume_ma
        } else {
            1.0
        };

        if volume_ratio > 1.5 {
            if buy_signals > sell_signals {
                buy_signals += 1;
                debug!("Volume confirms buy signal");
            } else if sell_signals > buy_signals {
                sell_signals += 1;
                debug!("Volume confirms sell signal");
            }
        }

        // 5. Trend Strength
        let trend_strength = if latest.ma_slow > 0.0 {
            ((latest.ma_fast - latest.ma_slow) / latest.ma_slow) * 100.0
        } else {
            0.0
        };

        if trend_strength.abs() > 2.0 {
            if trend_strength > 0.0 {
                buy_signals += 1;
            } else {
                sell_signals += 1;
            }
        }

        // Decision logic
        let signal = if buy_signals >= 3 && buy_signals > sell_signals {
            info!(
                "BUY signal generated (buy: {}, sell: {})",
                buy_signals, sell_signals
            );
            Signal::Buy
        } else if sell_signals >= 3 && sell_signals > buy_signals {
            info!(
                "SELL signal generated (buy: {}, sell: {})",
                buy_signals, sell_signals
            );
            Signal::Sell
        } else {
            debug!(
                "HOLD signal (buy: {}, sell: {})",
                buy_signals, sell_signals
            );
            Signal::Hold
        };

        Ok(signal)
    }

    fn reset(&mut self) {
        self.position = None;
        info!("Strategy reset");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Candle;
    use chrono::Utc;

    fn create_test_data() -> MarketData {
        let mut candles = Vec::new();
        let base_price = 2000.0;

        for i in 0..100 {
            candles.push(Candle {
                timestamp: Utc::now(),
                open: base_price + i as f64,
                high: base_price + i as f64 + 10.0,
                low: base_price + i as f64 - 10.0,
                close: base_price + i as f64 + 5.0,
                volume: 100000.0,
            });
        }

        MarketData {
            symbol: "GC=F".to_string(),
            candles,
        }
    }

    #[test]
    fn test_strategy_creation() {
        let config = StrategyConfig {
            name: "Test".to_string(),
            fast_ma: 20,
            slow_ma: 50,
            rsi_period: 14,
            rsi_oversold: 30.0,
            rsi_overbought: 70.0,
            bb_period: 20,
            bb_std: 2.0,
            volume_ma: 20,
        };

        let strategy = GoldMomentumStrategy::new(config);
        assert_eq!(strategy.name(), "Test");
    }

    #[test]
    fn test_indicator_calculation() {
        let config = StrategyConfig {
            name: "Test".to_string(),
            fast_ma: 20,
            slow_ma: 50,
            rsi_period: 14,
            rsi_oversold: 30.0,
            rsi_overbought: 70.0,
            bb_period: 20,
            bb_std: 2.0,
            volume_ma: 20,
        };

        let strategy = GoldMomentumStrategy::new(config);
        let data = create_test_data();
        let indicators = strategy.calculate_indicators(&data).unwrap();

        assert_eq!(indicators.ma_fast.len(), data.len());
        assert_eq!(indicators.ma_slow.len(), data.len());
    }
}
