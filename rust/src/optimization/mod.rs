// Walk-Forward Optimization Module
//
// Implements sophisticated parameter optimization using walk-forward analysis to prevent overfitting.
// The system uses a rolling window approach: optimize on historical data, test on future data,
// then continuously reoptimize as new data becomes available.
//
// Key Features:
// - Rolling window optimization (90-day train, 30-day test)
// - Grid search for parameter combinations
// - Performance tracking across windows
// - Auto-reoptimization scheduler
// - Prevents overfitting through out-of-sample testing

use crate::{
    backtesting::BacktestEngine,
    config::Config,
    strategies::GoldMomentumStrategy,
    types::MarketData,
    Error, Result,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Configuration for walk-forward optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardConfig {
    /// Number of days to use for optimization (training)
    pub optimization_window_days: i64,

    /// Number of days to test forward (out-of-sample)
    pub testing_window_days: i64,

    /// Number of days to step forward between windows
    pub step_size_days: i64,

    /// Minimum number of data points required
    pub min_data_points: usize,

    /// Fitness metric to optimize (sharpe_ratio, profit_factor, win_rate, etc.)
    pub fitness_metric: FitnessMetric,

    /// Minimum acceptable fitness score
    pub min_fitness_threshold: f64,

    /// Maximum number of parameter combinations to test (prevents excessive computation)
    pub max_combinations: usize,
}

impl Default for WalkForwardConfig {
    fn default() -> Self {
        Self {
            optimization_window_days: 90,
            testing_window_days: 30,
            step_size_days: 30,
            min_data_points: 200,
            fitness_metric: FitnessMetric::SharpeRatio,
            min_fitness_threshold: 1.0,
            max_combinations: 1000,
        }
    }
}

/// Fitness metrics for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FitnessMetric {
    SharpeRatio,
    ProfitFactor,
    WinRate,
    TotalReturn,
    MaxDrawdown,
    CalmarRatio,
    Sortino,
}

impl FitnessMetric {
    pub fn name(&self) -> &'static str {
        match self {
            FitnessMetric::SharpeRatio => "Sharpe Ratio",
            FitnessMetric::ProfitFactor => "Profit Factor",
            FitnessMetric::WinRate => "Win Rate",
            FitnessMetric::TotalReturn => "Total Return",
            FitnessMetric::MaxDrawdown => "Max Drawdown (inverted)",
            FitnessMetric::CalmarRatio => "Calmar Ratio",
            FitnessMetric::Sortino => "Sortino Ratio",
        }
    }
}

/// Represents a single parameter in the search space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterRange {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

impl ParameterRange {
    pub fn new(name: &str, min: f64, max: f64, step: f64) -> Self {
        Self {
            name: name.to_string(),
            min,
            max,
            step,
        }
    }

    /// Generate all values in this parameter's range
    pub fn values(&self) -> Vec<f64> {
        let mut values = Vec::new();
        let mut current = self.min;

        while current <= self.max {
            values.push(current);
            current += self.step;
        }

        values
    }

    pub fn count(&self) -> usize {
        self.values().len()
    }
}

/// Parameter grid for grid search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterGrid {
    pub parameters: Vec<ParameterRange>,
}

impl ParameterGrid {
    pub fn new() -> Self {
        Self {
            parameters: Vec::new(),
        }
    }

    pub fn add_parameter(&mut self, param: ParameterRange) {
        self.parameters.push(param);
    }

    /// Get total number of combinations
    pub fn combination_count(&self) -> usize {
        self.parameters
            .iter()
            .map(|p| p.count())
            .product()
    }

    /// Generate all parameter combinations
    pub fn generate_combinations(&self) -> Vec<HashMap<String, f64>> {
        if self.parameters.is_empty() {
            return vec![HashMap::new()];
        }

        let mut combinations = Vec::new();
        self.generate_recursive(&mut combinations, &mut HashMap::new(), 0);
        combinations
    }

