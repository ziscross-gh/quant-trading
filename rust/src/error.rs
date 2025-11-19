//! Error types for the trading system

use thiserror::Error;

/// Result type alias for the trading system
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur in the trading system
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Data fetching error: {0}")]
    DataFetch(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("Risk management error: {0}")]
    RiskManagement(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Backtesting error: {0}")]
    Backtesting(String),

    #[error("Optimization error: {0}")]
    OptimizationError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Insufficient data: need at least {needed} points, got {actual}")]
    InsufficientData { needed: usize, actual: usize },

    #[error("Position not found: {0}")]
    PositionNotFound(u64),

    #[error("Invalid signal: {0}")]
    InvalidSignal(i8),

    #[error("Trading not allowed: {0}")]
    TradingNotAllowed(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<config::ConfigError> for Error {
    fn from(err: config::ConfigError) -> Self {
        Error::Config(err.to_string())
    }
}

impl From<csv::Error> for Error {
    fn from(err: csv::Error) -> Self {
        Error::Io(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}
