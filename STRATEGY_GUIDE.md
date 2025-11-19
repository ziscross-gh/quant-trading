# Trading Strategy Implementation Guide

This guide shows you how to create your own custom trading strategies for the Gold/USD trading bot.

## Table of Contents
- [Strategy Basics](#strategy-basics)
- [Quick Start](#quick-start)
- [Strategy Examples](#strategy-examples)
- [Best Practices](#best-practices)
- [Testing Your Strategy](#testing-your-strategy)
- [Advanced Topics](#advanced-topics)

## Strategy Basics

### What is a Strategy?

A strategy implements the `Strategy` trait and must provide:
1. **Indicator Calculation** - Technical indicators from market data
2. **Signal Generation** - BUY/SELL/HOLD decisions based on indicators
3. **State Management** - Resetting strategy state between runs

### Strategy Trait

```rust
#[async_trait]
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn calculate_indicators(&self, data: &MarketData) -> Result<IndicatorData>;
    fn generate_signal(&self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal>;
    fn reset(&mut self);
}
```

## Quick Start

### 1. Create Your Strategy File

```bash
cd rust/src/strategies
touch my_strategy.rs
```

### 2. Basic Template

```rust
//! My Custom Strategy
//!
//! Describe your strategy logic here

use super::{IndicatorData, Strategy};
use crate::config::Config;
use crate::indicators::*;
use crate::{Error, MarketData, Result, Signal};
use tracing::{debug, info};

pub struct MyStrategy {
    config: Config,
    // Add your strategy parameters here
    parameter1: f64,
    parameter2: usize,
}

impl MyStrategy {
    pub fn new(config: Config) -> Self {
        info!("Initializing My Strategy");

        Self {
            config,
            parameter1: 1.5,
            parameter2: 20,
        }
    }
}

#[async_trait::async_trait]
impl Strategy for MyStrategy {
    fn name(&self) -> &str {
        "My Strategy"
    }

    fn calculate_indicators(&self, data: &MarketData) -> Result<IndicatorData> {
        // Extract price data
        let closes: Vec<f64> = data.iter().map(|c| c.close).collect();

        // Calculate indicators
        // TODO: Add your indicator calculations

        Ok(IndicatorData {
            ma_fast: vec![],
            ma_slow: vec![],
            rsi: vec![],
            bb_upper: vec![],
            bb_middle: vec![],
            bb_lower: vec![],
            volume_ma: vec![],
            momentum: vec![],
        })
    }

    fn generate_signal(&self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
        // Get last values
        let last_candle = data.last()
            .ok_or_else(|| Error::InvalidData("No data".to_string()))?;

        let values = indicators.last_values()
            .ok_or_else(|| Error::InvalidData("No indicators".to_string()))?;

        // TODO: Implement your signal logic

        Ok(Signal::Hold)
    }

    fn reset(&mut self) {
        debug!("Resetting My Strategy");
    }
}
```

### 3. Register Your Strategy

Edit `rust/src/strategies/mod.rs`:

```rust
pub mod my_strategy;
pub use my_strategy::MyStrategy;
```

### 4. Use Your Strategy

```rust
use gold_quant_trading::strategies::MyStrategy;

let config = Config::load()?;
let strategy = MyStrategy::new(config);
```

## Strategy Examples

### Example 1: Simple Moving Average Crossover

```rust
fn calculate_indicators(&self, data: &MarketData) -> Result<IndicatorData> {
    let closes: Vec<f64> = data.iter().map(|c| c.close).collect();

    // Fast MA (20 period)
    let ma_fast = sma(&closes, 20)?;

    // Slow MA (50 period)
    let ma_slow = sma(&closes, 50)?;

    Ok(IndicatorData {
        ma_fast,
        ma_slow,
        rsi: vec![],
        bb_upper: vec![],
        bb_middle: vec![],
        bb_lower: vec![],
        volume_ma: vec![],
        momentum: vec![],
    })
}

fn generate_signal(&self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
    let values = indicators.last_values()
        .ok_or_else(|| Error::InvalidData("No indicators".to_string()))?;

    // BUY when fast MA crosses above slow MA
    if values.ma_fast > values.ma_slow {
        info!("🟢 BUY: Fast MA ({:.2}) > Slow MA ({:.2})",
            values.ma_fast, values.ma_slow);
        return Ok(Signal::Buy);
    }

    // SELL when fast MA crosses below slow MA
    if values.ma_fast < values.ma_slow {
        info!("🔴 SELL: Fast MA ({:.2}) < Slow MA ({:.2})",
            values.ma_fast, values.ma_slow);
        return Ok(Signal::Sell);
    }

    Ok(Signal::Hold)
}
```

### Example 2: RSI Extremes

```rust
fn calculate_indicators(&self, data: &MarketData) -> Result<IndicatorData> {
    let closes: Vec<f64> = data.iter().map(|c| c.close).collect();

    // Calculate 14-period RSI
    let rsi_values = rsi(&closes, 14)?;

    Ok(IndicatorData {
        rsi: rsi_values,
        // ... other fields
    })
}

fn generate_signal(&self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
    let values = indicators.last_values()
        .ok_or_else(|| Error::InvalidData("No indicators".to_string()))?;

    // BUY when RSI < 30 (oversold)
    if values.rsi < 30.0 {
        info!("🟢 BUY: RSI oversold ({:.2})", values.rsi);
        return Ok(Signal::Buy);
    }

    // SELL when RSI > 70 (overbought)
    if values.rsi > 70.0 {
        info!("🔴 SELL: RSI overbought ({:.2})", values.rsi);
        return Ok(Signal::Sell);
    }

    Ok(Signal::Hold)
}
```

### Example 3: Multi-Condition Strategy

```rust
fn generate_signal(&self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
    let last_candle = data.last().unwrap();
    let values = indicators.last_values().unwrap();

    // BUY conditions (ALL must be true)
    let ma_bullish = values.ma_fast > values.ma_slow;
    let rsi_not_overbought = values.rsi < 65.0;
    let price_above_bb_middle = last_candle.close > values.bb_middle;
    let strong_volume = last_candle.volume > values.volume_ma * 1.2;

    if ma_bullish && rsi_not_overbought && price_above_bb_middle && strong_volume {
        info!("🟢 BUY: All conditions met");
        return Ok(Signal::Buy);
    }

    // SELL conditions
    let ma_bearish = values.ma_fast < values.ma_slow;
    let rsi_not_oversold = values.rsi > 35.0;
    let price_below_bb_middle = last_candle.close < values.bb_middle;

    if ma_bearish && rsi_not_oversold && price_below_bb_middle && strong_volume {
        info!("🔴 SELL: All conditions met");
        return Ok(Signal::Sell);
    }

    Ok(Signal::Hold)
}
```

## Best Practices

### 1. Data Validation

Always check if you have enough data:

```rust
fn calculate_indicators(&self, data: &MarketData) -> Result<IndicatorData> {
    let closes: Vec<f64> = data.iter().map(|c| c.close).collect();

    let min_required = 50; // Your longest indicator period
    if closes.len() < min_required {
        return Err(Error::InvalidData(format!(
            "Need {} candles, have {}",
            min_required, closes.len()
        )));
    }

    // ... rest of your code
}
```

### 2. Use Debug Logging

Help yourself debug with detailed logging:

```rust
debug!("Price: {:.2}, MA: {:.2}, RSI: {:.2}",
    last_close, values.ma_fast, values.rsi);

info!("🟢 BUY Signal: Price broke resistance at {:.2}", resistance);
```

### 3. Handle Edge Cases

```rust
// Avoid division by zero
let ratio = if denominator > 0.001 {
    numerator / denominator
} else {
    0.0
};

// Safe parsing
let price = value.parse::<f64>().unwrap_or(0.0);
```

### 4. Configuration from YAML

Use config values instead of hardcoding:

```rust
impl MyStrategy {
    pub fn new(config: Config) -> Self {
        Self {
            fast_period: config.strategy.fast_ma,
            slow_period: config.strategy.slow_ma,
            rsi_oversold: config.strategy.rsi_oversold,
            // ...
        }
    }
}
```

## Testing Your Strategy

### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Candle;
    use chrono::Utc;

    #[test]
    fn test_strategy_creation() {
        let config = Config::default();
        let strategy = MyStrategy::new(config);
        assert_eq!(strategy.name(), "My Strategy");
    }

    #[test]
    fn test_buy_signal() {
        let config = Config::default();
        let strategy = MyStrategy::new(config);

        // Create test data
        let mut data = Vec::new();
        for i in 0..60 {
            data.push(Candle {
                timestamp: Utc::now(),
                open: 2000.0 + i as f64,
                high: 2010.0 + i as f64,
                low: 1990.0 + i as f64,
                close: 2005.0 + i as f64,
                volume: 1000.0,
            });
        }

        let indicators = strategy.calculate_indicators(&data).unwrap();
        let signal = strategy.generate_signal(&data, &indicators).unwrap();

        // Assert signal is Buy or Hold
        assert!(matches!(signal, Signal::Buy | Signal::Hold));
    }
}
```

### 2. Backtest Your Strategy

```rust
// In main.rs or a test file
use gold_quant_trading::backtesting::BacktestEngine;
use gold_quant_trading::strategies::MyStrategy;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    let strategy = Arc::new(MyStrategy::new(config.clone()));

    let mut engine = BacktestEngine::new(config.clone(), strategy);
    let results = engine.run().await?;

    println!("Win Rate: {:.2}%", results.win_rate);
    println!("Total P&L: ${:.2}", results.total_pnl);

    Ok(())
}
```

### 3. Paper Trading Test

Start with paper trading to see real-time behavior:

```bash
# In .env
TRADING_MODE=paper

cargo run --bin trade --release
```

## Advanced Topics

### News-Enhanced Strategy

Wrap your strategy to add news sentiment:

```rust
use gold_quant_trading::strategies::NewsEnhancedStrategy;

let base_strategy = MyStrategy::new(config.clone());
let mut enhanced = NewsEnhancedStrategy::new(base_strategy, config);

// Now uses news sentiment + your technical signals
let signal = enhanced.generate_signal_with_news(&data, &indicators).await?;
```

### Custom Indicators

Create your own indicator:

```rust
pub fn my_custom_indicator(data: &[f64], period: usize) -> Result<Vec<f64>> {
    let mut result = Vec::new();

    for i in period..=data.len() {
        let window = &data[i - period..i];
        let value = window.iter().sum::<f64>() / period as f64;
        result.push(value);
    }

    // Pad with zeros
    let padding = data.len() - result.len();
    let mut final_result = vec![0.0; padding];
    final_result.extend(result);

    Ok(final_result)
}
```

### Multi-Timeframe Analysis

```rust
// Fetch data for multiple timeframes
let data_1h = fetcher.fetch_latest(Interval::Hour1, 500).await?;
let data_4h = fetcher.fetch_latest(Interval::Hour4, 125).await?;
let data_1d = fetcher.fetch_latest(Interval::Day1, 30).await?;

// Combine signals
let signal_1h = strategy.generate_signal(&data_1h, &indicators_1h)?;
let signal_4h = strategy.generate_signal(&data_4h, &indicators_4h)?;
let signal_1d = strategy.generate_signal(&data_1d, &indicators_1d)?;

// Trade only when all align
if signal_1h == Signal::Buy && signal_4h == Signal::Buy && signal_1d == Signal::Buy {
    // High confidence buy!
}
```

## Available Indicators

The system includes these built-in indicators:

```rust
use crate::indicators::*;

// Moving Averages
let sma_values = sma(&closes, period)?;
let ema_values = ema(&closes, period)?;

// Oscillators
let rsi_values = rsi(&closes, period)?;
let macd = macd(&closes, fast, slow, signal)?;

// Bands
let (upper, middle, lower) = bollinger_bands(&closes, period, std_dev)?;

// Volume
let volume_ma = sma(&volumes, period)?;

// Custom calculations
let momentum: Vec<f64> = closes.windows(2)
    .map(|w| (w[1] - w[0]) / w[0] * 100.0)
    .collect();
```

## Example Strategies Included

1. **GoldMomentumStrategy** - Trend following with MA/RSI/BB
2. **MeanReversionStrategy** - Buy low, sell high in ranges
3. **BreakoutStrategy** - Trade support/resistance breakouts
4. **NewsEnhancedStrategy** - Wrapper adding news sentiment

Study these for inspiration!

## Need Help?

- Check `rust/src/strategies/` for complete examples
- Run tests: `cargo test`
- Enable debug logging: `RUST_LOG=debug cargo run`
- Ask questions on GitHub Discussions

Happy Trading! 📈✨
