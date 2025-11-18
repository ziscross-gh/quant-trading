//! News-enhanced trading strategy
//!
//! Wraps a base strategy and enhances signals with news sentiment analysis.

use super::{Strategy, IndicatorData, MarketData};
use crate::news::NewsManager;
use crate::{Result, Signal, config::Config};
use tracing::{debug, info};

/// News-enhanced strategy wrapper
pub struct NewsEnhancedStrategy<T: Strategy> {
    base_strategy: T,
    news_manager: NewsManager,
    config: Config,
}

impl<T: Strategy> NewsEnhancedStrategy<T> {
    /// Create new news-enhanced strategy
    pub fn new(base_strategy: T, config: Config) -> Self {
        let news_manager = NewsManager::new(config.news.cache_ttl_seconds);

        info!("Created news-enhanced strategy wrapper");
        info!("  News enabled: {}", config.news.enabled);
        info!("  Sentiment weight: {}", config.news.sentiment_weight);
        info!("  Lookback hours: {}", config.news.lookback_hours);

        Self {
            base_strategy,
            news_manager,
            config,
        }
    }

    /// Get the news manager reference
    pub fn news_manager(&mut self) -> &mut NewsManager {
        &mut self.news_manager
    }

    /// Combine technical signal with news sentiment
    fn combine_signals(&self, technical_signal: Signal, news_sentiment: f64) -> Signal {
        if !self.config.news.enabled {
            return technical_signal;
        }

        let weight = self.config.news.sentiment_weight;
        let threshold = self.config.news.sentiment_threshold;

        debug!(
            "Combining signals - Technical: {:?}, Sentiment: {:.3}, Weight: {:.2}",
            technical_signal, news_sentiment, weight
        );

        match technical_signal {
            Signal::Buy => {
                // If news is strongly negative, downgrade to hold
                if news_sentiment < -threshold * 2.0 {
                    info!("Downgrading BUY to HOLD due to negative news sentiment");
                    Signal::Hold
                } else {
                    Signal::Buy
                }
            }
            Signal::Sell => {
                // If news is strongly positive, downgrade to hold
                if news_sentiment > threshold * 2.0 {
                    info!("Downgrading SELL to HOLD due to positive news sentiment");
                    Signal::Hold
                } else {
                    Signal::Sell
                }
            }
            Signal::Hold => {
                // News can trigger signals if strong enough
                if news_sentiment > threshold * 3.0 && weight > 0.4 {
                    info!("Upgrading HOLD to BUY due to strong positive news");
                    Signal::Buy
                } else if news_sentiment < -threshold * 3.0 && weight > 0.4 {
                    info!("Upgrading HOLD to SELL due to strong negative news");
                    Signal::Sell
                } else {
                    Signal::Hold
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl<T: Strategy + Send + Sync> Strategy for NewsEnhancedStrategy<T> {
    fn name(&self) -> &str {
        "News-Enhanced Strategy"
    }

    fn calculate_indicators(&self, data: &MarketData) -> Result<IndicatorData> {
        self.base_strategy.calculate_indicators(data)
    }

    fn generate_signal(&self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
        // Get base technical signal
        let technical_signal = self.base_strategy.generate_signal(data, indicators)?;

        // If news is disabled, return technical signal only
        if !self.config.news.enabled {
            return Ok(technical_signal);
        }

        Ok(technical_signal)
    }

    fn reset(&mut self) {
        self.base_strategy.reset();
    }

    async fn generate_signal_with_news(&mut self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
        // Get base technical signal
        let technical_signal = self.base_strategy.generate_signal(data, indicators)?;

        // If news is disabled, return technical signal only
        if !self.config.news.enabled {
            return Ok(technical_signal);
        }

        // Get news sentiment
        let symbols = vec![
            self.config.trading.symbol.clone(),
            self.config.trading.symbol_alt.clone(),
        ];

        let news_sentiment = self.news_manager
            .get_sentiment(&symbols, self.config.news.lookback_hours)
            .await
            .unwrap_or(0.0); // Default to neutral if news fetch fails

        // Combine signals
        let final_signal = self.combine_signals(technical_signal, news_sentiment);

        info!(
            "Signal generation: Technical={:?}, Sentiment={:.3}, Final={:?}",
            technical_signal, news_sentiment, final_signal
        );

        Ok(final_signal)
    }

    async fn execute(&mut self, data: &MarketData, indicators: &IndicatorData) -> Result<Signal> {
        self.generate_signal_with_news(data, indicators).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategies::GoldMomentumStrategy;

    #[tokio::test]
    async fn test_news_enhanced_strategy() {
        let config = Config::default();
        let base_strategy = GoldMomentumStrategy::new(config.clone());
        let mut enhanced = NewsEnhancedStrategy::new(base_strategy, config);

        // Test that it wraps correctly
        assert_eq!(enhanced.name(), "News-Enhanced Strategy");
    }

    #[test]
    fn test_signal_combination() {
        let config = Config::default();
        let base_strategy = GoldMomentumStrategy::new(config.clone());
        let enhanced = NewsEnhancedStrategy::new(base_strategy, config);

        // Strong negative sentiment should downgrade BUY to HOLD
        let combined = enhanced.combine_signals(Signal::Buy, -0.3);
        assert_eq!(combined, Signal::Hold);

        // Strong positive sentiment should downgrade SELL to HOLD
        let combined = enhanced.combine_signals(Signal::Sell, 0.3);
        assert_eq!(combined, Signal::Hold);

        // Weak sentiment should not change signals
        let combined = enhanced.combine_signals(Signal::Buy, 0.02);
        assert_eq!(combined, Signal::Buy);
    }
}
