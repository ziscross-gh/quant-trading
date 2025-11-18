//! Autonomous trading runner for Gold/USD

use clap::Parser;
use gold_quant_trading::config::Config;
use gold_quant_trading::execution::AutonomousTrader;
use gold_quant_trading::strategies::GoldMomentumStrategy;
use gold_quant_trading::TradingMode;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber;

/// Autonomous Gold/USD trading system
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Trading mode: paper or live
    #[arg(short, long, default_value = "paper")]
    mode: String,

    /// Path to configuration file
    #[arg(short, long)]
    config: Option<String>,

    /// Run once and exit (for testing)
    #[arg(long)]
    once: bool,
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

    // Override trading mode if specified
    config.trading.trading_mode = args.mode.parse::<TradingMode>()
        .unwrap_or(TradingMode::Paper);

    // Verify trading mode
    if config.trading.trading_mode == TradingMode::Live {
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
    } else {
        println!("Running in PAPER TRADING mode (simulation)");
    }

    // Initialize strategy
    println!("Initializing trading strategy...");
    let strategy = Arc::new(GoldMomentumStrategy::new(config.strategy.clone()));

    // Initialize autonomous trader
    println!("Initializing autonomous trader...");
    let mut trader = AutonomousTrader::new(config, strategy);

    // Handle Ctrl+C
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();

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
        println!("Starting autonomous trading system...");
        println!("Press Ctrl+C to stop\n");

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
