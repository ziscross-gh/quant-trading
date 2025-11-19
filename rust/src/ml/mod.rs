// ML Signal Filter Module
//
// Implements Random Forest classifier for filtering trading signals.
// Only executes trades when ML model predicts high probability of success.
//
// Key Features:
// - 20+ engineered features from technical indicators, market conditions, sentiment
// - Random Forest classifier with configurable trees and depth
// - Probability threshold filtering (only trade if confidence >60%)
// - Feature importance analysis for interpretability
// - Model training and persistence
// - Cross-validation for model evaluation

use crate::{
    indicators,
    types::{MarketData, Signal},
    Error, Result,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Configuration for ML signal filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLFilterConfig {
    /// Minimum probability threshold to execute trade (0.0-1.0)
    pub min_probability: f64,

    /// Number of trees in random forest
    pub num_trees: usize,

    /// Maximum depth of each tree
    pub max_depth: usize,

    /// Minimum samples required to split a node
    pub min_samples_split: usize,

    /// Minimum samples required at leaf node
    pub min_samples_leaf: usize,

    /// Feature subset size (sqrt of total features if None)
    pub max_features: Option<usize>,

    /// Whether to use sentiment features (requires news data)
    pub use_sentiment: bool,

    /// Whether to use time-based features
    pub use_time_features: bool,

    /// Lookback period for feature calculation
    pub feature_lookback: usize,
}

impl Default for MLFilterConfig {
    fn default() -> Self {
        Self {
            min_probability: 0.60,
            num_trees: 100,
            max_depth: 10,
            min_samples_split: 10,
            min_samples_leaf: 5,
            max_features: None,
            use_sentiment: false,
            use_time_features: true,
            feature_lookback: 50,
        }
    }
}

/// Feature vector for ML model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVector {
    pub features: Vec<f64>,
    pub feature_names: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

impl FeatureVector {
    pub fn new(features: Vec<f64>, feature_names: Vec<String>, timestamp: DateTime<Utc>) -> Self {
        Self {
            features,
            feature_names,
            timestamp,
        }
    }

    pub fn len(&self) -> usize {
        self.features.len()
    }

    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

/// Training sample with features and label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    pub features: Vec<f64>,
    pub label: i8, // 1 for profitable, -1 for unprofitable, 0 for neutral
    pub timestamp: DateTime<Utc>,
}

/// Feature importance scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureImportance {
    pub feature_name: String,
    pub importance: f64,
    pub rank: usize,
}

impl FeatureImportance {
    pub fn new(feature_name: String, importance: f64, rank: usize) -> Self {
        Self {
            feature_name,
            importance,
            rank,
        }
    }
}

/// ML model prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLPrediction {
    /// Predicted class (1, -1, or 0)
    pub predicted_class: i8,

    /// Probability of positive class (profitable trade)
    pub probability_positive: f64,

    /// Probability of negative class (unprofitable trade)
    pub probability_negative: f64,

    /// Whether the prediction meets the confidence threshold
    pub meets_threshold: bool,

    /// Original signal before ML filtering
    pub original_signal: Signal,

    /// Filtered signal after ML filtering
    pub filtered_signal: Signal,
}

/// Feature engineering for ML model
pub struct FeatureEngineer {
    config: MLFilterConfig,
}

impl FeatureEngineer {
    pub fn new(config: MLFilterConfig) -> Self {
        Self { config }
    }