    fn generate_recursive(
        &self,
        combinations: &mut Vec<HashMap<String, f64>>,
        current: &mut HashMap<String, f64>,
        depth: usize,
    ) {
        if depth >= self.parameters.len() {
            combinations.push(current.clone());
            return;
        }

        let param = &self.parameters[depth];
        for value in param.values() {
            current.insert(param.name.clone(), value);
            self.generate_recursive(combinations, current, depth + 1);
        }
    }
}

impl Default for ParameterGrid {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a single optimization window with train/test split
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationWindow {
    /// Start of optimization (training) period
    pub train_start: DateTime<Utc>,

    /// End of optimization (training) period
    pub train_end: DateTime<Utc>,

    /// Start of testing (out-of-sample) period
    pub test_start: DateTime<Utc>,

    /// End of testing (out-of-sample) period
    pub test_end: DateTime<Utc>,
}

impl OptimizationWindow {
    pub fn new(
        train_start: DateTime<Utc>,
        train_end: DateTime<Utc>,
        test_start: DateTime<Utc>,
        test_end: DateTime<Utc>,
    ) -> Self {
        Self {
            train_start,
            train_end,
            test_start,
            test_end,
        }
    }

    pub fn train_days(&self) -> i64 {
        self.train_end
            .signed_duration_since(self.train_start)
            .num_days()
    }

    pub fn test_days(&self) -> i64 {
        self.test_end
            .signed_duration_since(self.test_start)
            .num_days()
    }
}

/// Results from a single parameter combination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterResult {
    pub parameters: HashMap<String, f64>,
    pub fitness_score: f64,
    pub sharpe_ratio: f64,
    pub total_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub win_rate_pct: f64,
    pub profit_factor: f64,
    pub trade_count: usize,
}

/// Results from a complete optimization window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowResult {
    pub window: OptimizationWindow,
    pub best_parameters: HashMap<String, f64>,
    pub in_sample_performance: ParameterResult,
    pub out_of_sample_performance: ParameterResult,
    pub tested_combinations: usize,
    pub optimization_duration_secs: f64,
}

impl WindowResult {
    pub fn performance_degradation(&self) -> f64 {
        // Calculate how much performance degraded from in-sample to out-of-sample
        // Negative value means out-of-sample performed better
        let in_sample = self.in_sample_performance.fitness_score;
        let out_of_sample = self.out_of_sample_performance.fitness_score;

        if in_sample == 0.0 {
            return 0.0;
        }

        ((in_sample - out_of_sample) / in_sample.abs()) * 100.0
    }
}

/// Complete walk-forward optimization results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardResult {
    pub config: WalkForwardConfig,
    pub window_results: Vec<WindowResult>,
    pub total_duration_secs: f64,
    pub parameter_stability: HashMap<String, f64>, // Coefficient of variation for each parameter
}

impl WalkForwardResult {
    pub fn average_degradation(&self) -> f64 {
        if self.window_results.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.window_results
            .iter()
            .map(|w| w.performance_degradation())
            .sum();

        sum / self.window_results.len() as f64
    }

    pub fn out_of_sample_sharpe(&self) -> f64 {
        if self.window_results.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.window_results
            .iter()
            .map(|w| w.out_of_sample_performance.sharpe_ratio)
            .sum();

        sum / self.window_results.len() as f64
    }

    pub fn summary(&self) -> String {
        format!(
            "Walk-Forward Results: {} windows | Avg Out-of-Sample Sharpe: {:.2} | Avg Degradation: {:.1}%",
            self.window_results.len(),
            self.out_of_sample_sharpe(),
            self.average_degradation()
        )
    }
}

/// Walk-forward optimizer
pub struct WalkForwardOptimizer {
    config: WalkForwardConfig,
    parameter_grid: ParameterGrid,
}

impl WalkForwardOptimizer {
    pub fn new(config: WalkForwardConfig, parameter_grid: ParameterGrid) -> Self {
        Self {
            config,
            parameter_grid,
        }
    }

