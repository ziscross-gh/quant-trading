//! Core types for the trading system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Trading signal type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    /// Buy/Long signal
    Buy = 1,
    /// Hold/No action
    Hold = 0,
    /// Sell/Short signal
    Sell = -1,
}

impl Signal {
    pub fn from_i8(value: i8) -> Option<Self> {
        match value {
            1 => Some(Signal::Buy),
            0 => Some(Signal::Hold),
            -1 => Some(Signal::Sell),
            _ => None,
        }
    }

    pub fn to_i8(self) -> i8 {
        self as i8
    }

    pub fn description(&self) -> &'static str {
        match self {
            Signal::Buy => "BUY/LONG",
            Signal::Hold => "HOLD/NO ACTION",
            Signal::Sell => "SELL/SHORT",
        }
    }
}

/// OHLCV (Open, High, Low, Close, Volume) candle data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Market data time series
#[derive(Debug, Clone)]
pub struct MarketData {
    pub symbol: String,
    pub candles: Vec<Candle>,
}

impl MarketData {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            candles: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.candles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.candles.is_empty()
    }

    pub fn last(&self) -> Option<&Candle> {
        self.candles.last()
    }

    pub fn closes(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.close).collect()
    }

    pub fn highs(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.high).collect()
    }

    pub fn lows(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.low).collect()
    }

    pub fn volumes(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.volume).collect()
    }
}

/// Trading position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: u64,
    pub signal: Signal,
    pub entry_price: f64,
    pub size: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub trailing_stop: Option<f64>,
    pub entry_time: DateTime<Utc>,
    pub peak_price: f64,
}

impl Position {
    pub fn new(
        id: u64,
        signal: Signal,
        entry_price: f64,
        size: f64,
        stop_loss: f64,
        take_profit: f64,
        entry_time: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            signal,
            entry_price,
            size,
            stop_loss,
            take_profit,
            trailing_stop: None,
            entry_time,
            peak_price: entry_price,
        }
    }

    /// Calculate unrealized P&L
    pub fn unrealized_pnl(&self, current_price: f64) -> f64 {
        match self.signal {
            Signal::Buy => (current_price - self.entry_price) * self.size,
            Signal::Sell => (self.entry_price - current_price) * self.size,
            Signal::Hold => 0.0,
        }
    }

    /// Calculate unrealized P&L percentage
    pub fn unrealized_pnl_pct(&self, current_price: f64) -> f64 {
        let pnl = self.unrealized_pnl(current_price);
        (pnl / (self.entry_price * self.size)) * 100.0
    }
}

/// Completed trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: f64,
    pub signal: Signal,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub entry_time: DateTime<Utc>,
    pub exit_time: DateTime<Utc>,
    pub duration_hours: f64,
    pub commission: f64,
    pub slippage: f64,
}

impl Trade {
    pub fn new(
        position: &Position,
        exit_price: f64,
        exit_time: DateTime<Utc>,
        commission: f64,
        slippage: f64,
    ) -> Self {
        let pnl = match position.signal {
            Signal::Buy => (exit_price - position.entry_price) * position.size,
            Signal::Sell => (position.entry_price - exit_price) * position.size,
            Signal::Hold => 0.0,
        } - commission;

        let pnl_pct = (pnl / (position.entry_price * position.size)) * 100.0;
        let duration = exit_time.signed_duration_since(position.entry_time);
        let duration_hours = duration.num_seconds() as f64 / 3600.0;

        Self {
            entry_price: position.entry_price,
            exit_price,
            size: position.size,
            signal: position.signal,
            pnl,
            pnl_pct,
            entry_time: position.entry_time,
            exit_time,
            duration_hours,
            commission,
            slippage,
        }
    }

    pub fn is_winner(&self) -> bool {
        self.pnl > 0.0
    }
}

/// Trading signal result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalResult {
    pub signal: Signal,
    pub timestamp: DateTime<Utc>,
    pub price: f64,
    pub strategy: String,
}

/// Equity point for tracking portfolio value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub timestamp: DateTime<Utc>,
    pub equity: f64,
    pub price: f64,
}

/// Trading mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradingMode {
    Paper,
    Live,
}

impl std::str::FromStr for TradingMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "paper" => Ok(TradingMode::Paper),
            "live" => Ok(TradingMode::Live),
            _ => Err(format!("Invalid trading mode: {}", s)),
        }
    }
}

/// Data interval
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interval {
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "30m")]
    ThirtyMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "1d")]
    OneDay,
}

impl Interval {
    pub fn as_str(&self) -> &'static str {
        match self {
            Interval::OneMinute => "1m",
            Interval::FiveMinutes => "5m",
            Interval::FifteenMinutes => "15m",
            Interval::ThirtyMinutes => "30m",
            Interval::OneHour => "1h",
            Interval::OneDay => "1d",
        }
    }
}

impl std::str::FromStr for Interval {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1m" => Ok(Interval::OneMinute),
            "5m" => Ok(Interval::FiveMinutes),
            "15m" => Ok(Interval::FifteenMinutes),
            "30m" => Ok(Interval::ThirtyMinutes),
            "1h" => Ok(Interval::OneHour),
            "1d" => Ok(Interval::OneDay),
            _ => Err(format!("Invalid interval: {}", s)),
        }
    }
}