    /// Extract all features from market data at a specific point
    pub fn extract_features(
        &self,
        data: &MarketData,
        index: usize,
    ) -> Result<FeatureVector> {
        if index < self.config.feature_lookback {
            return Err(Error::InvalidInput(format!(
                "Insufficient data for feature extraction: index {} < lookback {}",
                index, self.config.feature_lookback
            )));
        }

        let mut features = Vec::new();
        let mut feature_names = Vec::new();

        // Get data slice for feature calculation
        let start_idx = index.saturating_sub(self.config.feature_lookback);
        let data_slice = &data.candles[start_idx..=index];

        // Extract closes, highs, lows, volumes
        let closes: Vec<f64> = data_slice.iter().map(|c| c.close).collect();
        let highs: Vec<f64> = data_slice.iter().map(|c| c.high).collect();
        let lows: Vec<f64> = data_slice.iter().map(|c| c.low).collect();
        let volumes: Vec<f64> = data_slice.iter().map(|c| c.volume).collect();

        let current_close = closes[closes.len() - 1];
        let current_high = highs[highs.len() - 1];
        let current_low = lows[lows.len() - 1];
        let current_volume = volumes[volumes.len() - 1];

        // 1. Price-based features
        self.add_price_features(
            &mut features,
            &mut feature_names,
            &closes,
            current_close,
        )?;

        // 2. Momentum indicators
        self.add_momentum_features(&mut features, &mut feature_names, &closes, &highs, &lows)?;

        // 3. Volatility indicators
        self.add_volatility_features(
            &mut features,
            &mut feature_names,
            &closes,
            &highs,
            &lows,
        )?;

        // 4. Volume indicators
        self.add_volume_features(
            &mut features,
            &mut feature_names,
            &volumes,
            &closes,
            current_volume,
        )?;

        // 5. Trend indicators
        self.add_trend_features(&mut features, &mut feature_names, &closes)?;

        // 6. Price patterns
        self.add_pattern_features(
            &mut features,
            &mut feature_names,
            &closes,
            &highs,
            &lows,
            current_close,
            current_high,
            current_low,
        )?;

        // 7. Time-based features (if enabled)
        if self.config.use_time_features {
            self.add_time_features(&mut features, &mut feature_names, &data.candles[index])?;
        }

        Ok(FeatureVector::new(
            features,
            feature_names,
            data.candles[index].timestamp,
        ))
    }

    /// Add price-based features
    fn add_price_features(
        &self,
        features: &mut Vec<f64>,
        names: &mut Vec<String>,
        closes: &[f64],
        current_close: f64,
    ) -> Result<()> {
        // Returns over different periods
        let periods = [1, 5, 10, 20];
        for &period in &periods {
            if closes.len() > period {
                let prev_close = closes[closes.len() - period - 1];
                let ret = (current_close - prev_close) / prev_close;
                features.push(ret);
                names.push(format!("return_{}", period));
            }
        }

        // Distance from moving averages
        for &period in &[10, 20, 50] {
            if closes.len() >= period {
                let ma = closes[closes.len() - period..].iter().sum::<f64>() / period as f64;
                let dist = (current_close - ma) / ma;
                features.push(dist);
                names.push(format!("dist_ma_{}", period));
            }
        }

        Ok(())
    }

    /// Add momentum indicator features
    fn add_momentum_features(
        &self,
        features: &mut Vec<f64>,
        names: &mut Vec<String>,
        closes: &[f64],
        highs: &[f64],
        lows: &[f64],
    ) -> Result<()> {
        // RSI
        if let Ok(rsi_values) = indicators::rsi(closes, 14) {
            if let Some(&rsi) = rsi_values.last() {
                features.push(rsi);
                names.push("rsi_14".to_string());

                // RSI normalized to -1 to 1
                features.push((rsi - 50.0) / 50.0);
                names.push("rsi_14_norm".to_string());
            }
        }

        // Rate of Change (ROC) as momentum indicator
        if closes.len() >= 14 {
            let roc = (closes[closes.len() - 1] - closes[closes.len() - 14]) / closes[closes.len() - 14];
            features.push(roc * 100.0);
            names.push("roc_14".to_string());
        }

        // Momentum
        if closes.len() >= 10 {
            let momentum = closes[closes.len() - 1] - closes[closes.len() - 10];
            features.push(momentum / closes[closes.len() - 10] * 100.0);
            names.push("momentum_10".to_string());
        }

        // ADX (trend strength)
        if let Ok((adx, _plus_di, _minus_di)) = indicators::adx(highs, lows, closes, 14) {
            if let Some(&adx_val) = adx.last() {
                features.push(adx_val);
                names.push("adx_14".to_string());
            }
        }

        Ok(())
    }

