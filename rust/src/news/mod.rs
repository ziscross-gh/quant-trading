//! News management module
//!
//! Fetches and analyzes news from Yahoo Finance and other sources.
//! Provides sentiment analysis to inform trading decisions.

use crate::{Error, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub mod calendar;
pub mod sentiment;
pub mod yahoo_finance;

pub use calendar::{CalendarConfig, EconomicCalendar, EconomicEvent, EventImpact, EventType};
pub use sentiment::{SentimentAnalyzer, SentimentScore};
pub use yahoo_finance::YahooNewsProvider;

/// News article
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsArticle {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub url: String,
    pub source: String,
    pub published_at: DateTime<Utc>,
    pub fetched_at: DateTime<Utc>,
    pub symbols: Vec<String>,
    pub sentiment: Option<SentimentScore>,
}

/// News provider trait
#[async_trait::async_trait]
pub trait NewsProvider: Send + Sync {
    /// Fetch latest news for given symbols
    async fn fetch_news(&self, symbols: &[String], limit: usize) -> Result<Vec<NewsArticle>>;

    /// Get provider name
    fn name(&self) -> &str;
}

/// News manager - aggregates news from multiple providers
pub struct NewsManager {
    providers: Vec<Box<dyn NewsProvider>>,
    sentiment_analyzer: SentimentAnalyzer,
    cache: HashMap<String, Vec<NewsArticle>>,
    cache_ttl_seconds: i64,
}

impl NewsManager {
    /// Create new news manager
    pub fn new(cache_ttl_seconds: i64) -> Self {
        let yahoo_provider = Box::new(YahooNewsProvider::new());

        Self {
            providers: vec![yahoo_provider],
            sentiment_analyzer: SentimentAnalyzer::new(),
            cache: HashMap::new(),
            cache_ttl_seconds,
        }
    }

    /// Add a news provider
    pub fn add_provider(&mut self, provider: Box<dyn NewsProvider>) {
        info!("Adding news provider: {}", provider.name());
        self.providers.push(provider);
    }

    /// Fetch and analyze news for symbols
    pub async fn get_news(&mut self, symbols: &[String], limit: usize) -> Result<Vec<NewsArticle>> {
        let cache_key = format!("{:?}", symbols);

        // Check cache
        if let Some(cached) = self.cache.get(&cache_key) {
            if !cached.is_empty() {
                let age = Utc::now()
                    .signed_duration_since(cached[0].fetched_at)
                    .num_seconds();

                if age < self.cache_ttl_seconds {
                    debug!("Returning cached news (age: {}s)", age);
                    return Ok(cached.clone());
                }
            }
        }

        // Fetch from all providers
        let mut all_articles = Vec::new();

        for provider in &self.providers {
            match provider.fetch_news(symbols, limit).await {
                Ok(articles) => {
                    info!("Fetched {} articles from {}", articles.len(), provider.name());
                    all_articles.extend(articles);
                }
                Err(e) => {
                    warn!("Failed to fetch from {}: {}", provider.name(), e);
                }
            }
        }

        // Deduplicate by URL
        let mut seen_urls = std::collections::HashSet::new();
        all_articles.retain(|article| seen_urls.insert(article.url.clone()));

        // Analyze sentiment
        for article in &mut all_articles {
            let text = format!("{} {}", article.title, article.summary);
            article.sentiment = Some(self.sentiment_analyzer.analyze(&text));
        }

        // Sort by published date (newest first)
        all_articles.sort_by(|a, b| b.published_at.cmp(&a.published_at));

        // Limit results
        all_articles.truncate(limit);

        // Update cache
        self.cache.insert(cache_key, all_articles.clone());

        Ok(all_articles)
    }

    /// Get overall sentiment for symbols
    pub async fn get_sentiment(&mut self, symbols: &[String], lookback_hours: i64) -> Result<f64> {
        let articles = self.get_news(symbols, 50).await?;

        let cutoff = Utc::now() - chrono::Duration::hours(lookback_hours);
        let recent_articles: Vec<_> = articles
            .iter()
            .filter(|a| a.published_at > cutoff)
            .collect();

        if recent_articles.is_empty() {
            return Ok(0.0); // Neutral if no news
        }

        let total_score: f64 = recent_articles
            .iter()
            .filter_map(|a| a.sentiment.as_ref())
            .map(|s| s.compound)
            .sum();

        let avg_sentiment = total_score / recent_articles.len() as f64;

        info!(
            "Overall sentiment for {:?}: {:.3} ({} articles in last {}h)",
            symbols, avg_sentiment, recent_articles.len(), lookback_hours
        );

        Ok(avg_sentiment)
    }

    /// Get news summary statistics
    pub fn get_stats(&self, articles: &[NewsArticle]) -> NewsSummary {
        if articles.is_empty() {
            return NewsSummary::default();
        }

        let positive = articles
            .iter()
            .filter(|a| a.sentiment.as_ref().map_or(false, |s| s.compound > 0.05))
            .count();

        let negative = articles
            .iter()
            .filter(|a| a.sentiment.as_ref().map_or(false, |s| s.compound < -0.05))
            .count();

        let neutral = articles.len() - positive - negative;

        let avg_sentiment = articles
            .iter()
            .filter_map(|a| a.sentiment.as_ref())
            .map(|s| s.compound)
            .sum::<f64>()
            / articles.len() as f64;

        NewsSummary {
            total_articles: articles.len(),
            positive_count: positive,
            negative_count: negative,
            neutral_count: neutral,
            average_sentiment: avg_sentiment,
        }
    }
}

/// News summary statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NewsSummary {
    pub total_articles: usize,
    pub positive_count: usize,
    pub negative_count: usize,
    pub neutral_count: usize,
    pub average_sentiment: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_news_summary() {
        let articles = vec![
            NewsArticle {
                id: "1".to_string(),
                title: "Gold prices surge".to_string(),
                summary: "Great news".to_string(),
                url: "http://example.com/1".to_string(),
                source: "Test".to_string(),
                published_at: Utc::now(),
                fetched_at: Utc::now(),
                symbols: vec!["GOLD".to_string()],
                sentiment: Some(SentimentScore {
                    compound: 0.8,
                    positive: 0.8,
                    negative: 0.0,
                    neutral: 0.2,
                }),
            },
        ];

        let manager = NewsManager::new(300);
        let summary = manager.get_stats(&articles);

        assert_eq!(summary.total_articles, 1);
        assert_eq!(summary.positive_count, 1);
    }
}
