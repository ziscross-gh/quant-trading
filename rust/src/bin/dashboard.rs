//! Real-time Trading Dashboard Server
//!
//! Launch a web dashboard to monitor trading performance in real-time.
//!
//! Features:
//! - Live equity curve and drawdown visualization
//! - Strategy performance comparison
//! - Active positions and recent trades
//! - Risk metrics and alerts
//! - News sentiment feed
//! - System health monitoring
//! - WebSocket for real-time updates
//!
//! Usage:
//!   cargo run --release --bin dashboard
//!
//! Then open http://localhost:3000 in your browser

use gold_quant_trading::{
    config::Config,
    dashboard::{start_dashboard, DashboardConfig},
    Result,
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .init();

    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║       Gold/USD Autonomous Trading Dashboard              ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Load configuration
    info!("Loading configuration...");
    let config = Config::load()?;

    // Dashboard configuration
    let dashboard_config = DashboardConfig {
        host: "127.0.0.1".to_string(),
        port: 3000,
        update_interval_ms: 1000,
    };

    println!("📊 Dashboard Features:");
    println!("  • Real-time equity curve visualization");
    println!("  • Strategy performance comparison charts");
    println!("  • Live position and trade monitoring");
    println!("  • Risk metrics and alerts");
    println!("  • News sentiment feed");
    println!("  • System health monitoring");
    println!("  • WebSocket real-time updates\n");

    println!("🌐 Dashboard URL: http://{}:{}", dashboard_config.host, dashboard_config.port);
    println!("📡 WebSocket URL: ws://{}:{}/ws\n", dashboard_config.host, dashboard_config.port);

    println!("Press Ctrl+C to stop the server\n");
    println!("{}", "═".repeat(60));

    // Start dashboard server
    start_dashboard(config, dashboard_config).await?;

    Ok(())
}
