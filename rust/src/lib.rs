//! # Gold/USD Autonomous Quantitative Trading System
//!
//! A high-performance, autonomous trading system for Gold/USD built in Rust.
//!
//! ## Features
//!
//! - Autonomous trading with real-time market monitoring
//! - Momentum-based strategy with multiple technical indicators
//! - Comprehensive risk management
//! - Backtesting framework with detailed performance metrics
//! - Paper and live trading modes
//! - High performance and memory safety

pub mod config;
pub mod data;
pub mod strategies;
pub mod backtesting;
pub mod execution;
pub mod risk_management;
pub mod indicators;
pub mod types;
pub mod error;

pub use error::{Error, Result};
pub use types::*;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
