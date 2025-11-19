//! Market Regime Detection Module
//!
//! Classifies market conditions into distinct regimes to enable
//! regime-aware strategy selection and position sizing.

use crate::{indicators, MarketData, Result};
use serde::{Deserialize, Serialize};

/// Market regime classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketRegime {
    /// Strong uptrend or downtrend (ADX > 25, clear direction)
    StrongTrend,
    /// Weak trend (ADX 20-25, some direction)
    WeakTrend,
    /// Ranging/choppy market (ADX < 20, no clear direction)
    Ranging,
    /// High volatility environment (ATR > historical average)
    HighVolatility,
    /// Low volatility environment (ATR < historical average)
    LowVolatility,
}

impl MarketRegime {
    pub fn description(&self) -> &'static str {
        match self {
            MarketRegime::StrongTrend => "Strong Trend",
            MarketRegime::WeakTrend => "Weak Trend",
            MarketRegime::Ranging => "Ranging/Choppy",
            MarketRegime::HighVolatility => "High Volatility",
            MarketRegime::LowVolatility => "Low Volatility",
        }
    }

    /// Get recommended strategies for this regime
    pub fn recommended_strategies(&self) -> Vec<&'static str> {
        match self {
            MarketRegime::StrongTrend => vec!["Momentum", "Trend Following", "Multi-Timeframe"],
            MarketRegime::WeakTrend => vec!["Multi-Timeframe", "Volatility Breakout"],
            MarketRegime::Ranging => vec!["Mean Reversion", "Bollinger Bands"],
            MarketRegime::HighVolatility => vec!["Volatility Breakout", "Wide Stops"],
            MarketRegime::LowVolatility => vec!["Breakout", "Tight Stops"],
        }
    }

    /// Get recommended position sizing multiplier (0.5 - 1.5)
    pub fn position_size_multiplier(&self) -> f64 {
        match self {
            MarketRegime::StrongTrend => 1.2,      // Increase size in strong trends
            MarketRegime::WeakTrend => 1.0,        // Normal size
            MarketRegime::Ranging => 0.8,          // Reduce size in choppy markets
            MarketRegime::HighVolatility => 0.6,   // Significantly reduce in high vol
            MarketRegime::LowVolatility => 1.1,    // Slightly increase in low vol
        }
    }
}

/// Configuration for regime detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeConfig {
    /// ADX period for trend strength
    pub adx_period: usize,
    /// ATR period for volatility measurement
    pub atr_period: usize,
    /// Bollinger Bands period
    pub bb_period: usize,
    /// Bollinger Bands standard deviation
    pub bb_std: f64,
    /// Lookback period for volatility percentile
    pub volatility_lookback: usize,
}

impl Default for RegimeConfig {
    fn default() -> Self {
        Self {
            adx_period: 14,
            atr_period: 14,
            bb_period: 20,
            bb_std: 2.0,
            volatility_lookback: 50,
        }
    }
}

/// Market regime detector
pub struct RegimeDetector {
    config: RegimeConfig,
}

impl RegimeDetector {
    pub fn new(config: RegimeConfig) -> Self {
        Self { config }
    }

