//! Paper trading broker (simulation)
//!
//! Simulates order execution without real money.
//! Perfect for testing strategies safely.

use super::*;
use crate::config::Config;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;
use uuid::Uuid;

/// Paper broker state
struct PaperState {
    cash: f64,
    positions: HashMap<String, BrokerPosition>,
    orders: HashMap<String, OrderConfirmation>,
    next_order_id: u64,
}

/// Paper trading broker
pub struct PaperBroker {
    state: Arc<Mutex<PaperState>>,
    initial_capital: f64,
}

impl PaperBroker {
    pub fn new(config: &Config) -> Result<Self> {
        let initial_capital = config.trading.initial_capital;

        info!("Paper broker initialized with ${:,.2}", initial_capital);

        Ok(Self {
            state: Arc::new(Mutex::new(PaperState {
                cash: initial_capital,
                positions: HashMap::new(),
                orders: HashMap::new(),
                next_order_id: 1,
            })),
            initial_capital,
        })
    }

    fn generate_order_id(&self) -> String {
        let mut state = self.state.lock().unwrap();
        let id = state.next_order_id;
        state.next_order_id += 1;
        format!("PAPER_{}", id)
    }
}

#[async_trait]
impl Broker for PaperBroker {
    async fn place_order(&self, order: Order) -> Result<OrderConfirmation> {
        let mut state = self.state.lock().unwrap();

        let order_id = self.generate_order_id();

        // Get current price (in real scenario, would fetch from market)
        // For now, simulate with a fixed price
        let simulated_price = 2000.0; // Gold price approximation

        let total_cost = order.quantity * simulated_price;

        // Check if we have enough cash
        if matches!(order.side, OrderSide::Buy) && total_cost > state.cash {
            return Err(Error::Execution("Insufficient funds".to_string()));
        }

        // Update cash
        match order.side {
            OrderSide::Buy => state.cash -= total_cost,
            OrderSide::Sell => state.cash += total_cost,
        }

        // Update position
        let position = state
            .positions
            .entry(order.symbol.clone())
            .or_insert(BrokerPosition {
                symbol: order.symbol.clone(),
                quantity: 0.0,
                avg_entry_price: 0.0,
                market_value: 0.0,
                unrealized_pnl: 0.0,
                unrealized_pnl_pct: 0.0,
                side: order.side.clone(),
            });

        match order.side {
            OrderSide::Buy => {
                let new_qty = position.quantity + order.quantity;
                position.avg_entry_price = ((position.avg_entry_price * position.quantity)
                    + (simulated_price * order.quantity))
                    / new_qty;
                position.quantity = new_qty;
            }
            OrderSide::Sell => {
                position.quantity -= order.quantity;
                if position.quantity <= 0.0 {
                    state.positions.remove(&order.symbol);
                }
            }
        }

        let confirmation = OrderConfirmation {
            order_id: order_id.clone(),
            symbol: order.symbol,
            filled_qty: order.quantity,
            filled_price: simulated_price,
            status: OrderStatus::Filled,
            filled_at: Some(Utc::now()),
        };

        state.orders.insert(order_id, confirmation.clone());

        info!("Paper order filled: {:.2} @ ${:.2}", order.quantity, simulated_price);

        Ok(confirmation)
    }

    async fn cancel_order(&self, order_id: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();

        if let Some(order) = state.orders.get_mut(order_id) {
            order.status = OrderStatus::Cancelled;
            Ok(())
        } else {
            Err(Error::Execution(format!("Order not found: {}", order_id)))
        }
    }

    async fn get_order(&self, order_id: &str) -> Result<OrderConfirmation> {
        let state = self.state.lock().unwrap();

        state
            .orders
            .get(order_id)
            .cloned()
            .ok_or_else(|| Error::Execution(format!("Order not found: {}", order_id)))
    }

    async fn get_positions(&self) -> Result<Vec<BrokerPosition>> {
        let state = self.state.lock().unwrap();
        Ok(state.positions.values().cloned().collect())
    }

    async fn get_account(&self) -> Result<AccountInfo> {
        let state = self.state.lock().unwrap();

        let portfolio_value: f64 = state
            .positions
            .values()
            .map(|p| p.market_value)
            .sum::<f64>()
            + state.cash;

        Ok(AccountInfo {
            account_id: "PAPER".to_string(),
            cash: state.cash,
            buying_power: state.cash * 4.0, // Simulated margin
            portfolio_value,
            equity: portfolio_value,
        })
    }

    async fn get_open_orders(&self) -> Result<Vec<OrderConfirmation>> {
        let state = self.state.lock().unwrap();

        Ok(state
            .orders
            .values()
            .filter(|o| matches!(o.status, OrderStatus::Pending | OrderStatus::PartiallyFilled))
            .cloned()
            .collect())
    }

    async fn close_position(&self, symbol: &str) -> Result<OrderConfirmation> {
        let mut state = self.state.lock().unwrap();

        if let Some(position) = state.positions.remove(symbol) {
            let simulated_price = 2000.0;
            state.cash += position.quantity * simulated_price;

            Ok(OrderConfirmation {
                order_id: self.generate_order_id(),
                symbol: symbol.to_string(),
                filled_qty: position.quantity,
                filled_price: simulated_price,
                status: OrderStatus::Filled,
                filled_at: Some(Utc::now()),
            })
        } else {
            Err(Error::Execution(format!("No position found for {}", symbol)))
        }
    }

    async fn is_market_open(&self) -> Result<bool> {
        // Paper trading is always "open"
        Ok(true)
    }

    fn name(&self) -> &str {
        "Paper Trading"
    }
}