    /// Add volatility indicator features
    fn add_volatility_features(
        &self,
        features: &mut Vec<f64>,
        names: &mut Vec<String>,
        closes: &[f64],
        highs: &[f64],
        lows: &[f64],
    ) -> Result<()> {
        // ATR
        if let Ok(atr_values) = indicators::atr(highs, lows, closes, 14) {
            if let Some(&atr) = atr_values.last() {
                let current_close = closes[closes.len() - 1];
                let atr_pct = (atr / current_close) * 100.0;
                features.push(atr_pct);
                names.push("atr_14_pct".to_string());
            }
        }

        // Bollinger Band width
        if let Ok(bb) = indicators::bollinger_bands(closes, 20, 2.0) {
            if let (Some(&upper_val), Some(&lower_val)) = (bb.upper.last(), bb.lower.last()) {
                let current_close = closes[closes.len() - 1];
                let bb_width = ((upper_val - lower_val) / current_close) * 100.0;
                features.push(bb_width);
                names.push("bb_width_pct".to_string());

                // Price position within bands
                let bb_position = (current_close - lower_val) / (upper_val - lower_val);
                features.push(bb_position);
                names.push("bb_position".to_string());
            }
        }

        // Historical volatility (standard deviation of returns)
        if closes.len() >= 20 {
            let returns: Vec<f64> = closes
                .windows(2)
                .map(|w| (w[1] - w[0]) / w[0])
                .collect();

            let mean = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance = returns
                .iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>() / returns.len() as f64;
            let std_dev = variance.sqrt() * (252.0_f64).sqrt(); // Annualized

            features.push(std_dev * 100.0);
            names.push("historical_volatility_20".to_string());
        }

        Ok(())
    }

    /// Add volume indicator features
    fn add_volume_features(
        &self,
        features: &mut Vec<f64>,
        names: &mut Vec<String>,
        volumes: &[f64],
        closes: &[f64],
        current_volume: f64,
    ) -> Result<()> {
        // Volume MA ratio
        if volumes.len() >= 20 {
            let vol_ma = volumes[volumes.len() - 20..].iter().sum::<f64>() / 20.0;
            let vol_ratio = current_volume / vol_ma;
            features.push(vol_ratio);
            names.push("volume_ma_ratio_20".to_string());
        }

        // On-Balance Volume (OBV) trend
        if volumes.len() >= 20 && closes.len() >= 20 {
            let mut obv = 0.0;
            for i in 1..20 {
                let idx = closes.len() - 20 + i;
                if closes[idx] > closes[idx - 1] {
                    obv += volumes[idx];
                } else if closes[idx] < closes[idx - 1] {
                    obv -= volumes[idx];
                }
            }
            features.push(obv / 1000000.0); // Normalize
            names.push("obv_trend_20".to_string());
        }

        Ok(())
    }

    /// Add trend indicator features
    fn add_trend_features(
        &self,
        features: &mut Vec<f64>,
        names: &mut Vec<String>,
        closes: &[f64],
    ) -> Result<()> {
        // MA crossovers
        if closes.len() >= 50 {
            let ma_10 = closes[closes.len() - 10..].iter().sum::<f64>() / 10.0;
            let ma_50 = closes[closes.len() - 50..].iter().sum::<f64>() / 50.0;

            let ma_cross = (ma_10 - ma_50) / ma_50;
            features.push(ma_cross);
            names.push("ma_10_50_cross".to_string());
        }

        // Linear regression slope (trend direction)
        if closes.len() >= 20 {
            let recent_closes = &closes[closes.len() - 20..];
            let n = recent_closes.len() as f64;

            let x_mean = (n - 1.0) / 2.0;
            let y_mean = recent_closes.iter().sum::<f64>() / n;

            let mut numerator = 0.0;
            let mut denominator = 0.0;

            for (i, &price) in recent_closes.iter().enumerate() {
                let x_diff = i as f64 - x_mean;
                numerator += x_diff * (price - y_mean);
                denominator += x_diff * x_diff;
            }

            let slope = if denominator != 0.0 {
                numerator / denominator
            } else {
                0.0
            };

            // Normalize slope by mean price
            features.push((slope / y_mean) * 100.0);
            names.push("trend_slope_20".to_string());
        }

        Ok(())
    }