    /// Detect the current market regime
    pub fn detect_regime(&self, data: &MarketData) -> Result<MarketRegime> {
        if data.len() < self.config.adx_period * 2 {
            // Default to ranging if insufficient data
            return Ok(MarketRegime::Ranging);
        }

        // Calculate ADX for trend strength
        let (adx_values, _plus_di, _minus_di) = indicators::adx(
            &data.highs(),
            &data.lows(),
            &data.closes(),
            self.config.adx_period,
        )?;

        let current_adx = adx_values
            .iter()
            .rev()
            .find(|&&x| !x.is_nan())
            .copied()
            .unwrap_or(0.0);

        // Calculate ATR for volatility
        let atr_values = indicators::atr(
            &data.highs(),
            &data.lows(),
            &data.closes(),
            self.config.atr_period,
        )?;

        let current_atr = atr_values
            .iter()
            .rev()
            .find(|&&x| !x.is_nan())
            .copied()
            .unwrap_or(0.0);

        // Calculate ATR percentile for volatility regime
        let volatility_percentile = self.calculate_volatility_percentile(&atr_values);

        // Calculate Bollinger Bands width for volatility measure
        let bb = indicators::bollinger_bands(
            &data.closes(),
            self.config.bb_period,
            self.config.bb_std,
        )?;

        let bb_width = if let (Some(&upper), Some(&lower), Some(&middle)) =
            (bb.upper.last(), bb.lower.last(), bb.middle.last())
        {
            if middle > 0.0 {
                ((upper - lower) / middle) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Regime classification logic
        let regime = self.classify_regime(current_adx, volatility_percentile, bb_width);

        Ok(regime)
    }

    /// Classify regime based on indicators
    fn classify_regime(&self, adx: f64, vol_percentile: f64, bb_width: f64) -> MarketRegime {
        // Priority 1: Check for extreme volatility
        if vol_percentile > 75.0 || bb_width > 4.0 {
            return MarketRegime::HighVolatility;
        }

        if vol_percentile < 25.0 || bb_width < 1.5 {
            return MarketRegime::LowVolatility;
        }

        // Priority 2: Check trend strength
        if adx > 25.0 {
            return MarketRegime::StrongTrend;
        }

        if adx > 20.0 {
            return MarketRegime::WeakTrend;
        }

        // Default: Ranging
        MarketRegime::Ranging
    }

    /// Calculate volatility percentile (0-100)
    fn calculate_volatility_percentile(&self, atr_values: &[f64]) -> f64 {
        let lookback = self.config.volatility_lookback.min(atr_values.len());

        if lookback < 10 {
            return 50.0; // Default to median if not enough data
        }

        // Get recent ATR values
        let recent_atr: Vec<f64> = atr_values
            .iter()
            .rev()
            .filter(|&&x| !x.is_nan())
            .take(lookback)
            .copied()
            .collect();

        if recent_atr.is_empty() {
            return 50.0;
        }

        let current_atr = recent_atr[0];

        // Count how many values are below current
        let below_count = recent_atr.iter().filter(|&&x| x < current_atr).count();

        (below_count as f64 / recent_atr.len() as f64) * 100.0
    }

    /// Get detailed regime analysis
    pub fn analyze_regime(&self, data: &MarketData) -> Result<RegimeAnalysis> {
        let regime = self.detect_regime(data)?;

        // Calculate indicators
        let (adx_values, plus_di, minus_di) = indicators::adx(
            &data.highs(),
            &data.lows(),
            &data.closes(),
            self.config.adx_period,
        )?;

        let atr_values = indicators::atr(
            &data.highs(),
            &data.lows(),
            &data.closes(),
            self.config.atr_period,
        )?;

        let current_adx = adx_values
            .iter()
            .rev()
            .find(|&&x| !x.is_nan())
            .copied()
            .unwrap_or(0.0);

        let current_plus_di = plus_di
            .iter()
            .rev()
            .find(|&&x| !x.is_nan())
            .copied()
            .unwrap_or(0.0);

        let current_minus_di = minus_di
            .iter()
            .rev()
            .find(|&&x| !x.is_nan())
            .copied()
            .unwrap_or(0.0);

        let current_atr = atr_values
            .iter()
            .rev()
            .find(|&&x| !x.is_nan())
            .copied()
            .unwrap_or(0.0);

        let volatility_percentile = self.calculate_volatility_percentile(&atr_values);

        let trend_direction = if current_plus_di > current_minus_di {
            TrendDirection::Up
        } else {
            TrendDirection::Down
        };

        Ok(RegimeAnalysis {
            regime,
            adx: current_adx,
            plus_di: current_plus_di,
            minus_di: current_minus_di,
            atr: current_atr,
            volatility_percentile,
            trend_direction,
        })
    }
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Up,
    Down,
}

impl TrendDirection {
    pub fn description(&self) -> &'static str {
        match self {
            TrendDirection::Up => "Uptrend",
            TrendDirection::Down => "Downtrend",
        }
    }
}

/// Detailed regime analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeAnalysis {
    pub regime: MarketRegime,
    pub adx: f64,
    pub plus_di: f64,
    pub minus_di: f64,
    pub atr: f64,
    pub volatility_percentile: f64,
    pub trend_direction: TrendDirection,
}

impl RegimeAnalysis {
    pub fn summary(&self) -> String {
        format!(
            "{} | {} | ADX: {:.1} | ATR: {:.2} | Vol Percentile: {:.0}%",
            self.regime.description(),
            self.trend_direction.description(),
            self.adx,
            self.atr,
            self.volatility_percentile
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Candle, MarketData};
    use chrono::Utc;

    fn create_test_data(prices: Vec<f64>) -> MarketData {
        let candles: Vec<Candle> = prices
            .iter()
            .enumerate()
            .map(|(i, &close)| Candle {
                timestamp: Utc::now() + chrono::Duration::hours(i as i64),
                open: close * 0.99,
                high: close * 1.01,
                low: close * 0.98,
                close,
                volume: 1000.0,
            })
            .collect();

        MarketData {
            symbol: "XAU/USD".to_string(),
            candles,
        }
    }

    #[test]
    fn test_regime_detection() {
        // Create trending data
        let trending_prices: Vec<f64> = (0..100).map(|i| 2000.0 + i as f64 * 2.0).collect();
        let trending_data = create_test_data(trending_prices);

        let detector = RegimeDetector::new(RegimeConfig::default());
        let regime = detector.detect_regime(&trending_data).unwrap();

        // Should detect some kind of trend
        assert!(matches!(
            regime,
            MarketRegime::StrongTrend | MarketRegime::WeakTrend
        ));
    }

    #[test]
    fn test_position_size_multiplier() {
        assert_eq!(MarketRegime::StrongTrend.position_size_multiplier(), 1.2);
        assert_eq!(MarketRegime::HighVolatility.position_size_multiplier(), 0.6);
    }
}
