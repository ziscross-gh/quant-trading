//! Strategy Comparison Tool
//!
//! Backtests ALL strategies and compares their performance side-by-side.
//! Helps you choose the best strategy for your risk profile and market conditions.

use gold_quant_trading::{
    analytics::PerformanceMetrics,
    backtesting::BacktestEngine,
    config::Config,
    strategies::*,
    Result,
};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set subscriber");

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("        GOLD/USD STRATEGY PERFORMANCE COMPARISON");
    println!("═══════════════════════════════════════════════════════════════\n");

    let config = Config::load()?;
    let initial_capital = config.trading.initial_capital;

    // Load historical market data
    info!("Loading historical data...");
    let data_fetcher = gold_quant_trading::data::DataFetcher::new(
        config.trading.symbol.clone(),
        config.data.cache_enabled,
        config.data.cache_dir.clone(),
    );
    let data = data_fetcher
        .fetch_historical(
            &config.backtest.start_date,
            &config.backtest.end_date,
            &config.data.interval,
        )
        .await?;
    info!("Loaded {} candles", data.candles.len());

    // Create backtest engine once
    let engine = BacktestEngine::new(config.clone());

    // List of all strategies to test
    let strategies: Vec<(&str, Arc<dyn Strategy>)> = vec![
        (
            "Gold Momentum",
            Arc::new(GoldMomentumStrategy::new(config.strategy.clone())),
        ),
        (
            "Mean Reversion",
            Arc::new(MeanReversionStrategy::new(config.clone())),
        ),
        (
            "Breakout",
            Arc::new(BreakoutStrategy::new(config.clone())),
        ),
        (
            "ATR Volatility",
            Arc::new(VolatilityBreakoutStrategy::new(config.clone())),
        ),
        (
            "Multi-Timeframe",
            Arc::new(MultiTimeframeStrategy::new(config.clone())),
        ),
        (
            "Ensemble (ALL)",
            Arc::new(EnsembleStrategy::new(config.clone())),
        ),
    ];

    let mut results = Vec::new();

    // Run backtest for each strategy
    for (name, strategy) in strategies {
        println!("╔═══════════════════════════════════════════════════════════");
        println!("║ Testing: {}", name);
        println!("╚═══════════════════════════════════════════════════════════\n");

        match engine.run(strategy.as_ref(), &data) {
            Ok(backtest_results) => {
                // Calculate detailed metrics
                let metrics = PerformanceMetrics::from_trades(
                    &backtest_results.trades,
                    initial_capital,
                );

                results.push((name, metrics.clone()));

                // Print summary
                println!("✅ {} Results:", name);
                println!("  Total Trades:   {}", metrics.total_trades);
                println!("  Win Rate:       {:.1}%", metrics.win_rate);
                println!("  Total P&L:      ${:.2}", metrics.total_pnl);
                println!("  Sharpe Ratio:   {:.2}", metrics.sharpe_ratio);
                println!("  Max Drawdown:   ${:.2} ({:.1}%)",
                    metrics.max_drawdown, metrics.max_drawdown_pct);
                println!("  Profit Factor:  {:.2}x\n", metrics.profit_factor);
            }
            Err(e) => {
                println!("❌ {} Failed: {}\n", name, e);
            }
        }
    }

    // Print comparison table
    print_comparison_table(&results);

    // Identify best strategies by different metrics
    print_recommendations(&results);

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                     BACKTEST COMPLETE");
    println!("═══════════════════════════════════════════════════════════════\n");

    Ok(())
}

fn print_comparison_table(results: &[(&str, PerformanceMetrics)]) {
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                    STRATEGY COMPARISON TABLE");
    println!("═══════════════════════════════════════════════════════════════\n");

    println!("{:<20} {:>8} {:>8} {:>12} {:>10} {:>10}",
        "Strategy", "Trades", "Win%", "Total P&L", "Sharpe", "Max DD%");
    println!("{}", "─".repeat(75));

    for (name, metrics) in results {
        println!("{:<20} {:>8} {:>7.1}% ${:>10.2} {:>9.2} {:>9.1}%",
            name,
            metrics.total_trades,
            metrics.win_rate,
            metrics.total_pnl,
            metrics.sharpe_ratio,
            metrics.max_drawdown_pct,
        );
    }

    println!();
}

fn print_recommendations(results: &[(&str, PerformanceMetrics)]) {
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                      RECOMMENDATIONS");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Best by total profit
    if let Some((name, metrics)) = results.iter()
        .max_by(|a, b| a.1.total_pnl.partial_cmp(&b.1.total_pnl).unwrap())
    {
        println!("🏆 HIGHEST PROFIT: {}", name);
        println!("   Total P&L: ${:.2} ({:.1}%)", metrics.total_pnl, metrics.total_pnl_pct);
        println!();
    }

    // Best by Sharpe ratio (risk-adjusted)
    if let Some((name, metrics)) = results.iter()
        .filter(|r| r.1.total_trades >= 10) // Minimum trades for reliability
        .max_by(|a, b| a.1.sharpe_ratio.partial_cmp(&b.1.sharpe_ratio).unwrap())
    {
        println!("⭐ BEST RISK-ADJUSTED: {}", name);
        println!("   Sharpe Ratio: {:.2}", metrics.sharpe_ratio);
        println!("   Win Rate: {:.1}%", metrics.win_rate);
        println!();
    }

    // Best win rate
    if let Some((name, metrics)) = results.iter()
        .filter(|r| r.1.total_trades >= 10)
        .max_by(|a, b| a.1.win_rate.partial_cmp(&b.1.win_rate).unwrap())
    {
        println!("🎯 HIGHEST WIN RATE: {}", name);
        println!("   Win Rate: {:.1}% ({}/{} trades)",
            metrics.win_rate, metrics.winning_trades, metrics.total_trades);
        println!();
    }

    // Lowest drawdown
    if let Some((name, metrics)) = results.iter()
        .min_by(|a, b| a.1.max_drawdown_pct.partial_cmp(&b.1.max_drawdown_pct).unwrap())
    {
        println!("🛡️  LOWEST DRAWDOWN: {}", name);
        println!("   Max Drawdown: {:.1}%", metrics.max_drawdown_pct);
        println!();
    }

    // Best profit factor
    if let Some((name, metrics)) = results.iter()
        .filter(|r| r.1.total_trades >= 10 && r.1.profit_factor > 0.0)
        .max_by(|a, b| a.1.profit_factor.partial_cmp(&b.1.profit_factor).unwrap())
    {
        println!("💰 BEST PROFIT FACTOR: {}", name);
        println!("   Profit Factor: {:.2}x", metrics.profit_factor);
        println!("   Avg Win: ${:.2} vs Avg Loss: ${:.2}",
            metrics.avg_win, metrics.avg_loss);
        println!();
    }

    println!("💡 USAGE TIPS:");
    println!("   • Use Ensemble for most robust performance");
    println!("   • Use Multi-Timeframe for trending markets");
    println!("   • Use Mean Reversion for ranging markets");
    println!("   • Use ATR Volatility after consolidation periods");
    println!();
}
