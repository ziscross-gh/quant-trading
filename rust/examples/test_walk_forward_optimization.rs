use chrono::{DateTime, Duration, TimeZone, Utc};
use gold_quant_trading::{
    config::Config,
    optimization::{
        FitnessMetric, ParameterGrid, ParameterRange, WalkForwardConfig, WalkForwardOptimizer,
    },
    types::{Candle, MarketData},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Walk-Forward Optimization Example ===\n");

    // Generate synthetic market data (365 days of hourly data)
    let market_data = generate_synthetic_data(365);

    println!("Generated {} candles of market data", market_data.len());
    println!(
        "Date range: {} to {}\n",
        market_data.candles[0].timestamp.format("%Y-%m-%d"),
        market_data.candles[market_data.len() - 1]
            .timestamp
            .format("%Y-%m-%d")
    );

    // Load base configuration
    let base_config = Config::from_file("config.toml").unwrap_or_else(|_| {
        println!("Warning: Could not load config.toml, using default config");
        Config::default()
    });

    // Configure walk-forward optimization
    let wf_config = WalkForwardConfig {
        optimization_window_days: 90,  // Train on 90 days
        testing_window_days: 30,        // Test on 30 days
        step_size_days: 30,             // Step forward by 30 days
        min_data_points: 200,
        fitness_metric: FitnessMetric::SharpeRatio,
        min_fitness_threshold: 1.0,
        max_combinations: 100,
    };

    println!("Walk-Forward Configuration:");
    println!("  Optimization window: {} days", wf_config.optimization_window_days);
    println!("  Testing window: {} days", wf_config.testing_window_days);
    println!("  Step size: {} days", wf_config.step_size_days);
    println!("  Fitness metric: {:?}", wf_config.fitness_metric);
    println!("  Max combinations: {}\n", wf_config.max_combinations);

    // Define parameter grid for optimization
    let mut parameter_grid = ParameterGrid::new();

    // RSI parameters
    parameter_grid.add_parameter(ParameterRange::new("rsi_period", 10.0, 20.0, 5.0));
    parameter_grid.add_parameter(ParameterRange::new("rsi_oversold", 20.0, 30.0, 5.0));
    parameter_grid.add_parameter(ParameterRange::new("rsi_overbought", 70.0, 80.0, 5.0));

    // Moving average parameters
    parameter_grid.add_parameter(ParameterRange::new("fast_ma", 10.0, 20.0, 5.0));
    parameter_grid.add_parameter(ParameterRange::new("slow_ma", 30.0, 50.0, 10.0));

    // Bollinger Band parameters
    parameter_grid.add_parameter(ParameterRange::new("bb_period", 15.0, 25.0, 5.0));
    parameter_grid.add_parameter(ParameterRange::new("bb_std", 1.5, 2.5, 0.5));

    println!("Parameter Grid:");
    for param in &parameter_grid.parameters {
        println!(
            "  {}: {} to {} (step {})",
            param.name, param.min, param.max, param.step
        );
    }
    println!(
        "  Total combinations: {}\n",
        parameter_grid.combination_count()
    );

    // Note: This is limited by max_combinations in config
    if parameter_grid.combination_count() > wf_config.max_combinations {
        println!(
            "Note: Grid has {} combinations, but will test only {} (limited by max_combinations)\n",
            parameter_grid.combination_count(),
            wf_config.max_combinations
        );
    }

    // Create optimizer
    let optimizer = WalkForwardOptimizer::new(wf_config, parameter_grid);

    // Run walk-forward optimization
    println!("Starting walk-forward optimization...\n");
    println!("This may take a few minutes depending on the number of windows and combinations.");
    println!("Each window will optimize parameters on training data and test on out-of-sample data.\n");

    match optimizer.optimize(&market_data, &base_config) {
        Ok(result) => {
            println!("\n=== OPTIMIZATION COMPLETE ===\n");
            println!("{}", result.summary());
            println!();

            // Display results for each window
            println!("Window-by-Window Results:");
            println!("{:<8} {:<25} {:<25} {:<12} {:<12} {:<15}",
                "Window", "Train Period", "Test Period", "In-Sample", "Out-Sample", "Degradation"
            );
            println!("{}", "-".repeat(110));

            for (i, window_result) in result.window_results.iter().enumerate() {
                println!(
                    "{:<8} {:<25} {:<25} {:<12.2} {:<12.2} {:<15.1}%",
                    format!("{}/{}", i + 1, result.window_results.len()),
                    format!(
                        "{} to {}",
                        window_result.window.train_start.format("%Y-%m-%d"),
                        window_result.window.train_end.format("%Y-%m-%d")
                    ),
                    format!(
                        "{} to {}",
                        window_result.window.test_start.format("%Y-%m-%d"),
                        window_result.window.test_end.format("%Y-%m-%d")
                    ),
                    window_result.in_sample_performance.sharpe_ratio,
                    window_result.out_of_sample_performance.sharpe_ratio,
                    window_result.performance_degradation()
                );
            }

            println!();

            // Show parameter stability
            println!("Parameter Stability (Coefficient of Variation %):");
            println!("Lower values indicate more stable parameters across windows\n");

            let mut stability_vec: Vec<_> = result.parameter_stability.iter().collect();
            stability_vec.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());

            for (param, cv) in stability_vec {
                let stability_assessment = if *cv < 10.0 {
                    "Very Stable"
                } else if *cv < 20.0 {
                    "Stable"
                } else if *cv < 30.0 {
                    "Moderate"
                } else {
                    "Unstable"
                };
                println!("  {:<20} {:<8.1}%  ({})", param, cv, stability_assessment);
            }

            println!();

            // Show best parameters from last window
            if let Some(last_window) = result.window_results.last() {
                println!("Recommended Parameters (from most recent window):");
                let mut params_vec: Vec<_> = last_window.best_parameters.iter().collect();
                params_vec.sort_by_key(|a| a.0);

                for (param, value) in params_vec {
                    println!("  {:<20} = {:.2}", param, value);
                }
            }

            println!();
            println!("=== Performance Analysis ===");
            println!("Average Out-of-Sample Sharpe Ratio: {:.2}", result.out_of_sample_sharpe());
            println!("Average Performance Degradation: {:.1}%", result.average_degradation());
            println!();

            if result.average_degradation() < 10.0 {
                println!("✓ Low degradation suggests parameters generalize well");
            } else if result.average_degradation() < 25.0 {
                println!("⚠ Moderate degradation - some overfitting may be present");
            } else {
                println!("✗ High degradation indicates potential overfitting");
            }

            println!("\nTotal optimization time: {:.1} seconds", result.total_duration_secs);
        }
        Err(e) => {
            eprintln!("Optimization failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Generate synthetic market data for testing
fn generate_synthetic_data(days: usize) -> MarketData {
    let mut market_data = MarketData::new("XAU/USD".to_string());

    // Start from 1 year ago
    let start_time = Utc::now() - Duration::days(days as i64);

    let mut price = 1850.0; // Gold starting price
    let mut trend = 0.0;

    for i in 0..(days * 24) {
        // Hourly data
        let timestamp = start_time + Duration::hours(i as i64);

        // Simulate trending behavior
        if i % 100 == 0 {
            trend = (rand::random::<f64>() - 0.5) * 2.0; // Random trend change
        }

        // Add some volatility and trend
        let volatility = 5.0 + (rand::random::<f64>() * 10.0);
        let change = trend + (rand::random::<f64>() - 0.5) * volatility;

        price += change;
        price = price.max(1000.0).min(2500.0); // Keep in realistic range

        let high = price + rand::random::<f64>() * 5.0;
        let low = price - rand::random::<f64>() * 5.0;
        let open = low + rand::random::<f64>() * (high - low);
        let close = low + rand::random::<f64>() * (high - low);
        let volume = 1000.0 + rand::random::<f64>() * 5000.0;

        market_data.candles.push(Candle {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        });
    }

    market_data
}

// Simple random number generation for demo purposes
mod rand {
    use std::cell::Cell;

    thread_local! {
        static SEED: Cell<u64> = Cell::new(12345);
    }

    pub fn random<T: From<f64>>() -> T {
        SEED.with(|seed| {
            let mut s = seed.get();
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            seed.set(s);
            T::from((s as f64 / u64::MAX as f64))
        })
    }
}
