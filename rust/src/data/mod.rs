//! Data fetching and management module

use crate::{Candle, Error, MarketData, Result};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub mod cache;

pub use cache::DataCache;

/// Yahoo Finance API response structure
#[derive(Debug, Deserialize)]
struct YahooResponse {
    chart: ChartData,
}

#[derive(Debug, Deserialize)]
struct ChartData {
    result: Vec<ChartResult>,
}

#[derive(Debug, Deserialize)]
struct ChartResult {
    timestamp: Vec<i64>,
    indicators: Indicators,
}

#[derive(Debug, Deserialize)]
struct Indicators {
    quote: Vec<Quote>,
}

#[derive(Debug, Deserialize)]
struct Quote {
    open: Vec<Option<f64>>,
    high: Vec<Option<f64>>,
    low: Vec<Option<f64>>,
    close: Vec<Option<f64>>,
    volume: Vec<Option<f64>>,
}

/// Data fetcher for Gold/USD prices
pub struct DataFetcher {
    symbol: String,
    client: Client,
    cache: Option<DataCache>,
}

impl DataFetcher {
    /// Create a new data fetcher
    pub fn new(symbol: String, cache_enabled: bool, cache_dir: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        let cache = if cache_enabled {
            Some(DataCache::new(cache_dir))
        } else {
            None
        };

        info!("DataFetcher initialized for symbol: {}", symbol);

        Self {
            symbol,
            client,
            cache,
        }
    }

    /// Fetch historical market data
    pub async fn fetch_historical(
        &self,
        start_date: &str,
        end_date: &str,
        interval: &str,
    ) -> Result<MarketData> {
        info!(
            "Fetching historical data from {} to {}, interval: {}",
            start_date, end_date, interval
        );

        // Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(data) = cache.load(&self.symbol, start_date, end_date, interval)? {
                info!("Loaded data from cache");
                return Ok(data);
            }
        }

        // Fetch from API
        let data = self.fetch_from_yahoo(start_date, end_date, interval).await?;

        // Cache the data
        if let Some(ref cache) = self.cache {
            cache.save(&self.symbol, start_date, end_date, interval, &data)?;
        }

        info!("Fetched {} data points", data.len());
        Ok(data)
    }

    /// Fetch recent/live data
    pub async fn fetch_live(&self, period: &str, interval: &str) -> Result<MarketData> {
        debug!("Fetching live data, period: {}, interval: {}", period, interval);

        let end = Utc::now();
        let start = match period {
            "1d" => end - Duration::days(1),
            "5d" => end - Duration::days(5),
            "1mo" => end - Duration::days(30),
            "3mo" => end - Duration::days(90),
            "6mo" => end - Duration::days(180),
            "1y" => end - Duration::days(365),
            _ => end - Duration::days(1),
        };

        let start_str = start.format("%Y-%m-%d").to_string();
        let end_str = end.format("%Y-%m-%d").to_string();

        self.fetch_from_yahoo(&start_str, &end_str, interval).await
    }

    /// Get current price
    pub async fn get_current_price(&self) -> Result<(f64, DateTime<Utc>)> {
        let data = self.fetch_live("1d", "1m").await?;

        if let Some(candle) = data.last() {
            debug!("Current price: ${:.2} at {}", candle.close, candle.timestamp);
            Ok((candle.close, candle.timestamp))
        } else {
            Err(Error::InvalidData("No current price data available".to_string()))
        }
    }

    /// Fetch data from Yahoo Finance API
    async fn fetch_from_yahoo(
        &self,
        start_date: &str,
        end_date: &str,
        interval: &str,
    ) -> Result<MarketData> {
        // Parse dates to timestamps
        let start_ts = self.parse_date(start_date)?;
        let end_ts = self.parse_date(end_date)?;

        // Build URL
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?period1={}&period2={}&interval={}",
            self.symbol, start_ts, end_ts, interval
        );

        // Fetch data
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::DataFetch(format!("Failed to fetch data: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::DataFetch(format!(
                "API returned error status: {}",
                response.status()
            )));
        }

        let yahoo_data: YahooResponse = response
            .json()
            .await
            .map_err(|e| Error::DataFetch(format!("Failed to parse response: {}", e)))?;

        // Parse response into MarketData
        self.parse_yahoo_response(yahoo_data)
    }

    /// Parse Yahoo Finance API response
    fn parse_yahoo_response(&self, response: YahooResponse) -> Result<MarketData> {
        if response.chart.result.is_empty() {
            return Err(Error::InvalidData("No data in response".to_string()));
        }

        let result = &response.chart.result[0];
        let timestamps = &result.timestamp;
        let quotes = &result.indicators.quote[0];

        let mut candles = Vec::new();

        for (i, &ts) in timestamps.iter().enumerate() {
            // Skip if any value is None
            if let (Some(open), Some(high), Some(low), Some(close), Some(volume)) = (
                quotes.open.get(i).and_then(|&v| v),
                quotes.high.get(i).and_then(|&v| v),
                quotes.low.get(i).and_then(|&v| v),
                quotes.close.get(i).and_then(|&v| v),
                quotes.volume.get(i).and_then(|&v| v),
            ) {
                let timestamp = DateTime::from_timestamp(ts, 0)
                    .ok_or_else(|| Error::InvalidData(format!("Invalid timestamp: {}", ts)))?;

                candles.push(Candle {
                    timestamp,
                    open,
                    high,
                    low,
                    close,
                    volume,
                });
            }
        }

        if candles.is_empty() {
            warn!("No valid data points found for {}", self.symbol);
        }

        Ok(MarketData {
            symbol: self.symbol.clone(),
            candles,
        })
    }

    /// Parse date string to Unix timestamp
    fn parse_date(&self, date_str: &str) -> Result<i64> {
        let naive_date = NaiveDateTime::parse_from_str(&format!("{} 00:00:00", date_str), "%Y-%m-%d %H:%M:%S")
            .map_err(|e| Error::Parse(format!("Invalid date format: {}", e)))?;

        Ok(naive_date.and_utc().timestamp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_data_fetcher_creation() {
        let fetcher = DataFetcher::new("GC=F".to_string(), false, "data/raw".to_string());
        assert_eq!(fetcher.symbol, "GC=F");
    }

    #[test]
    fn test_date_parsing() {
        let fetcher = DataFetcher::new("GC=F".to_string(), false, "data/raw".to_string());
        let ts = fetcher.parse_date("2024-01-01").unwrap();
        assert!(ts > 0);
    }
}