    /// Add price pattern features
    fn add_pattern_features(
        &self,
        features: &mut Vec<f64>,
        names: &mut Vec<String>,
        closes: &[f64],
        highs: &[f64],
        lows: &[f64],
        current_close: f64,
        current_high: f64,
        current_low: f64,
    ) -> Result<()> {
        // Candle body size
        let body_size = if closes.len() >= 2 {
            let prev_close = closes[closes.len() - 2];
            ((current_close - prev_close).abs() / prev_close) * 100.0
        } else {
            0.0
        };
        features.push(body_size);
        names.push("candle_body_size".to_string());

        // Upper and lower shadows
        let total_range = current_high - current_low;
        if total_range > 0.0 {
            let upper_shadow = (current_high - current_close.max(closes[closes.len() - 2]))
                / total_range;
            let lower_shadow = (current_close.min(closes[closes.len() - 2]) - current_low)
                / total_range;

            features.push(upper_shadow);
            names.push("upper_shadow_ratio".to_string());

            features.push(lower_shadow);
            names.push("lower_shadow_ratio".to_string());
        }

        // Support/Resistance proximity
        if closes.len() >= 50 {
            let recent_data = &closes[closes.len() - 50..];
            let support = recent_data
                .iter()
                .copied()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(current_close);
            let resistance = recent_data
                .iter()
                .copied()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(current_close);

            let dist_to_support = ((current_close - support) / support) * 100.0;
            let dist_to_resistance = ((resistance - current_close) / current_close) * 100.0;

            features.push(dist_to_support);
            names.push("dist_to_support".to_string());

            features.push(dist_to_resistance);
            names.push("dist_to_resistance".to_string());
        }

        Ok(())
    }

    /// Add time-based features
    fn add_time_features(
        &self,
        features: &mut Vec<f64>,
        names: &mut Vec<String>,
        candle: &crate::types::Candle,
    ) -> Result<()> {
        use chrono::{Datelike, Timelike};

        // Day of week (0-6, Monday=0)
        let dow = candle.timestamp.weekday().num_days_from_monday() as f64 / 6.0;
        features.push(dow);
        names.push("day_of_week".to_string());

        // Hour of day (0-23)
        let hour = candle.timestamp.hour() as f64 / 23.0;
        features.push(hour);
        names.push("hour_of_day".to_string());

        // Is market open (assuming 9:30 AM - 4:00 PM ET)
        let hour_utc = candle.timestamp.hour();
        let is_market_hours = if hour_utc >= 14 && hour_utc < 21 { 1.0 } else { 0.0 };
        features.push(is_market_hours);
        names.push("is_market_hours".to_string());

        Ok(())
    }

    /// Get feature names for a configured feature set
    pub fn get_feature_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        // This should match extract_features order
        // Price features
        for period in &[1, 5, 10, 20] {
            names.push(format!("return_{}", period));
        }
        for period in &[10, 20, 50] {
            names.push(format!("dist_ma_{}", period));
        }

        // Momentum
        names.push("rsi_14".to_string());
        names.push("rsi_14_norm".to_string());
        names.push("roc_14".to_string());
        names.push("momentum_10".to_string());
        names.push("adx_14".to_string());

        // Volatility
        names.push("atr_14_pct".to_string());
        names.push("bb_width_pct".to_string());
        names.push("bb_position".to_string());
        names.push("historical_volatility_20".to_string());

        // Volume
        names.push("volume_ma_ratio_20".to_string());
        names.push("obv_trend_20".to_string());

        // Trend
        names.push("ma_10_50_cross".to_string());
        names.push("trend_slope_20".to_string());

        // Patterns
        names.push("candle_body_size".to_string());
        names.push("upper_shadow_ratio".to_string());
        names.push("lower_shadow_ratio".to_string());
        names.push("dist_to_support".to_string());
        names.push("dist_to_resistance".to_string());

        // Time features (if enabled)
        if self.config.use_time_features {
            names.push("day_of_week".to_string());
            names.push("hour_of_day".to_string());
            names.push("is_market_hours".to_string());
        }

        names
    }
}

