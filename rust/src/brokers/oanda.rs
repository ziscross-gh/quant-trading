//! OANDA broker integration
//!
//! OANDA offers forex and commodities trading with a robust REST API v20.
//! Excellent for Gold/USD (XAU_USD) algorithmic trading with demo and live accounts.
//!
//! API Documentation: https://developer.oanda.com/rest-live-v20/introduction/

use super::*;
use crate::config::Config;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use chrono::Datelike;

const OANDA_PRACTICE_URL: &str = "https://api-fxpractice.oanda.com";
const OANDA_LIVE_URL: &str = "https://api-fxtrade.oanda.com";

/// OANDA API response structures
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OandaOrderResponse {
    order_create_transaction: Option<OandaTransaction>,
    order_fill_transaction: Option<OandaOrderFill>,
    related_transaction_i_ds: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OandaTransaction {
    id: String,
    #[serde(rename = "type")]
    transaction_type: String,
    instrument: String,
    units: String,
    time: String,
    price: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OandaOrderFill {
    id: String,
    instrument: String,
    units: String,
    price: String,
    pl: String,
    time: String,
}

#[derive(Debug, Deserialize)]
struct OandaPosition {
    instrument: String,
    long: OandaPositionSide,
    short: OandaPositionSide,
    #[serde(rename = "unrealizedPL")]
    unrealized_pl: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OandaPositionSide {
    units: String,
    average_price: Option<String>,
    #[serde(rename = "unrealizedPL")]
    unrealized_pl: String,
}

#[derive(Debug, Deserialize)]
struct OandaAccount {
    account: OandaAccountDetails,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OandaAccountDetails {
    id: String,
    balance: String,
    #[serde(rename = "NAV")]
    nav: String,
    margin_available: String,
    #[serde(rename = "unrealizedPL")]
    unrealized_pl: String,
}

#[derive(Debug, Deserialize)]
struct OandaOrdersResponse {
    orders: Vec<OandaOrder>,
}

#[derive(Debug, Deserialize)]
struct OandaOrder {
    id: String,
    instrument: String,
    units: String,
    #[serde(rename = "type")]
    order_type: String,
    state: String,
    #[serde(rename = "createTime")]
    create_time: String,
}

#[derive(Debug, Deserialize)]
struct OandaPositionsResponse {
    positions: Vec<OandaPosition>,
}

#[derive(Debug, Serialize)]
struct OandaOrderRequest {
    order: OandaOrderData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OandaOrderData {
    instrument: String,
    units: String,
    #[serde(rename = "type")]
    order_type: String,
    time_in_force: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    price_bound: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_loss_on_fill: Option<OandaStopLoss>,
    #[serde(skip_serializing_if = "Option::is_none")]
    take_profit_on_fill: Option<OandaTakeProfit>,
}

#[derive(Debug, Serialize)]
struct OandaStopLoss {
    price: String,
}

#[derive(Debug, Serialize)]
struct OandaTakeProfit {
    price: String,
}

#[derive(Debug, Deserialize)]
struct OandaCloseResponse {
    #[serde(rename = "longOrderFillTransaction")]
    long_order_fill: Option<OandaOrderFill>,
    #[serde(rename = "shortOrderFillTransaction")]
    short_order_fill: Option<OandaOrderFill>,
}

/// OANDA broker implementation
pub struct OandaBroker {
    client: Client,
    api_token: String,
    account_id: String,
    base_url: String,
    practice_mode: bool,
}

impl OandaBroker {
    /// Create new OANDA broker
    pub fn new(config: &Config) -> Result<Self> {
        let api_token = std::env::var("OANDA_API_TOKEN")
            .or_else(|_| std::env::var("OANDA_ACCESS_TOKEN"))
            .map_err(|_| Error::Config("OANDA_API_TOKEN not set".to_string()))?;

        let account_id = std::env::var("OANDA_ACCOUNT_ID")
            .map_err(|_| Error::Config("OANDA_ACCOUNT_ID not set".to_string()))?;

        let practice_mode = config.trading.trading_mode == crate::TradingMode::Paper;
        let base_url = if practice_mode {
            OANDA_PRACTICE_URL.to_string()
        } else {
            OANDA_LIVE_URL.to_string()
        };

        info!("OANDA broker initialized in {} mode (account: {})",
            if practice_mode { "PRACTICE" } else { "LIVE" }, account_id);

        Ok(Self {
            client: Client::new(),
            api_token,
            account_id,
            base_url,
            practice_mode,
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
            .request(method.clone(), &url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        debug!("OANDA API request: {} {}", method, endpoint);

        let response = request
            .send()
            .await
            .map_err(|e| Error::Execution(format!("OANDA API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::Execution(format!(
                "OANDA API error {}: {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Execution(format!("Failed to parse OANDA response: {}", e)))
    }

    /// Convert symbol to OANDA instrument format
    fn to_oanda_instrument(symbol: &str) -> String {
        // Convert common symbols to OANDA format
        match symbol {
            "XAUUSD" | "GOLD" | "GC=F" | "GLD" => "XAU_USD".to_string(),
            "EURUSD" => "EUR_USD".to_string(),
            "GBPUSD" => "GBP_USD".to_string(),
            _ => {
                // If already in OANDA format, use as-is
                if symbol.contains('_') {
                    symbol.to_string()
                } else {
                    // Try to guess format
                    warn!("Unknown symbol format: {}, using as-is", symbol);
                    symbol.to_string()
                }
            }
        }
    }

    /// Convert quantity to OANDA units (negative for sell)
    fn to_oanda_units(quantity: f64, side: &OrderSide) -> String {
        let units = (quantity * 1.0).round() as i64; // OANDA uses whole units
        match side {
            OrderSide::Buy => units.to_string(),
            OrderSide::Sell => format!("-{}", units),
        }
    }

    /// Convert OANDA units to quantity
    fn from_oanda_units(units: &str) -> f64 {
        units.parse::<f64>().unwrap_or(0.0).abs()
    }
}

#[async_trait]
impl Broker for OandaBroker {
    async fn place_order(&self, order: Order) -> Result<OrderConfirmation> {
        debug!("Placing OANDA order: {:?}", order);

        let instrument = Self::to_oanda_instrument(&order.symbol);
        let units = Self::to_oanda_units(order.quantity, &order.side);

        let (order_type, price) = match &order.order_type {
            OrderType::Market => ("MARKET", None),
            OrderType::Limit { limit_price } => {
                ("LIMIT", Some(limit_price.to_string()))
            }
            OrderType::Stop { stop_price } => {
                ("STOP", Some(stop_price.to_string()))
            }
            OrderType::StopLimit { stop_price, .. } => {
                // OANDA doesn't have stop-limit, use STOP order
                ("STOP", Some(stop_price.to_string()))
            }
        };

        let time_in_force = match order.time_in_force {
            TimeInForce::Day => "DAY",
            TimeInForce::GTC => "GTC",
            TimeInForce::IOC => "IOC",
            TimeInForce::FOK => "FOK",
        };

        let order_data = OandaOrderData {
            instrument: instrument.clone(),
            units,
            order_type: order_type.to_string(),
            time_in_force: time_in_force.to_string(),
            price,
            price_bound: None,
            stop_loss_on_fill: None,
            take_profit_on_fill: None,
        };

        let request = OandaOrderRequest { order: order_data };

        let response: OandaOrderResponse = self
            .request(
                reqwest::Method::POST,
                &format!("/v3/accounts/{}/orders", self.account_id),
                Some(serde_json::to_value(request)?),
            )
            .await?;

        // Extract order details from response
        let (order_id, filled_price, filled_qty, status) = if let Some(fill) = response.order_fill_transaction {
            let qty = Self::from_oanda_units(&fill.units);
            let price = fill.price.parse().unwrap_or(0.0);
            (fill.id, price, qty, OrderStatus::Filled)
        } else if let Some(create) = response.order_create_transaction {
            let price = create.price.as_ref()
                .and_then(|p| p.parse().ok())
                .unwrap_or(0.0);
            (create.id, price, 0.0, OrderStatus::Pending)
        } else {
            return Err(Error::Execution("No transaction in OANDA response".to_string()));
        };

        Ok(OrderConfirmation {
            order_id,
            symbol: instrument,
            filled_qty,
            filled_price,
            status,
            filled_at: Some(Utc::now()),
        })
    }

    async fn cancel_order(&self, order_id: &str) -> Result<()> {
        debug!("Cancelling OANDA order: {}", order_id);

        let _: serde_json::Value = self
            .request(
                reqwest::Method::PUT,
                &format!("/v3/accounts/{}/orders/{}/cancel", self.account_id, order_id),
                None,
            )
            .await?;

        Ok(())
    }

    async fn get_order(&self, order_id: &str) -> Result<OrderConfirmation> {
        #[derive(Deserialize)]
        struct OrderResponse {
            order: OandaOrder,
        }

        let response: OrderResponse = self
            .request(
                reqwest::Method::GET,
                &format!("/v3/accounts/{}/orders/{}", self.account_id, order_id),
                None,
            )
            .await?;

        let order = response.order;
        let status = match order.state.as_str() {
            "FILLED" => OrderStatus::Filled,
            "TRIGGERED" => OrderStatus::PartiallyFilled,
            "CANCELLED" => OrderStatus::Cancelled,
            "PENDING" => OrderStatus::Pending,
            _ => OrderStatus::Pending,
        };

        Ok(OrderConfirmation {
            order_id: order.id,
            symbol: order.instrument,
            filled_qty: Self::from_oanda_units(&order.units),
            filled_price: 0.0,
            status,
            filled_at: None,
        })
    }

    async fn get_positions(&self) -> Result<Vec<BrokerPosition>> {
        let response: OandaPositionsResponse = self
            .request(
                reqwest::Method::GET,
                &format!("/v3/accounts/{}/positions", self.account_id),
                None,
            )
            .await?;

        let mut positions = Vec::new();

        for pos in response.positions {
            // Check long positions
            if let Ok(long_units) = pos.long.units.parse::<f64>() {
                if long_units != 0.0 {
                    let avg_price = pos.long.average_price
                        .as_ref()
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(0.0);
                    let unrealized_pl = pos.long.unrealized_pl.parse().unwrap_or(0.0);

                    positions.push(BrokerPosition {
                        symbol: pos.instrument.clone(),
                        quantity: long_units,
                        avg_entry_price: avg_price,
                        market_value: long_units * avg_price,
                        unrealized_pnl: unrealized_pl,
                        unrealized_pnl_pct: if avg_price > 0.0 {
                            (unrealized_pl / (long_units * avg_price)) * 100.0
                        } else {
                            0.0
                        },
                        side: OrderSide::Buy,
                    });
                }
            }

            // Check short positions
            if let Ok(short_units) = pos.short.units.parse::<f64>() {
                if short_units != 0.0 {
                    let avg_price = pos.short.average_price
                        .as_ref()
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(0.0);
                    let unrealized_pl = pos.short.unrealized_pl.parse().unwrap_or(0.0);

                    positions.push(BrokerPosition {
                        symbol: pos.instrument.clone(),
                        quantity: short_units.abs(),
                        avg_entry_price: avg_price,
                        market_value: short_units.abs() * avg_price,
                        unrealized_pnl: unrealized_pl,
                        unrealized_pnl_pct: if avg_price > 0.0 {
                            (unrealized_pl / (short_units.abs() * avg_price)) * 100.0
                        } else {
                            0.0
                        },
                        side: OrderSide::Sell,
                    });
                }
            }
        }

        Ok(positions)
    }

    async fn get_account(&self) -> Result<AccountInfo> {
        let response: OandaAccount = self
            .request(
                reqwest::Method::GET,
                &format!("/v3/accounts/{}", self.account_id),
                None,
            )
            .await?;

        let account = response.account;

        Ok(AccountInfo {
            account_id: account.id,
            cash: account.balance.parse().unwrap_or(0.0),
            buying_power: account.margin_available.parse().unwrap_or(0.0),
            portfolio_value: account.nav.parse().unwrap_or(0.0),
            equity: account.nav.parse().unwrap_or(0.0),
        })
    }

    async fn get_open_orders(&self) -> Result<Vec<OrderConfirmation>> {
        let response: OandaOrdersResponse = self
            .request(
                reqwest::Method::GET,
                &format!("/v3/accounts/{}/pendingOrders", self.account_id),
                None,
            )
            .await?;

        Ok(response
            .orders
            .into_iter()
            .map(|o| {
                let status = match o.state.as_str() {
                    "FILLED" => OrderStatus::Filled,
                    "TRIGGERED" => OrderStatus::PartiallyFilled,
                    "CANCELLED" => OrderStatus::Cancelled,
                    _ => OrderStatus::Pending,
                };

                OrderConfirmation {
                    order_id: o.id,
                    symbol: o.instrument,
                    filled_qty: Self::from_oanda_units(&o.units),
                    filled_price: 0.0,
                    status,
                    filled_at: None,
                }
            })
            .collect())
    }

    async fn close_position(&self, symbol: &str) -> Result<OrderConfirmation> {
        debug!("Closing OANDA position: {}", symbol);

        let instrument = Self::to_oanda_instrument(symbol);

        #[derive(Serialize)]
        struct CloseRequest {
            #[serde(rename = "longUnits")]
            long_units: String,
            #[serde(rename = "shortUnits")]
            short_units: String,
        }

        let request = CloseRequest {
            long_units: "ALL".to_string(),
            short_units: "ALL".to_string(),
        };

        let response: OandaCloseResponse = self
            .request(
                reqwest::Method::PUT,
                &format!("/v3/accounts/{}/positions/{}/close", self.account_id, instrument),
                Some(serde_json::to_value(request)?),
            )
            .await?;

        // Use whichever fill transaction is present
        let (order_id, filled_qty, filled_price) = if let Some(fill) = response.long_order_fill {
            (fill.id, Self::from_oanda_units(&fill.units), fill.price.parse().unwrap_or(0.0))
        } else if let Some(fill) = response.short_order_fill {
            (fill.id, Self::from_oanda_units(&fill.units), fill.price.parse().unwrap_or(0.0))
        } else {
            return Err(Error::Execution("No position to close".to_string()));
        };

        Ok(OrderConfirmation {
            order_id,
            symbol: instrument,
            filled_qty,
            filled_price,
            status: OrderStatus::Filled,
            filled_at: Some(Utc::now()),
        })
    }

    async fn is_market_open(&self) -> Result<bool> {
        // OANDA forex market is open 24/5 (Sunday 5pm ET - Friday 5pm ET)
        // For simplicity, check if it's weekday
        let now = Utc::now();
        let weekday = now.weekday();

        // Market closed on Saturday and Sunday (most of the day)
        let is_weekend = matches!(weekday, chrono::Weekday::Sat | chrono::Weekday::Sun);

        // For more precise checking, we could query OANDA's pricing endpoint
        // but this simple check works for most cases
        Ok(!is_weekend)
    }

    fn name(&self) -> &str {
        if self.practice_mode {
            "OANDA (Practice)"
        } else {
            "OANDA (Live)"
        }
    }
}