    /// Generate all optimization windows from market data
    pub fn generate_windows(&self, market_data: &MarketData) -> Result<Vec<OptimizationWindow>> {
        let timestamps = market_data.timestamps();
        if timestamps.len() < self.config.min_data_points {
            return Err(Error::InvalidInput(format!(
                "Insufficient data points: {} < {}",
                timestamps.len(),
                self.config.min_data_points
            )));
        }

        let data_start = timestamps[0];
        let data_end = timestamps[timestamps.len() - 1];

        let mut windows = Vec::new();
        let mut current_start = data_start;

        loop {
            let train_end = current_start + Duration::days(self.config.optimization_window_days);
            let test_start = train_end;
            let test_end = test_start + Duration::days(self.config.testing_window_days);

            // Check if we have enough data for this window
            if test_end > data_end {
                break;
            }

            windows.push(OptimizationWindow::new(
                current_start,
                train_end,
                test_start,
                test_end,
            ));

            // Step forward
            current_start = current_start + Duration::days(self.config.step_size_days);
        }

        Ok(windows)
    }

    /// Run walk-forward optimization on market data
    pub fn optimize(&self, market_data: &MarketData, base_config: &Config) -> Result<WalkForwardResult> {
        let start_time = std::time::Instant::now();

        info!("Starting walk-forward optimization...");
        info!("  Optimization window: {} days", self.config.optimization_window_days);
        info!("  Testing window: {} days", self.config.testing_window_days);
        info!("  Step size: {} days", self.config.step_size_days);

        let windows = self.generate_windows(market_data)?;
        info!("  Generated {} optimization windows", windows.len());

        let combinations = self.parameter_grid.generate_combinations();
        let combination_count = combinations.len().min(self.config.max_combinations);
        info!("  Testing {} parameter combinations per window", combination_count);

        if combination_count > self.config.max_combinations {
            warn!(
                "  Limiting combinations from {} to {}",
                combinations.len(),
                self.config.max_combinations
            );
        }

        let mut window_results = Vec::new();

        for (i, window) in windows.iter().enumerate() {
            info!("");
            info!("Window {}/{}: Train {:?} to {:?}, Test {:?} to {:?}",
                i + 1,
                windows.len(),
                window.train_start.format("%Y-%m-%d"),
                window.train_end.format("%Y-%m-%d"),
                window.test_start.format("%Y-%m-%d"),
                window.test_end.format("%Y-%m-%d")
            );

            let window_result = self.optimize_window(
                window,
                market_data,
                base_config,
                &combinations[..combination_count],
            )?;

            info!("  Best parameters: {:?}", window_result.best_parameters);
            info!("  In-sample Sharpe: {:.2}", window_result.in_sample_performance.sharpe_ratio);
            info!("  Out-of-sample Sharpe: {:.2}", window_result.out_of_sample_performance.sharpe_ratio);
            info!("  Performance degradation: {:.1}%", window_result.performance_degradation());

            window_results.push(window_result);
        }

        let total_duration = start_time.elapsed().as_secs_f64();

        // Calculate parameter stability across windows
        let parameter_stability = self.calculate_parameter_stability(&window_results);

        let result = WalkForwardResult {
            config: self.config.clone(),
            window_results,
            total_duration_secs: total_duration,
            parameter_stability,
        };

        info!("");
        info!("Walk-forward optimization complete!");
        info!("{}", result.summary());
        info!("Total duration: {:.1} seconds", total_duration);

        Ok(result)
    }