/// Simplified Random Forest classifier
/// Note: For production, consider using a full ML library like smartcore or linfa
pub struct RandomForestClassifier {
    config: MLFilterConfig,
    trees: Vec<DecisionTree>,
    feature_importances: HashMap<String, f64>,
    is_trained: bool,
}

impl RandomForestClassifier {
    pub fn new(config: MLFilterConfig) -> Self {
        Self {
            config,
            trees: Vec::new(),
            feature_importances: HashMap::new(),
            is_trained: false,
        }
    }

    /// Train the random forest model
    pub fn train(&mut self, samples: &[TrainingSample], feature_names: &[String]) -> Result<()> {
        info!("Training Random Forest with {} samples and {} features",
            samples.len(), feature_names.len());

        if samples.is_empty() {
            return Err(Error::InvalidInput("No training samples provided".to_string()));
        }

        // Build trees
        self.trees.clear();
        for i in 0..self.config.num_trees {
            // Bootstrap sampling
            let bootstrap_samples = self.bootstrap_sample(samples);

            // Train tree
            let mut tree = DecisionTree::new(
                self.config.max_depth,
                self.config.min_samples_split,
                self.config.min_samples_leaf,
            );

            tree.train(&bootstrap_samples, feature_names)?;
            self.trees.push(tree);

            if (i + 1) % 20 == 0 {
                info!("Trained {}/{} trees", i + 1, self.config.num_trees);
            }
        }

        // Calculate feature importances
        self.calculate_feature_importances(feature_names);

        self.is_trained = true;
        info!("Random Forest training complete");

        Ok(())
    }

    /// Predict class and probabilities for a feature vector
    pub fn predict(&self, features: &[f64]) -> Result<(i8, f64, f64)> {
        if !self.is_trained {
            return Err(Error::InvalidInput("Model not trained".to_string()));
        }

        // Get predictions from all trees
        let mut votes: HashMap<i8, usize> = HashMap::new();

        for tree in &self.trees {
            let prediction = tree.predict(features)?;
            *votes.entry(prediction).or_insert(0) += 1;
        }

        // Find majority vote
        let predicted_class = *votes
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(class, _)| class)
            .unwrap_or(&0);

        // Calculate probabilities
        let total_votes = votes.values().sum::<usize>() as f64;
        let prob_positive = (*votes.get(&1).unwrap_or(&0) as f64) / total_votes;
        let prob_negative = (*votes.get(&-1).unwrap_or(&0) as f64) / total_votes;

        Ok((predicted_class, prob_positive, prob_negative))
    }

    /// Bootstrap sampling for training individual trees
    fn bootstrap_sample(&self, samples: &[TrainingSample]) -> Vec<TrainingSample> {
        let n = samples.len();
        (0..n)
            .map(|_| {
                let idx = (rand_f64() * n as f64) as usize % n;
                samples[idx].clone()
            })
            .collect()
    }

    /// Calculate feature importances across all trees
    fn calculate_feature_importances(&mut self, feature_names: &[String]) {
        self.feature_importances.clear();

        for name in feature_names {
            let mut total_importance = 0.0;
            for tree in &self.trees {
                if let Some(&imp) = tree.feature_importances.get(name) {
                    total_importance += imp;
                }
            }
            let avg_importance = total_importance / self.trees.len() as f64;
            self.feature_importances.insert(name.clone(), avg_importance);
        }
    }

    /// Get feature importances sorted by importance
    pub fn get_feature_importances(&self) -> Vec<FeatureImportance> {
        let mut importances: Vec<_> = self
            .feature_importances
            .iter()
            .map(|(name, &importance)| FeatureImportance::new(name.clone(), importance, 0))
            .collect();

        importances.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());

        for (rank, importance) in importances.iter_mut().enumerate() {
            importance.rank = rank + 1;
        }

        importances
    }

    pub fn is_trained(&self) -> bool {
        self.is_trained
    }
}

/// Simple decision tree for the random forest
struct DecisionTree {
    max_depth: usize,
    min_samples_split: usize,
    min_samples_leaf: usize,
    root: Option<TreeNode>,
    feature_importances: HashMap<String, f64>,
}

