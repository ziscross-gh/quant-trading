//! Alpaca broker integration
//!
//! Alpaca offers commission-free trading with a clean REST API.
//! Perfect for algorithmic trading with paper and live accounts.

use super::*;
use crate::config::Config;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

const ALPACA_PAPER_URL: &str = "https://paper-api.alpaca.markets";
const ALPACA_LIVE_URL: &str = "https://api.alpaca.markets";

/// Alpaca API response structures
#[derive(Debug, Deserialize)]
struct AlpacaOrder {
    id: String,
    symbol: String,
    qty: String,
    filled_qty: String,
    filled_avg_price: Option<String>,
    status: String,
    filled_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AlpacaPosition {
    symbol: String,
    qty: String,
    avg_entry_price: String,
    market_value: String,
    unrealized_pl: String,
    unrealized_plpc: String,
    side: String,
}

#[derive(Debug, Deserialize)]
struct AlpacaAccount {
    id: String,
    cash: String,
    buying_power: String,
    portfolio_value: String,
    equity: String,
}

#[derive(Debug, Serialize)]
struct AlpacaOrderRequest {
    symbol: String,
    qty: String,
    side: String,
    #[serde(rename = "type")]
    order_type: String,
    time_in_force: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_price: Option<String>,
}

/// Alpaca broker implementation
pub struct AlpacaBroker {
    client: Client,
    api_key: String,
    api_secret: String,
    base_url: String,
    paper_trading: bool,
}

impl AlpacaBroker {
    /// Create new Alpaca broker
    pub fn new(config: &Config) -> Result<Self> {
        let api_key = std::env::var("ALPACA_API_KEY")
            .or_else(|_| std::env::var("APCA_API_KEY_ID"))
            .map_err(|_| Error::Config("ALPACA_API_KEY not set".to_string()))?;

        let api_secret = std::env::var("ALPACA_API_SECRET")
            .or_else(|_| std::env::var("APCA_API_SECRET_KEY"))
            .map_err(|_| Error::Config("ALPACA_API_SECRET not set".to_string()))?;

        let paper_trading = config.trading.trading_mode == crate::TradingMode::Paper;
        let base_url = if paper_trading {
            ALPACA_PAPER_URL.to_string()
        } else {
            ALPACA_LIVE_URL.to_string()
        };

        info!("Alpaca broker initialized in {} mode",
            if paper_trading { "PAPER" } else { "LIVE" });

        Ok(Self {
            client: Client::new(),
            api_key,
            api_secret,
            base_url,
            paper_trading,
        })
    }

    /// Make authenticated request
    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);

        let mut request = self.client
            .request(method, &url)
            .header("APCA-API-KEY-ID", &self.api_key)
            .header("APCA-API-SECRET-KEY", &self.api_secret);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| Error::Execution(format!("Alpaca API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::Execution(format!(
                "Alpaca API error {}: {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Execution(format!("Failed to parse Alpaca response: {}", e)))
    }
}

#[async_trait]
impl Broker for AlpacaBroker {
    async fn place_order(&self, order: Order) -> Result<OrderConfirmation> {
        debug!("Placing Alpaca order: {:?}", order);

        let side = match order.side {
            OrderSide::Buy => "buy",
            OrderSide::Sell => "sell",
        };

        let (order_type, limit_price, stop_price) = match order.order_type {
            OrderType::Market => ("market", None, None),
            OrderType::Limit { limit_price } => {
                ("limit", Some(limit_price.to_string()), None)
            }
            OrderType::Stop { stop_price } => {
                ("stop", None, Some(stop_price.to_string()))
            }
            OrderType::StopLimit { stop_price, limit_price } => {
                ("stop_limit", Some(limit_price.to_string()), Some(stop_price.to_string()))
            }
        };

        let time_in_force = match order.time_in_force {
            TimeInForce::Day => "day",
            TimeInForce::GTC => "gtc",
            TimeInForce::IOC => "ioc",
            TimeInForce::FOK => "fok",
        };

        let request = AlpacaOrderRequest {
            symbol: order.symbol.clone(),
            qty: order.quantity.to_string(),
            side: side.to_string(),
            order_type: order_type.to_string(),
            time_in_force: time_in_force.to_string(),
            limit_price,
            stop_price,
        };

        let alpaca_order: AlpacaOrder = self
            .request(
                reqwest::Method::POST,
                "/v2/orders",
                Some(serde_json::to_value(request)?),
            )
            .await?;

        let status = match alpaca_order.status.as_str() {
            "new" | "pending_new" | "accepted" => OrderStatus::Pending,
            "partially_filled" => OrderStatus::PartiallyFilled,
            "filled" => OrderStatus::Filled,
            "canceled" | "pending_cancel" => OrderStatus::Cancelled,
            "rejected" | "expired" => OrderStatus::Rejected,
            _ => OrderStatus::Pending,
        };

        let filled_at = alpaca_order
            .filled_at
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(OrderConfirmation {
            order_id: alpaca_order.id,
            symbol: alpaca_order.symbol,
            filled_qty: alpaca_order.filled_qty.parse().unwrap_or(0.0),
            filled_price: alpaca_order
                .filled_avg_price
                .and_then(|p| p.parse().ok())
                .unwrap_or(0.0),
            status,
            filled_at,
        })
    }