    /// Optimize a single window
    fn optimize_window(
        &self,
        window: &OptimizationWindow,
        market_data: &MarketData,
        base_config: &Config,
        combinations: &[HashMap<String, f64>],
    ) -> Result<WindowResult> {
        let window_start_time = std::time::Instant::now();

        // Filter market data for training period
        let train_data = market_data.filter_by_date_range(window.train_start, window.train_end)?;

        // Find best parameters on training data
        let mut best_score = f64::NEG_INFINITY;
        let mut best_params = HashMap::new();
        let mut best_result = None;

        for params in combinations {
            // Apply parameters to config
            let mut config = base_config.clone();
            self.apply_parameters(&mut config, params);

            // Create strategy and run backtest on training data
            let strategy = GoldMomentumStrategy::new(config.strategy.clone());
            let engine = BacktestEngine::new(config);
            let backtest_result = engine.run(&strategy, &train_data)?;

            // Calculate fitness score
            let score = self.calculate_fitness(&backtest_result, &self.config.fitness_metric);

            if score > best_score {
                best_score = score;
                best_params = params.clone();
                best_result = Some(ParameterResult {
                    parameters: params.clone(),
                    fitness_score: score,
                    sharpe_ratio: backtest_result.metrics.sharpe_ratio,
                    total_return_pct: backtest_result.metrics.total_return_pct,
                    max_drawdown_pct: backtest_result.metrics.max_drawdown_pct,
                    win_rate_pct: backtest_result.metrics.win_rate_pct,
                    profit_factor: backtest_result.metrics.profit_factor,
                    trade_count: backtest_result.trades.len(),
                });
            }
        }

        let in_sample_performance = best_result.ok_or_else(|| {
            Error::OptimizationError("No valid parameter combinations found".to_string())
        })?;

        // Test best parameters on out-of-sample data
        let test_data = market_data.filter_by_date_range(window.test_start, window.test_end)?;
        let mut test_config = base_config.clone();
        self.apply_parameters(&mut test_config, &best_params);

        let test_strategy = GoldMomentumStrategy::new(test_config.strategy.clone());
        let test_engine = BacktestEngine::new(test_config);
        let test_result = test_engine.run(&test_strategy, &test_data)?;

        let out_of_sample_performance = ParameterResult {
            parameters: best_params.clone(),
            fitness_score: self.calculate_fitness(&test_result, &self.config.fitness_metric),
            sharpe_ratio: test_result.metrics.sharpe_ratio,
            total_return_pct: test_result.metrics.total_return_pct,
            max_drawdown_pct: test_result.metrics.max_drawdown_pct,
            win_rate_pct: test_result.metrics.win_rate_pct,
            profit_factor: test_result.metrics.profit_factor,
            trade_count: test_result.trades.len(),
        };

        let optimization_duration = window_start_time.elapsed().as_secs_f64();

        Ok(WindowResult {
            window: window.clone(),
            best_parameters: best_params,
            in_sample_performance,
            out_of_sample_performance,
            tested_combinations: combinations.len(),
            optimization_duration_secs: optimization_duration,
        })
    }

    /// Apply parameters to config
    fn apply_parameters(&self, config: &mut Config, params: &HashMap<String, f64>) {
        // Apply RSI parameters
        if let Some(&period) = params.get("rsi_period") {
            config.strategy.rsi_period = period as usize;
        }
        if let Some(&oversold) = params.get("rsi_oversold") {
            config.strategy.rsi_oversold = oversold;
        }
        if let Some(&overbought) = params.get("rsi_overbought") {
            config.strategy.rsi_overbought = overbought;
        }

        // Apply Moving Average parameters
        if let Some(&fast) = params.get("fast_ma") {
            config.strategy.fast_ma = fast as usize;
        }
        if let Some(&slow) = params.get("slow_ma") {
            config.strategy.slow_ma = slow as usize;
        }

        // Apply Bollinger Band parameters
        if let Some(&period) = params.get("bb_period") {
            config.strategy.bb_period = period as usize;
        }
        if let Some(&std_dev) = params.get("bb_std") {
            config.strategy.bb_std = std_dev;
        }

        // Apply risk parameters
        if let Some(&stop_loss) = params.get("stop_loss_pct") {
            config.risk.stop_loss_pct = stop_loss;
        }
        if let Some(&take_profit) = params.get("take_profit_pct") {
            config.risk.take_profit_pct = take_profit;
        }
        if let Some(&trailing_stop) = params.get("trailing_stop_pct") {
            config.risk.trailing_stop_pct = trailing_stop;
        }
    }

