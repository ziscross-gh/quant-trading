//! Broker integrations for live trading

use crate::{Error, Position, Result, Signal};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod alpaca;
pub mod interactive_brokers;
pub mod paper;

pub use alpaca::AlpacaBroker;
pub use interactive_brokers::InteractiveBrokersBroker;
pub use paper::PaperBroker;

/// Order types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit { limit_price: f64 },
    Stop { stop_price: f64 },
    StopLimit { stop_price: f64, limit_price: f64 },
}

/// Order side
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order time in force
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeInForce {
    Day,
    GTC,  // Good Till Cancelled
    IOC,  // Immediate Or Cancel
    FOK,  // Fill Or Kill
}

/// Order structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: f64,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
}

/// Order confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderConfirmation {
    pub order_id: String,
    pub symbol: String,
    pub filled_qty: f64,
    pub filled_price: f64,
    pub status: OrderStatus,
    pub filled_at: Option<DateTime<Utc>>,
}

/// Order status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub account_id: String,
    pub cash: f64,
    pub buying_power: f64,
    pub portfolio_value: f64,
    pub equity: f64,
}

/// Broker trait - all brokers must implement this
#[async_trait]
pub trait Broker: Send + Sync {
    /// Place a new order
    async fn place_order(&self, order: Order) -> Result<OrderConfirmation>;

    /// Cancel an existing order
    async fn cancel_order(&self, order_id: &str) -> Result<()>;

    /// Get order status
    async fn get_order(&self, order_id: &str) -> Result<OrderConfirmation>;

    /// Get all open positions
    async fn get_positions(&self) -> Result<Vec<BrokerPosition>>;

    /// Get account information
    async fn get_account(&self) -> Result<AccountInfo>;

    /// Get list of open orders
    async fn get_open_orders(&self) -> Result<Vec<OrderConfirmation>>;

    /// Close a position
    async fn close_position(&self, symbol: &str) -> Result<OrderConfirmation>;

    /// Check if market is open
    async fn is_market_open(&self) -> Result<bool>;

    /// Get broker name
    fn name(&self) -> &str;
}

/// Broker position (different from our internal Position)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerPosition {
    pub symbol: String,
    pub quantity: f64,
    pub avg_entry_price: f64,
    pub market_value: f64,
    pub unrealized_pnl: f64,
    pub unrealized_pnl_pct: f64,
    pub side: OrderSide,
}

/// Create broker from configuration
pub fn create_broker(broker_name: &str, config: &crate::config::Config) -> Result<Box<dyn Broker>> {
    match broker_name.to_lowercase().as_str() {
        "alpaca" => Ok(Box::new(AlpacaBroker::new(config)?)),
        "interactive_brokers" | "ib" => Ok(Box::new(InteractiveBrokersBroker::new(config)?)),
        "paper" => Ok(Box::new(PaperBroker::new(config)?)),
        _ => Err(Error::Config(format!("Unknown broker: {}", broker_name))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_creation() {
        let order = Order {
            symbol: "GC=F".to_string(),
            side: OrderSide::Buy,
            quantity: 1.0,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::Day,
        };

        assert_eq!(order.symbol, "GC=F");
        assert_eq!(order.quantity, 1.0);
    }
}
