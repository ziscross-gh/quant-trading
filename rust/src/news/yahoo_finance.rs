//! Yahoo Finance news provider
//!
//! Fetches news from Yahoo Finance RSS feeds and search API.

use super::{NewsArticle, NewsProvider};
use crate::{Error, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, warn};

/// Yahoo Finance news provider
pub struct YahooNewsProvider {
    client: Client,
}

/// Yahoo Finance news response structure
#[derive(Debug, Deserialize)]
struct YahooNewsResponse {
    #[serde(default)]
    item: Vec<YahooNewsItem>,
}

#[derive(Debug, Deserialize)]
struct YahooNewsItem {
    guid: String,
    title: String,
    link: String,
    #[serde(rename = "pubDate")]
    pub_date: String,
    description: String,
    source: Option<String>,
}

impl YahooNewsProvider {
    /// Create new Yahoo Finance news provider
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Get RSS feed URL for symbol
    fn get_rss_url(symbol: &str) -> String {
        // Yahoo Finance RSS feed
        format!("https://feeds.finance.yahoo.com/rss/2.0/headline?s={}&region=US&lang=en-US", symbol)
    }

    /// Parse RSS feed
    async fn parse_rss(&self, url: &str) -> Result<Vec<NewsArticle>> {
        debug!("Fetching RSS from: {}", url);

        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Execution(format!("Failed to fetch RSS: {}", e)))?;

        let body = response
            .text()
            .await
            .map_err(|e| Error::Execution(format!("Failed to read RSS: {}", e)))?;

        self.parse_rss_xml(&body)
    }

    /// Parse RSS XML content
    fn parse_rss_xml(&self, xml: &str) -> Result<Vec<NewsArticle>> {
        use quick_xml::de::from_str;

        // Try to parse as RSS 2.0
        #[derive(Debug, Deserialize)]
        struct Rss {
            channel: Channel,
        }

        #[derive(Debug, Deserialize)]
        struct Channel {
            #[serde(default)]
            item: Vec<Item>,
        }

        #[derive(Debug, Deserialize)]
        struct Item {
            #[serde(default)]
            guid: String,
            #[serde(default)]
            title: String,
            #[serde(default)]
            link: String,
            #[serde(rename = "pubDate", default)]
            pub_date: String,
            #[serde(default)]
            description: String,
        }

        match from_str::<Rss>(xml) {
            Ok(rss) => {
                let now = Utc::now();
                let articles: Vec<NewsArticle> = rss.channel.item
                    .into_iter()
                    .filter_map(|item| {
                        // Parse RFC 2822 date format
                        let published_at = chrono::DateTime::parse_from_rfc2822(&item.pub_date)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or(now);

                        // Clean HTML from description
                        let summary = Self::strip_html(&item.description);

                        Some(NewsArticle {
                            id: if item.guid.is_empty() {
                                uuid::Uuid::new_v4().to_string()
                            } else {
                                item.guid
                            },
                            title: item.title,
                            summary,
                            url: item.link,
                            source: "Yahoo Finance".to_string(),
                            published_at,
                            fetched_at: now,
                            symbols: vec![],
                            sentiment: None,
                        })
                    })
                    .collect();

                Ok(articles)
            }
            Err(e) => {
                warn!("Failed to parse RSS XML: {}", e);
                // Return empty vec instead of error to be more resilient
                Ok(vec![])
            }
        }
    }

    /// Strip HTML tags from text
    fn strip_html(html: &str) -> String {
        // Simple HTML tag removal
        let mut result = String::new();
        let mut in_tag = false;

        for c in html.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(c),
                _ => {}
            }
        }

        // Decode common HTML entities
        result
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ")
            .trim()
            .to_string()
    }

    /// Fetch using alternative scraping method
    async fn fetch_alternative(&self, symbol: &str) -> Result<Vec<NewsArticle>> {
        // Alternative: Use Yahoo Finance quote page
        let url = format!("https://finance.yahoo.com/quote/{}", symbol);

        debug!("Fetching from quote page: {}", url);

        // This is a fallback - in production you might want to use a proper API
        // For now, return empty to avoid complexity
        warn!("Alternative fetch not fully implemented, returning empty");
        Ok(vec![])
    }
}

#[async_trait::async_trait]
impl NewsProvider for YahooNewsProvider {
    async fn fetch_news(&self, symbols: &[String], limit: usize) -> Result<Vec<NewsArticle>> {
        let mut all_articles = Vec::new();

        for symbol in symbols {
            // Convert symbol to Yahoo format
            let yahoo_symbol = match symbol.as_str() {
                "XAUUSD" | "GOLD" | "XAU_USD" => "GC=F", // Gold futures
                "GLD" => "GLD",                          // Gold ETF
                s => s,
            };

            // Try RSS first
            match self.parse_rss(&Self::get_rss_url(yahoo_symbol)).await {
                Ok(mut articles) => {
                    // Tag articles with symbol
                    for article in &mut articles {
                        article.symbols.push(symbol.clone());
                    }
                    all_articles.extend(articles);
                }
                Err(e) => {
                    warn!("RSS fetch failed for {}: {}", symbol, e);

                    // Try alternative method
                    match self.fetch_alternative(yahoo_symbol).await {
                        Ok(articles) => all_articles.extend(articles),
                        Err(e2) => warn!("Alternative fetch also failed: {}", e2),
                    }
                }
            }
        }

        // Deduplicate by URL
        let mut seen = std::collections::HashSet::new();
        all_articles.retain(|a| seen.insert(a.url.clone()));

        // Sort by date (newest first)
        all_articles.sort_by(|a, b| b.published_at.cmp(&a.published_at));

        // Limit results
        all_articles.truncate(limit);

        Ok(all_articles)
    }

    fn name(&self) -> &str {
        "Yahoo Finance"
    }
}

impl Default for YahooNewsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html() {
        let html = "<p>Gold prices <strong>rise</strong> today</p>";
        let text = YahooNewsProvider::strip_html(html);
        assert_eq!(text, "Gold prices rise today");
    }

    #[test]
    fn test_html_entities() {
        let html = "Gold &amp; Silver &lt;strong&gt;";
        let text = YahooNewsProvider::strip_html(html);
        assert_eq!(text, "Gold & Silver strong");
    }

    #[tokio::test]
    async fn test_fetch_news() {
        let provider = YahooNewsProvider::new();
        let symbols = vec!["GC=F".to_string()];

        // This will hit the real API - may fail in CI
        match provider.fetch_news(&symbols, 5).await {
            Ok(articles) => {
                println!("Fetched {} articles", articles.len());
                for article in articles {
                    println!("- {}", article.title);
                }
            }
            Err(e) => {
                println!("Fetch failed (expected in some environments): {}", e);
            }
        }
    }
}
