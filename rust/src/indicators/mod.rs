//! Technical indicators module

use crate::{Error, Result};

/// Calculate Simple Moving Average (SMA)
pub fn sma(prices: &[f64], period: usize) -> Result<Vec<f64>> {
    if prices.len() < period {
        return Err(Error::InsufficientData {
            needed: period,
            actual: prices.len(),
        });
    }

    let mut sma_values = vec![f64::NAN; period - 1];

    for i in (period - 1)..prices.len() {
        let window = &prices[i - period + 1..=i];
        let sum: f64 = window.iter().sum();
        sma_values.push(sum / period as f64);
    }

    Ok(sma_values)
}

/// Calculate Exponential Moving Average (EMA)
pub fn ema(prices: &[f64], period: usize) -> Result<Vec<f64>> {
    if prices.len() < period {
        return Err(Error::InsufficientData {
            needed: period,
            actual: prices.len(),
        });
    }

    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut ema_values = vec![f64::NAN; period - 1];

    // Start with SMA for first value
    let first_sma: f64 = prices[..period].iter().sum::<f64>() / period as f64;
    ema_values.push(first_sma);

    let mut prev_ema = first_sma;
    for &price in &prices[period..] {
        let ema = (price - prev_ema) * multiplier + prev_ema;
        ema_values.push(ema);
        prev_ema = ema;
    }

    Ok(ema_values)
}

/// Calculate Relative Strength Index (RSI)
pub fn rsi(prices: &[f64], period: usize) -> Result<Vec<f64>> {
    if prices.len() < period + 1 {
        return Err(Error::InsufficientData {
            needed: period + 1,
            actual: prices.len(),
        });
    }

    let mut rsi_values = vec![f64::NAN; period];

    // Calculate price changes
    let mut gains = Vec::new();
    let mut losses = Vec::new();

    for i in 1..prices.len() {
        let change = prices[i] - prices[i - 1];
        if change > 0.0 {
            gains.push(change);
            losses.push(0.0);
        } else {
            gains.push(0.0);
            losses.push(change.abs());
        }
    }

    // Calculate average gain and loss
    let mut avg_gain: f64 = gains[..period].iter().sum::<f64>() / period as f64;
    let mut avg_loss: f64 = losses[..period].iter().sum::<f64>() / period as f64;

    // Calculate first RSI
    let rs = if avg_loss == 0.0 { 100.0 } else { avg_gain / avg_loss };
    rsi_values.push(100.0 - (100.0 / (1.0 + rs)));

    // Calculate subsequent RSI values using smoothed averages
    for i in period..gains.len() {
        avg_gain = (avg_gain * (period as f64 - 1.0) + gains[i]) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + losses[i]) / period as f64;

        let rs = if avg_loss == 0.0 { 100.0 } else { avg_gain / avg_loss };
        let rsi = 100.0 - (100.0 / (1.0 + rs));
        rsi_values.push(rsi);
    }

    Ok(rsi_values)
}

/// Bollinger Bands result
#[derive(Debug, Clone)]
pub struct BollingerBands {
    pub upper: Vec<f64>,
    pub middle: Vec<f64>,
    pub lower: Vec<f64>,
}

/// Calculate Bollinger Bands
pub fn bollinger_bands(prices: &[f64], period: usize, std_dev: f64) -> Result<BollingerBands> {
    if prices.len() < period {
        return Err(Error::InsufficientData {
            needed: period,
            actual: prices.len(),
        });
    }

    let middle = sma(prices, period)?;
    let mut upper = Vec::new();
    let mut lower = Vec::new();

    for i in 0..middle.len() {
        if i < period - 1 {
            upper.push(f64::NAN);
            lower.push(f64::NAN);
        } else {
            let window = &prices[i - period + 1..=i];
            let mean = middle[i];
            let variance: f64 = window.iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>() / period as f64;
            let std = variance.sqrt();

            upper.push(mean + std_dev * std);
            lower.push(mean - std_dev * std);
        }
    }

    Ok(BollingerBands {
        upper,
        middle,
        lower,
    })
}

/// Calculate standard deviation
pub fn std_dev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
    let variance: f64 = values.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / values.len() as f64;

    variance.sqrt()
}

/// Calculate percentage change
pub fn pct_change(prices: &[f64]) -> Vec<f64> {
    let mut changes = vec![f64::NAN];

    for i in 1..prices.len() {
        let change = (prices[i] - prices[i - 1]) / prices[i - 1];
        changes.push(change);
    }

    changes
}

/// Calculate momentum (rate of change)
pub fn momentum(prices: &[f64], period: usize) -> Vec<f64> {
    let mut momentum = vec![f64::NAN; period];

    for i in period..prices.len() {
        let change = ((prices[i] - prices[i - period]) / prices[i - period]) * 100.0;
        momentum.push(change);
    }

    momentum
}

/// Calculate Average True Range (ATR)
///
/// ATR measures market volatility. It's the average of true ranges over a period.
/// True Range is the greatest of:
/// - Current High - Current Low
/// - abs(Current High - Previous Close)
/// - abs(Current Low - Previous Close)
pub fn atr(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Result<Vec<f64>> {
    if highs.len() != lows.len() || highs.len() != closes.len() {
        return Err(Error::Config(
            "High, Low, and Close arrays must have the same length".to_string()
        ));
    }

    if highs.len() < period + 1 {
        return Err(Error::InsufficientData {
            needed: period + 1,
            actual: highs.len(),
        });
    }

    let mut atr_values = vec![f64::NAN; period];
    let mut true_ranges = Vec::new();

    // Calculate true ranges
    for i in 1..highs.len() {
        let high_low = highs[i] - lows[i];
        let high_close = (highs[i] - closes[i - 1]).abs();
        let low_close = (lows[i] - closes[i - 1]).abs();

        let tr = high_low.max(high_close).max(low_close);
        true_ranges.push(tr);
    }

    // Calculate first ATR as simple average
    let first_atr: f64 = true_ranges[..period].iter().sum::<f64>() / period as f64;
    atr_values.push(first_atr);

    // Calculate subsequent ATR values using smoothed average (Wilder's smoothing)
    let mut prev_atr = first_atr;
    for i in period..true_ranges.len() {
        let atr = (prev_atr * (period as f64 - 1.0) + true_ranges[i]) / period as f64;
        atr_values.push(atr);
        prev_atr = atr;
    }

    Ok(atr_values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma() {
        let prices = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let sma = sma(&prices, 3).unwrap();
        assert!((sma[2] - 2.0).abs() < 1e-10);
        assert!((sma[3] - 3.0).abs() < 1e-10);
        assert!((sma[4] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_rsi() {
        let prices = vec![
            44.0, 44.25, 44.5, 43.75, 44.0, 44.25, 45.0, 45.25, 45.5, 45.75,
            46.0, 45.5, 45.0, 44.5, 44.0,
        ];
        let rsi = rsi(&prices, 14).unwrap();
        assert!(rsi.len() > 0);
        assert!(rsi.last().unwrap() >= &0.0 && rsi.last().unwrap() <= &100.0);
    }

    #[test]
    fn test_bollinger_bands() {
        let prices = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let bb = bollinger_bands(&prices, 3, 2.0).unwrap();
        assert_eq!(bb.middle.len(), 5);
        assert_eq!(bb.upper.len(), 5);
        assert_eq!(bb.lower.len(), 5);
    }
}
