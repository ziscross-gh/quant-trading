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

pub mod analytics;
pub mod backtesting;
pub mod brokers;
pub mod config;
pub mod dashboard;
pub mod data;
// Database module requires DATABASE_URL - uncomment when PostgreSQL is set up
// pub mod database;
pub mod error;
pub mod execution;
pub mod indicators;
pub mod learning;
pub mod ml;
pub mod monitoring;
pub mod news;
pub mod notifications;
pub mod optimization;
pub mod regime;
pub mod risk_management;
pub mod strategies;
pub mod types;

pub use error::{Error, Result};
pub use types::*;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
