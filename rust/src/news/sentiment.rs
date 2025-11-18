//! Sentiment analysis for news articles
//!
//! Uses keyword-based sentiment analysis with financial markets focus.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sentiment score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentScore {
    /// Compound score (-1.0 to 1.0)
    pub compound: f64,
    /// Positive score (0.0 to 1.0)
    pub positive: f64,
    /// Negative score (0.0 to 1.0)
    pub negative: f64,
    /// Neutral score (0.0 to 1.0)
    pub neutral: f64,
}

/// Sentiment analyzer
pub struct SentimentAnalyzer {
    positive_words: HashMap<String, f64>,
    negative_words: HashMap<String, f64>,
    gold_positive_words: HashMap<String, f64>,
    gold_negative_words: HashMap<String, f64>,
}

impl SentimentAnalyzer {
    /// Create new sentiment analyzer
    pub fn new() -> Self {
        let mut analyzer = Self {
            positive_words: HashMap::new(),
            negative_words: HashMap::new(),
            gold_positive_words: HashMap::new(),
            gold_negative_words: HashMap::new(),
        };

        analyzer.load_general_lexicon();
        analyzer.load_gold_specific_lexicon();
        analyzer
    }

    /// Load general financial sentiment words
    fn load_general_lexicon(&mut self) {
        // Positive words (weight: 0.5 - 2.0)
        let positive = vec![
            ("surge", 1.8), ("soar", 1.8), ("rally", 1.5), ("gain", 1.2),
            ("rise", 1.0), ("increase", 1.0), ("up", 0.8), ("strong", 1.2),
            ("bullish", 1.5), ("profit", 1.3), ("growth", 1.2), ("boost", 1.3),
            ("jump", 1.5), ("spike", 1.6), ("climb", 1.2), ("advance", 1.0),
            ("outperform", 1.4), ("beat", 1.2), ("positive", 1.0), ("optimistic", 1.2),
            ("recovery", 1.3), ("strength", 1.2), ("momentum", 1.1), ("breakout", 1.5),
            ("record", 1.6), ("high", 1.2), ("improve", 1.1), ("upgrade", 1.4),
            ("buy", 1.3), ("accumulate", 1.2), ("recommend", 1.0), ("favorable", 1.1),
        ];

        // Negative words (weight: -0.5 to -2.0)
        let negative = vec![
            ("plunge", -1.8), ("crash", -2.0), ("fall", -1.2), ("decline", -1.2),
            ("drop", -1.2), ("decrease", -1.0), ("down", -0.8), ("weak", -1.2),
            ("bearish", -1.5), ("loss", -1.3), ("recession", -1.8), ("risk", -0.8),
            ("slump", -1.5), ("tumble", -1.6), ("slide", -1.3), ("retreat", -1.0),
            ("underperform", -1.4), ("miss", -1.2), ("negative", -1.0), ("pessimistic", -1.2),
            ("concern", -0.9), ("worry", -1.0), ("fear", -1.3), ("uncertainty", -1.0),
            ("low", -1.2), ("worst", -1.6), ("deteriorate", -1.4), ("downgrade", -1.5),
            ("sell", -1.3), ("dump", -1.6), ("avoid", -1.2), ("warning", -1.3),
        ];

        for (word, weight) in positive {
            self.positive_words.insert(word.to_string(), weight);
        }

        for (word, weight) in negative {
            self.negative_words.insert(word.to_string(), (weight as f64).abs());
        }
    }

    /// Load gold-specific sentiment words
    fn load_gold_specific_lexicon(&mut self) {
        // Gold-specific positive words
        let gold_positive = vec![
            ("safe-haven", 1.5), ("safe haven", 1.5), ("inflation hedge", 1.4),
            ("hedge", 1.2), ("demand", 1.1), ("shortage", 1.3), ("reserve", 1.0),
            ("central bank buying", 1.6), ("jewelry demand", 1.1), ("mine closure", 1.2),
            ("geopolitical tension", 1.3), ("dollar weakness", 1.4), ("weaker dollar", 1.4),
            ("inflation", 1.3), ("rising inflation", 1.5), ("haven", 1.2),
        ];

        // Gold-specific negative words
        let gold_negative = vec![
            ("strong dollar", -1.4), ("dollar strength", -1.4), ("rate hike", -1.3),
            ("rising rates", -1.3), ("higher rates", -1.3), ("oversupply", -1.2),
            ("mine production", -0.8), ("selling pressure", -1.3), ("profit taking", -0.9),
            ("risk appetite", -1.0), ("risk-on", -1.1), ("deflation", -1.0),
        ];

        for (phrase, weight) in gold_positive {
            self.gold_positive_words.insert(phrase.to_string(), weight);
        }

        for (phrase, weight) in gold_negative {
            self.gold_negative_words.insert(phrase.to_string(), (weight as f64).abs());
        }
    }

    /// Analyze text sentiment
    pub fn analyze(&self, text: &str) -> SentimentScore {
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();

        let mut positive_score = 0.0;
        let mut negative_score = 0.0;
        let mut word_count = 0;

        // Check general positive words
        for (word, weight) in &self.positive_words {
            if text_lower.contains(word) {
                positive_score += weight;
                word_count += 1;
            }
        }

        // Check general negative words
        for (word, weight) in &self.negative_words {
            if text_lower.contains(word) {
                negative_score += weight;
                word_count += 1;
            }
        }

        // Check gold-specific positive phrases (higher weight)
        for (phrase, weight) in &self.gold_positive_words {
            if text_lower.contains(phrase) {
                positive_score += weight * 1.5; // Boost gold-specific
                word_count += 1;
            }
        }

        // Check gold-specific negative phrases (higher weight)
        for (phrase, weight) in &self.gold_negative_words {
            if text_lower.contains(phrase) {
                negative_score += weight * 1.5; // Boost gold-specific
                word_count += 1;
            }
        }

        // Normalize scores
        let total = positive_score + negative_score;
        let (pos_norm, neg_norm) = if total > 0.0 {
            (positive_score / total, negative_score / total)
        } else {
            (0.0, 0.0)
        };

        let neutral_norm = 1.0 - pos_norm - neg_norm;

        // Calculate compound score (-1.0 to 1.0)
        let compound = if total > 0.0 {
            let raw = (positive_score - negative_score) / (total + 1.0);
            // Apply sigmoid-like normalization
            raw.tanh()
        } else {
            0.0
        };

        SentimentScore {
            compound,
            positive: pos_norm,
            negative: neg_norm,
            neutral: neutral_norm.max(0.0),
        }
    }
}

impl Default for SentimentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive_sentiment() {
        let analyzer = SentimentAnalyzer::new();
        let score = analyzer.analyze("Gold prices surge on strong demand and inflation hedge buying");

        assert!(score.compound > 0.0);
        assert!(score.positive > score.negative);
    }

    #[test]
    fn test_negative_sentiment() {
        let analyzer = SentimentAnalyzer::new();
        let score = analyzer.analyze("Gold plunges on strong dollar and rising interest rates");

        assert!(score.compound < 0.0);
        assert!(score.negative > score.positive);
    }

    #[test]
    fn test_neutral_sentiment() {
        let analyzer = SentimentAnalyzer::new();
        let score = analyzer.analyze("The market opened today");

        assert!(score.compound.abs() < 0.1);
    }

    #[test]
    fn test_gold_specific() {
        let analyzer = SentimentAnalyzer::new();
        let score = analyzer.analyze("Gold serves as safe-haven amid geopolitical tension");

        assert!(score.compound > 0.3);
    }
}