    async fn cancel_order(&self, order_id: &str) -> Result<()> {
        debug!("Cancelling Alpaca order: {}", order_id);

        let _: serde_json::Value = self
            .request(
                reqwest::Method::DELETE,
                &format!("/v2/orders/{}", order_id),
                None,
            )
            .await?;

        Ok(())
    }

    async fn get_order(&self, order_id: &str) -> Result<OrderConfirmation> {
        let alpaca_order: AlpacaOrder = self
            .request(
                reqwest::Method::GET,
                &format!("/v2/orders/{}", order_id),
                None,
            )
            .await?;

        let status = match alpaca_order.status.as_str() {
            "filled" => OrderStatus::Filled,
            "partially_filled" => OrderStatus::PartiallyFilled,
            "canceled" => OrderStatus::Cancelled,
            "rejected" | "expired" => OrderStatus::Rejected,
            _ => OrderStatus::Pending,
        };

        let filled_at = alpaca_order
            .filled_at
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(OrderConfirmation {
            order_id: alpaca_order.id,
            symbol: alpaca_order.symbol,
            filled_qty: alpaca_order.filled_qty.parse().unwrap_or(0.0),
            filled_price: alpaca_order
                .filled_avg_price
                .and_then(|p| p.parse().ok())
                .unwrap_or(0.0),
            status,
            filled_at,
        })
    }

    async fn get_positions(&self) -> Result<Vec<BrokerPosition>> {
        let positions: Vec<AlpacaPosition> = self
            .request(reqwest::Method::GET, "/v2/positions", None)
            .await?;

        Ok(positions
            .into_iter()
            .map(|p| BrokerPosition {
                symbol: p.symbol,
                quantity: p.qty.parse().unwrap_or(0.0),
                avg_entry_price: p.avg_entry_price.parse().unwrap_or(0.0),
                market_value: p.market_value.parse().unwrap_or(0.0),
                unrealized_pnl: p.unrealized_pl.parse().unwrap_or(0.0),
                unrealized_pnl_pct: p.unrealized_plpc.parse().unwrap_or(0.0),
                side: if p.side == "long" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                },
            })
            .collect())
    }

    async fn get_account(&self) -> Result<AccountInfo> {
        let account: AlpacaAccount = self
            .request(reqwest::Method::GET, "/v2/account", None)
            .await?;

        Ok(AccountInfo {
            account_id: account.id,
            cash: account.cash.parse().unwrap_or(0.0),
            buying_power: account.buying_power.parse().unwrap_or(0.0),
            portfolio_value: account.portfolio_value.parse().unwrap_or(0.0),
            equity: account.equity.parse().unwrap_or(0.0),
        })
    }

    async fn get_open_orders(&self) -> Result<Vec<OrderConfirmation>> {
        let orders: Vec<AlpacaOrder> = self
            .request(reqwest::Method::GET, "/v2/orders?status=open", None)
            .await?;

        Ok(orders
            .into_iter()
            .map(|o| {
                let status = match o.status.as_str() {
                    "filled" => OrderStatus::Filled,
                    "partially_filled" => OrderStatus::PartiallyFilled,
                    "canceled" => OrderStatus::Cancelled,
                    "rejected" => OrderStatus::Rejected,
                    _ => OrderStatus::Pending,
                };

                let filled_at = o
                    .filled_at
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                OrderConfirmation {
                    order_id: o.id,
                    symbol: o.symbol,
                    filled_qty: o.filled_qty.parse().unwrap_or(0.0),
                    filled_price: o
                        .filled_avg_price
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(0.0),
                    status,
                    filled_at,
                }
            })
            .collect())
    }

    async fn close_position(&self, symbol: &str) -> Result<OrderConfirmation> {
        debug!("Closing Alpaca position: {}", symbol);

        let alpaca_order: AlpacaOrder = self
            .request(
                reqwest::Method::DELETE,
                &format!("/v2/positions/{}", symbol),
                None,
            )
            .await?;

        Ok(OrderConfirmation {
            order_id: alpaca_order.id,
            symbol: alpaca_order.symbol,
            filled_qty: alpaca_order.filled_qty.parse().unwrap_or(0.0),
            filled_price: alpaca_order
                .filled_avg_price
                .and_then(|p| p.parse().ok())
                .unwrap_or(0.0),
            status: OrderStatus::Filled,
            filled_at: Some(Utc::now()),
        })
    }

    async fn is_market_open(&self) -> Result<bool> {
        #[derive(Deserialize)]
        struct Clock {
            is_open: bool,
        }

        let clock: Clock = self
            .request(reqwest::Method::GET, "/v2/clock", None)
            .await?;

        Ok(clock.is_open)
    }

    fn name(&self) -> &str {
        if self.paper_trading {
            "Alpaca (Paper)"
        } else {
            "Alpaca (Live)"
        }
    }
}