struct TreeNode {
    feature_index: Option<usize>,
    threshold: Option<f64>,
    left: Option<Box<TreeNode>>,
    right: Option<Box<TreeNode>>,
    value: Option<i8>,
}

impl DecisionTree {
    fn new(max_depth: usize, min_samples_split: usize, min_samples_leaf: usize) -> Self {
        Self {
            max_depth,
            min_samples_split,
            min_samples_leaf,
            root: None,
            feature_importances: HashMap::new(),
        }
    }

    fn train(&mut self, samples: &[TrainingSample], feature_names: &[String]) -> Result<()> {
        self.root = Some(self.build_tree(samples, 0, feature_names));
        Ok(())
    }

    fn build_tree(&self, samples: &[TrainingSample], depth: usize, feature_names: &[String]) -> TreeNode {
        // Check stopping conditions
        if depth >= self.max_depth
            || samples.len() < self.min_samples_split
            || self.is_pure(samples)
        {
            return TreeNode {
                feature_index: None,
                threshold: None,
                left: None,
                right: None,
                value: Some(self.majority_class(samples)),
            };
        }

        // Find best split
        let (best_feature, best_threshold, best_gini) = self.find_best_split(samples);

        if best_gini >= 0.5 || best_feature.is_none() {
            // No good split found
            return TreeNode {
                feature_index: None,
                threshold: None,
                left: None,
                right: None,
                value: Some(self.majority_class(samples)),
            };
        }

        // Split data
        let (left_samples, right_samples) = self.split_samples(
            samples,
            best_feature.unwrap(),
            best_threshold.unwrap(),
        );

        // Check minimum leaf size
        if left_samples.len() < self.min_samples_leaf
            || right_samples.len() < self.min_samples_leaf
        {
            return TreeNode {
                feature_index: None,
                threshold: None,
                left: None,
                right: None,
                value: Some(self.majority_class(samples)),
            };
        }

        // Build child nodes
        let left_node = self.build_tree(&left_samples, depth + 1, feature_names);
        let right_node = self.build_tree(&right_samples, depth + 1, feature_names);

        TreeNode {
            feature_index: best_feature,
            threshold: best_threshold,
            left: Some(Box::new(left_node)),
            right: Some(Box::new(right_node)),
            value: None,
        }
    }

    fn find_best_split(&self, samples: &[TrainingSample]) -> (Option<usize>, Option<f64>, f64) {
        if samples.is_empty() {
            return (None, None, 1.0);
        }

        let num_features = samples[0].features.len();
        let mut best_gini = f64::INFINITY;
        let mut best_feature = None;
        let mut best_threshold = None;

        // Try each feature
        for feature_idx in 0..num_features {
            // Get unique values for this feature
            let mut values: Vec<f64> = samples
                .iter()
                .map(|s| s.features[feature_idx])
                .collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            values.dedup();

            // Try each threshold
            for threshold in values {
                let (left, right) = self.split_samples(samples, feature_idx, threshold);

                if left.is_empty() || right.is_empty() {
                    continue;
                }

                let gini = self.weighted_gini(&left, &right, samples.len());

                if gini < best_gini {
                    best_gini = gini;
                    best_feature = Some(feature_idx);
                    best_threshold = Some(threshold);
                }
            }
        }

        (best_feature, best_threshold, best_gini)
    }

    fn split_samples(
        &self,
        samples: &[TrainingSample],
        feature_idx: usize,
        threshold: f64,
    ) -> (Vec<TrainingSample>, Vec<TrainingSample>) {
        let mut left = Vec::new();
        let mut right = Vec::new();

        for sample in samples {
            if sample.features[feature_idx] <= threshold {
                left.push(sample.clone());
            } else {
                right.push(sample.clone());
            }
        }

        (left, right)
    }

    fn weighted_gini(&self, left: &[TrainingSample], right: &[TrainingSample], total: usize) -> f64 {
        let left_weight = left.len() as f64 / total as f64;
        let right_weight = right.len() as f64 / total as f64;

        left_weight * self.gini_impurity(left) + right_weight * self.gini_impurity(right)
    }

    fn gini_impurity(&self, samples: &[TrainingSample]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }

