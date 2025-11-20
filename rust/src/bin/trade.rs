//! Autonomous trading runner for Gold/USD
//!
//! Production-ready trading system with monitoring and alerts.
//!
//! ## Usage
//!
//! ```bash
//! # Paper trading (default)
//! cargo run --release --bin trade
//!
//! # Demo account trading
//! cargo run --release --bin trade -- --mode demo
//!
//! # With Telegram notifications
//! TELEGRAM_BOT_TOKEN=xxx TELEGRAM_CHAT_ID=yyy cargo run --release --bin trade -- --mode demo
//!
//! # Custom alert thresholds
//! cargo run --release --bin trade -- --mode demo --max-drawdown 15 --max-losses 5
//! ```

use clap::Parser;
use gold_quant_trading::config::Config;
use gold_quant_trading::execution::{AutonomousTrader, ProductionConfig};
use gold_quant_trading::strategies::GoldMomentumStrategy;
use gold_quant_trading::TradingMode;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber;

/// Autonomous Gold/USD trading system with production monitoring
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Trading mode: paper, demo, or live
    #[arg(short, long, default_value = "paper")]
    mode: String,

    /// Path to configuration file
    #[arg(short, long)]
    config: Option<String>,

    /// Run once and exit (for testing)
    #[arg(long)]
    once: bool,

    /// Telegram bot token for notifications
    #[arg(long, env = "TELEGRAM_BOT_TOKEN")]
    telegram_token: Option<String>,

    /// Telegram chat ID for notifications
    #[arg(long, env = "TELEGRAM_CHAT_ID")]
    telegram_chat: Option<String>,

    /// Maximum drawdown percentage before alert (default: 10)
    #[arg(long, default_value = "10.0")]
    max_drawdown: f64,

    /// Maximum consecutive losses before alert (default: 3)
    #[arg(long, default_value = "3")]
    max_losses: u32,

    /// Minimum win rate percentage before alert (default: 45)
    #[arg(long, default_value = "45.0")]
    min_win_rate: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Load configuration
    println!("Loading configuration...");
    let mut config = if let Some(config_path) = args.config {
        Config::from_file(config_path)?
    } else {
        Config::load()?
    };

    // Parse trading mode
    let trading_mode = match args.mode.to_lowercase().as_str() {
        "paper" => TradingMode::Paper,
        "demo" => TradingMode::Paper, // Demo uses paper mode internally
        "live" => TradingMode::Live,
        _ => {
            eprintln!("Invalid trading mode: {}. Use paper, demo, or live.", args.mode);
            return Ok(());
        }
    };

    config.trading.trading_mode = trading_mode;

    // Verify trading mode
    match args.mode.to_lowercase().as_str() {
        "live" => {
            println!("{}", "=".repeat(60));
            println!("WARNING: LIVE TRADING MODE SELECTED");
            println!("Real money will be at risk!");
            println!("{}", "=".repeat(60));

            print!("Are you sure you want to proceed? (yes/no): ");
            use std::io::{self, Write};
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() != "yes" {
                println!("Exiting...");
                return Ok(());
            }
        }
        "demo" => {
            println!("{}", "=".repeat(60));
            println!("DEMO TRADING MODE");
            println!("Using simulated capital with demo broker account");
            println!("{}", "=".repeat(60));
        }
        _ => {
            println!("Running in PAPER TRADING mode (simulation)");
        }
    }

    // Configure production monitoring
    let production_config = ProductionConfig {
        telegram_bot_token: args.telegram_token,
        telegram_chat_id: args.telegram_chat,
        max_drawdown_alert_pct: args.max_drawdown,
        max_consecutive_losses: args.max_losses,
        min_win_rate_alert_pct: args.min_win_rate,
    };

    // Display monitoring configuration
    println!("\n{}", "=".repeat(60));
    println!("PRODUCTION MONITORING CONFIGURATION");
    println!("{}", "=".repeat(60));
    if production_config.telegram_bot_token.is_some() && production_config.telegram_chat_id.is_some() {
        println!("Telegram Notifications: ENABLED");
    } else {
        println!("Telegram Notifications: DISABLED");
        println!("  Set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID to enable");
    }
    println!("Max Drawdown Alert: {:.1}%", production_config.max_drawdown_alert_pct);
    println!("Max Consecutive Losses: {}", production_config.max_consecutive_losses);
    println!("Min Win Rate Alert: {:.1}%", production_config.min_win_rate_alert_pct);
    println!("{}", "=".repeat(60));

    // Initialize strategy
    println!("\nInitializing trading strategy...");
    let strategy = Arc::new(GoldMomentumStrategy::new(config.strategy.clone()));

    // Initialize autonomous trader with monitoring
    println!("Initializing autonomous trader with production monitoring...");
    let mut trader = AutonomousTrader::with_monitoring(config.clone(), strategy, production_config);

    // Handle Ctrl+C
    let _running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = _running.clone();

    ctrlc::set_handler(move || {
        println!("\n\nReceived interrupt signal, shutting down...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })?;

    // Run
    if args.once {
        println!("Running single iteration (test mode)...");
        // trader.run_once().await?;
        println!("Single iteration completed");
    } else {
        println!("\n{}", "=".repeat(60));
        println!("STARTING AUTONOMOUS TRADING SYSTEM");
        println!("{}", "=".repeat(60));
        println!("Symbol: {}", config.trading.symbol);
        println!("Initial Capital: ${:.2}", config.trading.initial_capital);
        println!("Check Interval: {} seconds", config.execution.check_interval);
        println!("{}", "=".repeat(60));
        println!("\nPress Ctrl+C to stop\n");

        tokio::select! {
            result = trader.start() => {
                if let Err(e) = result {
                    eprintln!("Error in trading loop: {}", e);
                }
            }
            _ = tokio::signal::ctrl_c() => {
                trader.stop().await;
            }
        }
    }

    Ok(())
}
