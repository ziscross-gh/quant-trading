//! Backtest runner for Gold/USD trading strategy

use gold_quant_trading::backtesting::BacktestEngine;
use gold_quant_trading::config::Config;
use gold_quant_trading::data::DataFetcher;
use gold_quant_trading::strategies::GoldMomentumStrategy;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    println!("{}", "=".repeat(60));
    println!("GOLD/USD TRADING STRATEGY BACKTEST");
    println!("{}", "=".repeat(60));

    // Load configuration
    println!("Loading configuration...");
    let config = Config::load()?;

    // Initialize components
    println!("Initializing components...");
    let data_fetcher = DataFetcher::new(
        config.trading.symbol.clone(),
        config.data.cache_enabled,
        config.data.cache_dir.clone(),
    );

    let strategy = Arc::new(GoldMomentumStrategy::new(config.strategy.clone()));

    // Fetch historical data
    println!("Fetching historical data...");
    let data = data_fetcher
        .fetch_historical(
            &config.backtest.start_date,
            &config.backtest.end_date,
            &config.data.interval,
        )
        .await?;

    println!("Data loaded: {} periods", data.len());
    println!(
        "Date range: {} to {}",
        data.candles.first().map(|c| c.timestamp.to_string()).unwrap_or_default(),
        data.candles.last().map(|c| c.timestamp.to_string()).unwrap_or_default()
    );

    // Run backtest
    println!("Running backtest...");
    let backtest = BacktestEngine::new(config);
    let results = backtest.run(strategy.as_ref(), &data)?;

    // Print results
    results.metrics.print_summary();

    // Save results to CSV
    println!("\nSaving results...");
    save_results(&results)?;

    println!("\n{}", "=".repeat(60));
    println!("Backtest completed successfully!");
    println!("{}", "=".repeat(60));

    Ok(())
}

fn save_results(results: &gold_quant_trading::backtesting::BacktestResults) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    fs::create_dir_all("rust/results")?;

    // Save trades
    if !results.trades.is_empty() {
        let mut wtr = csv::Writer::from_path("rust/results/backtest_trades.csv")?;

        wtr.write_record(&[
            "entry_time",
            "exit_time",
            "signal",
            "entry_price",
            "exit_price",
            "size",
            "pnl",
            "pnl_pct",
            "duration_hours",
        ])?;

        for trade in &results.trades {
            wtr.write_record(&[
                trade.entry_time.to_rfc3339(),
                trade.exit_time.to_rfc3339(),
                format!("{:?}", trade.signal),
                trade.entry_price.to_string(),
                trade.exit_price.to_string(),
                trade.size.to_string(),
                trade.pnl.to_string(),
                trade.pnl_pct.to_string(),
                trade.duration_hours.to_string(),
            ])?;
        }

        wtr.flush()?;
        println!("Trades saved to: rust/results/backtest_trades.csv");
    }

    // Save equity curve
    if !results.equity_curve.is_empty() {
        let mut wtr = csv::Writer::from_path("rust/results/backtest_equity_curve.csv")?;

        wtr.write_record(&["timestamp", "equity", "price"])?;

        for point in &results.equity_curve {
            wtr.write_record(&[
                point.timestamp.to_rfc3339(),
                point.equity.to_string(),
                point.price.to_string(),
            ])?;
        }

        wtr.flush()?;
        println!("Equity curve saved to: rust/results/backtest_equity_curve.csv");
    }

    // Save metrics
    let metrics_json = serde_json::to_string_pretty(&results.metrics)?;
    fs::write("rust/results/backtest_metrics.json", metrics_json)?;
    println!("Metrics saved to: rust/results/backtest_metrics.json");

    Ok(())
}