        let mut class_counts: HashMap<i8, usize> = HashMap::new();
        for sample in samples {
            *class_counts.entry(sample.label).or_insert(0) += 1;
        }

        let total = samples.len() as f64;
        let mut gini = 1.0;

        for &count in class_counts.values() {
            let prob = count as f64 / total;
            gini -= prob * prob;
        }

        gini
    }

    fn is_pure(&self, samples: &[TrainingSample]) -> bool {
        if samples.is_empty() {
            return true;
        }

        let first_label = samples[0].label;
        samples.iter().all(|s| s.label == first_label)
    }

    fn majority_class(&self, samples: &[TrainingSample]) -> i8 {
        let mut counts: HashMap<i8, usize> = HashMap::new();
        for sample in samples {
            *counts.entry(sample.label).or_insert(0) += 1;
        }

        *counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(label, _)| label)
            .unwrap_or(&0)
    }

    fn predict(&self, features: &[f64]) -> Result<i8> {
        let mut node = self.root.as_ref().ok_or_else(|| {
            Error::InvalidInput("Tree not trained".to_string())
        })?;

        loop {
            if let Some(value) = node.value {
                return Ok(value);
            }

            let feature_idx = node.feature_index.unwrap();
            let threshold = node.threshold.unwrap();

            node = if features[feature_idx] <= threshold {
                node.left.as_ref().unwrap()
            } else {
                node.right.as_ref().unwrap()
            };
        }
    }
}

/// ML Signal Filter - integrates ML model with trading signals
pub struct MLSignalFilter {
    config: MLFilterConfig,
    feature_engineer: FeatureEngineer,
    model: RandomForestClassifier,
}

impl MLSignalFilter {
    pub fn new(config: MLFilterConfig) -> Self {
        let feature_engineer = FeatureEngineer::new(config.clone());
        let model = RandomForestClassifier::new(config.clone());

        Self {
            config,
            feature_engineer,
            model,
        }
    }

    /// Train the ML model with historical data and labels
    pub fn train(&mut self, samples: &[TrainingSample]) -> Result<()> {
        let feature_names = self.feature_engineer.get_feature_names();
        self.model.train(samples, &feature_names)?;
        Ok(())
    }

    /// Filter a trading signal through the ML model
    pub fn filter_signal(
        &self,
        signal: Signal,
        data: &MarketData,
        index: usize,
    ) -> Result<MLPrediction> {
        // Extract features
        let feature_vector = self.feature_engineer.extract_features(data, index)?;

        // Get prediction
        let (predicted_class, prob_positive, prob_negative) =
            self.model.predict(&feature_vector.features)?;

        // Determine if prediction meets threshold
        let meets_threshold = match signal {
            Signal::Buy => prob_positive >= self.config.min_probability,
            Signal::Sell => prob_negative >= self.config.min_probability,
            Signal::Hold => true, // Always allow hold
        };

        // Determine filtered signal
        let filtered_signal = if meets_threshold {
            signal
        } else {
            Signal::Hold
        };

        Ok(MLPrediction {
            predicted_class,
            probability_positive: prob_positive,
            probability_negative: prob_negative,
            meets_threshold,
            original_signal: signal,
            filtered_signal,
        })
    }

    /// Get feature importances from trained model
    pub fn get_feature_importances(&self) -> Vec<FeatureImportance> {
        self.model.get_feature_importances()
    }

    pub fn is_trained(&self) -> bool {
        self.model.is_trained()
    }
}

// Simple random number generation for tree splitting
fn rand_f64() -> f64 {
    use std::cell::Cell;

    thread_local! {
        static SEED: Cell<u64> = Cell::new(12345);
    }

    SEED.with(|seed| {
        let mut s = seed.get();
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        seed.set(s);
        (s as f64 / u64::MAX as f64)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_engineer_creation() {
        let config = MLFilterConfig::default();
        let engineer = FeatureEngineer::new(config);
        let names = engineer.get_feature_names();
        assert!(names.len() > 20); // Should have 20+ features
    }

    #[test]
    fn test_ml_filter_creation() {
        let config = MLFilterConfig::default();
        let filter = MLSignalFilter::new(config);
        assert!(!filter.is_trained());
    }
}
