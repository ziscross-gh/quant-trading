use chrono::Utc;
use gold_quant_trading::{
    monitoring::{Alert, AlertLevel, MonitoringConfig, MonitoringSystem},
    types::{Signal, Trade},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Production Monitoring System Example ===\n");

    // Configure monitoring
    let config = MonitoringConfig {
        telegram_token: std::env::var("TELEGRAM_BOT_TOKEN").ok(),
        telegram_chat_id: std::env::var("TELEGRAM_CHAT_ID").ok(),
        telegram_enabled: false, // Set to true when you have Telegram configured
        alert_cooldown_secs: 60,
        daily_reports: true,
        daily_report_hour: 0,
        max_drawdown_alert_pct: 10.0,
        max_consecutive_losses: 3,
        min_win_rate_alert_pct: 45.0,
    };

    println!("Monitoring Configuration:");
    println!("  Telegram Enabled: {}", config.telegram_enabled);
    println!("  Alert Cooldown: {} seconds", config.alert_cooldown_secs);
    println!("  Max Drawdown Alert: {:.1}%", config.max_drawdown_alert_pct);
    println!("  Max Consecutive Losses: {}", config.max_consecutive_losses);
    println!("  Min Win Rate Alert: {:.1}%", config.min_win_rate_alert_pct);
    println!();

    if config.telegram_enabled {
        println!("✅ Telegram notifications are ENABLED");
        println!("   Set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID environment variables");
    } else {
        println!("⚠️  Telegram notifications are DISABLED");
        println!("   To enable:");
        println!("   1. Create a Telegram bot via @BotFather");
        println!("   2. Get your chat ID from @userinfobot");
        println!("   3. Set environment variables:");
        println!("      export TELEGRAM_BOT_TOKEN=\"your_bot_token\"");
        println!("      export TELEGRAM_CHAT_ID=\"your_chat_id\"");
        println!("   4. Set telegram_enabled=true in config");
    }
    println!();

    // Create monitoring system
    let initial_capital = 100000.0;
    let monitoring = MonitoringSystem::new(config, initial_capital);

    println!("📊 Monitoring System Initialized");
    println!("   Initial Capital: ${:.2}\n", initial_capital);

    // Send startup notification
    monitoring.send_startup_notification().await;
    println!("✅ Startup notification sent\n");

    // Simulate some trades
    println!("=== Simulating Trading Activity ===\n");

    // Trade 1: Winning trade
    println!("Trade 1: LONG Gold @ $1850.00");
    monitoring
        .record_trade_entry(
            Signal::Buy,
            1850.0,
            1.0,
            Some(1840.0),
            Some(1870.0),
            "Strong momentum signal with ML confidence 75%".to_string(),
        )
        .await;

    let trade1 = Trade {
        signal: Signal::Buy,
        entry_price: 1850.0,
        entry_time: Utc::now(),
        exit_price: 1865.0,
        exit_time: Utc::now(),
        size: 1.0,
        pnl: 15.0,
        pnl_pct: 0.81,
        duration_hours: 2.5,
        commission: 0.1,
        slippage: 0.05,
        strategy: "GoldMomentum".to_string(),
    };

    monitoring.record_trade(&trade1).await;
    println!("✅ Closed at $1865.00 - PnL: +$15.00 (+0.81%)\n");

    // Trade 2: Winning trade
    println!("Trade 2: LONG Gold @ $1866.00");
    monitoring
        .record_trade_entry(
            Signal::Buy,
            1866.0,
            1.0,
            Some(1856.0),
            Some(1886.0),
            "Regime: Strong Trend (1.2x sizing)".to_string(),
        )
        .await;

    let trade2 = Trade {
        signal: Signal::Buy,
        entry_price: 1866.0,
        entry_time: Utc::now(),
        exit_price: 1880.0,
        exit_time: Utc::now(),
        size: 1.0,
        pnl: 14.0,
        pnl_pct: 0.75,
        duration_hours: 3.0,
        commission: 0.1,
        slippage: 0.05,
        strategy: "GoldMomentum".to_string(),
    };

    monitoring.record_trade(&trade2).await;
    println!("✅ Closed at $1880.00 - PnL: +$14.00 (+0.75%)\n");

    // Trade 3: Losing trade
    println!("Trade 3: SHORT Gold @ $1881.00");
    monitoring
        .record_trade_entry(
            Signal::Sell,
            1881.0,
            1.0,
            Some(1891.0),
            Some(1861.0),
            "RSI overbought, ADX trending down".to_string(),
        )
        .await;

    let trade3 = Trade {
        signal: Signal::Sell,
        entry_price: 1881.0,
        entry_time: Utc::now(),
        exit_price: 1888.0,
        exit_time: Utc::now(),
        size: 1.0,
        pnl: -7.0,
        pnl_pct: -0.37,
        duration_hours: 1.5,
        commission: 0.1,
        slippage: 0.05,
        strategy: "GoldMomentum".to_string(),
    };

    monitoring.record_trade(&trade3).await;
    println!("❌ Stop loss hit at $1888.00 - PnL: -$7.00 (-0.37%)\n");

    // Get current metrics
    let metrics = monitoring.get_metrics().await;

    println!("=== Current Performance Metrics ===\n");
    println!("{}\n", metrics.summary());

    // Test alert system
    println!("=== Testing Alert System ===\n");

    // Info alert
    monitoring
        .send_alert(Alert::new(
            AlertLevel::Info,
            "Market Update".to_string(),
            "Gold breaking above key resistance at $1880".to_string(),
        ))
        .await;
    println!("ℹ️  Info alert sent");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Warning alert
    monitoring
        .send_alert(Alert::new(
            AlertLevel::Warning,
            "High Volatility".to_string(),
            "ATR increased by 35% in last hour".to_string(),
        ))
        .await;
    println!("⚠️  Warning alert sent");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Error alert
    monitoring
        .send_alert(Alert::new(
            AlertLevel::Error,
            "Data Feed Issue".to_string(),
            "Price data delayed by 2 minutes".to_string(),
        ))
        .await;
    println!("❌ Error alert sent");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Critical alert
    monitoring
        .send_alert(Alert::new(
            AlertLevel::Critical,
            "System Critical".to_string(),
            "Broker connection lost - entering safe mode".to_string(),
        ))
        .await;
    println!("🚨 Critical alert sent\n");

    // Simulate automatic alerts from trading conditions
    println!("=== Testing Automatic Alerts ===\n");

    // Simulate multiple losing trades to trigger consecutive loss alert
    for i in 4..=6 {
        let losing_trade = Trade {
            signal: Signal::Buy,
            entry_price: 1900.0,
            entry_time: Utc::now(),
            exit_price: 1895.0,
            exit_time: Utc::now(),
            size: 1.0,
            pnl: -5.0,
            pnl_pct: -0.26,
            duration_hours: 1.0,
            commission: 0.1,
            slippage: 0.05,
            strategy: "GoldMomentum".to_string(),
        };

        monitoring.record_trade(&losing_trade).await;
        println!("Trade {}: Loss recorded", i);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("\n⚠️  Should trigger consecutive losses alert (3 losses)\n");

    // Get health status
    println!("=== System Health Status ===\n");
    let health = monitoring.get_health().await;

    println!("Status: {} {:?}", health.status.emoji(), health.status);
    println!("Uptime: {:.2} hours", health.uptime_hours);
    println!("Data Connection: {}", if health.data_connection { "✅" } else { "❌" });
    println!("Broker Connection: {}", if health.broker_connection { "✅" } else { "❌" });
    println!();

    // Send daily report
    println!("=== Daily Performance Report ===\n");
    monitoring.send_daily_report().await;
    println!("📊 Daily report sent\n");

    // Final metrics
    let final_metrics = monitoring.get_metrics().await;
    println!("=== Final Performance Summary ===\n");
    println!("Total Trades: {}", final_metrics.total_trades);
    println!("Winning: {} ({:.1}%)",
        final_metrics.winning_trades,
        final_metrics.win_rate_pct
    );
    println!("Losing: {}", final_metrics.losing_trades);
    println!("Net PnL: ${:.2}", final_metrics.total_pnl);
    println!("Return: {:.2}%", final_metrics.total_return_pct);
    println!("Consecutive Losses: {}", final_metrics.consecutive_losses);
    println!();

    // Send shutdown notification
    monitoring.send_shutdown_notification().await;
    println!("✅ Shutdown notification sent\n");

    println!("=== Monitoring System Demo Complete ===");
    println!("\nKey Features Demonstrated:");
    println!("  ✓ Trade execution notifications");
    println!("  ✓ Real-time performance metrics");
    println!("  ✓ Multi-level alert system (Info/Warning/Error/Critical)");
    println!("  ✓ Automatic alert triggers (drawdown, losses, win rate)");
    println!("  ✓ Health status monitoring");
    println!("  ✓ Daily performance reports");
    println!("  ✓ Telegram integration (when enabled)");
    println!("\nProduction Ready Features:");
    println!("  • Alert cooldown to prevent spam");
    println!("  • Configurable alert thresholds");
    println!("  • Markdown formatting for Telegram");
    println!("  • Async/await for non-blocking notifications");
    println!("  • Thread-safe metric tracking");

    Ok(())
}