    /// Calculate fitness score from backtest result
    fn calculate_fitness(
        &self,
        result: &crate::backtesting::BacktestResults,
        metric: &FitnessMetric,
    ) -> f64 {
        match metric {
            FitnessMetric::SharpeRatio => result.metrics.sharpe_ratio,
            FitnessMetric::ProfitFactor => result.metrics.profit_factor,
            FitnessMetric::WinRate => result.metrics.win_rate_pct,
            FitnessMetric::TotalReturn => result.metrics.total_return_pct,
            FitnessMetric::MaxDrawdown => -result.metrics.max_drawdown_pct, // Invert since lower is better
            FitnessMetric::CalmarRatio => {
                if result.metrics.max_drawdown_pct == 0.0 {
                    0.0
                } else {
                    result.metrics.total_return_pct / result.metrics.max_drawdown_pct.abs()
                }
            }
            FitnessMetric::Sortino => {
                // Use the sortino_ratio from metrics
                result.metrics.sortino_ratio
            }
        }
    }

    /// Calculate parameter stability across windows (coefficient of variation)
    fn calculate_parameter_stability(&self, results: &[WindowResult]) -> HashMap<String, f64> {
        let mut stability = HashMap::new();

        if results.is_empty() {
            return stability;
        }

        // Get all parameter names
        let param_names: Vec<String> = results[0]
            .best_parameters
            .keys()
            .cloned()
            .collect();

        for param_name in param_names {
            let values: Vec<f64> = results
                .iter()
                .filter_map(|r| r.best_parameters.get(&param_name).copied())
                .collect();

            if values.is_empty() {
                continue;
            }

            let mean = values.iter().sum::<f64>() / values.len() as f64;

            if mean == 0.0 {
                stability.insert(param_name, 0.0);
                continue;
            }

            let variance = values
                .iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>() / values.len() as f64;

            let std_dev = variance.sqrt();
            let cv = (std_dev / mean.abs()) * 100.0; // Coefficient of variation as percentage

            stability.insert(param_name, cv);
        }

        stability
    }
}

/// Auto-reoptimization scheduler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReoptimizationScheduler {
    /// Reoptimize every N days
    pub reoptimization_interval_days: i64,

    /// Last optimization timestamp
    pub last_optimization: Option<DateTime<Utc>>,

    /// Current best parameters
    pub current_parameters: HashMap<String, f64>,
}

impl ReoptimizationScheduler {
    pub fn new(interval_days: i64) -> Self {
        Self {
            reoptimization_interval_days: interval_days,
            last_optimization: None,
            current_parameters: HashMap::new(),
        }
    }

    /// Check if reoptimization is needed
    pub fn should_reoptimize(&self, current_time: DateTime<Utc>) -> bool {
        match self.last_optimization {
            None => true, // Never optimized before
            Some(last) => {
                let days_since = current_time
                    .signed_duration_since(last)
                    .num_days();
                days_since >= self.reoptimization_interval_days
            }
        }
    }

    /// Update with new optimization results
    pub fn update(&mut self, parameters: HashMap<String, f64>, timestamp: DateTime<Utc>) {
        self.current_parameters = parameters;
        self.last_optimization = Some(timestamp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_range() {
        let range = ParameterRange::new("test", 10.0, 20.0, 5.0);
        let values = range.values();
        assert_eq!(values, vec![10.0, 15.0, 20.0]);
        assert_eq!(range.count(), 3);
    }

    #[test]
    fn test_parameter_grid() {
        let mut grid = ParameterGrid::new();
        grid.add_parameter(ParameterRange::new("a", 1.0, 2.0, 1.0));
        grid.add_parameter(ParameterRange::new("b", 10.0, 20.0, 10.0));

        assert_eq!(grid.combination_count(), 4); // 2 × 2

        let combinations = grid.generate_combinations();
        assert_eq!(combinations.len(), 4);
    }

    #[test]
    fn test_reoptimization_scheduler() {
        let mut scheduler = ReoptimizationScheduler::new(30);

        let now = Utc::now();
        assert!(scheduler.should_reoptimize(now));

        scheduler.update(HashMap::new(), now);
        assert!(!scheduler.should_reoptimize(now));

        let future = now + Duration::days(31);
        assert!(scheduler.should_reoptimize(future));
    }
}
