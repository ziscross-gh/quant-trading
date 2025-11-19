//! Monthly Performance Review Generator
//!
//! Analyzes all trades from the past month and generates a comprehensive
//! review with failure analysis, insights, and actionable recommendations.
//!
//! Usage:
//!   cargo run --release --bin monthly_review -- --month 2024-02

use gold_quant_trading::{
    config::Config,
    learning::{TradeAnalyzer, Priority, FailureType},
    types::Trade,
    Result,
};
use chrono::{Datelike, Utc};
use std::collections::HashMap;
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║        MONTHLY TRADING PERFORMANCE REVIEW                ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Get month to analyze (default: last month)
    let target_month = std::env::args()
        .nth(2)
        .unwrap_or_else(|| {
            let now = Utc::now();
            let last_month = if now.month() == 1 {
                (now.year() - 1, 12)
            } else {
                (now.year(), now.month() - 1)
            };
            format!("{}-{:02}", last_month.0, last_month.1)
        });

    println!("📅 Analyzing month: {}\n", target_month);

    // Load trades from database or CSV
    let trades = load_trades(&target_month)?;

    if trades.is_empty() {
        println!("❌ No trades found for {}", target_month);
        println!("\n💡 Make sure you have:");
        println!("   1. Executed trades during this month");
        println!("   2. Trades saved to database or CSV");
        println!("   3. Correct month format (YYYY-MM)\n");
        return Ok(());
    }

    println!("📊 Loaded {} trades from {}\n", trades.len(), target_month);

    // Generate analysis
    let mut analyzer = TradeAnalyzer::new();
    for trade in &trades {
        analyzer.add_trade(trade.clone());
    }

    let review = analyzer.generate_monthly_review(&target_month);

    // Print comprehensive review
    print_review(&review);

    // Save report to file
    save_report(&review, &target_month)?;

    println!("\n✅ Report saved to: reports/monthly_review_{}.md\n", target_month);

    Ok(())
}

fn load_trades(month: &str) -> Result<Vec<Trade>> {
    // Try to load from CSV first (simple implementation)
    let csv_path = format!("rust/results/trades_{}.csv", month);

    if std::path::Path::new(&csv_path).exists() {
        return load_from_csv(&csv_path);
    }

    // If no CSV, create sample data for demonstration
    println!("⚠️  No trade data found. Using sample data for demonstration.\n");
    Ok(generate_sample_trades(month))
}

fn load_from_csv(path: &str) -> Result<Vec<Trade>> {
    let mut trades = Vec::new();
    let mut rdr = csv::Reader::from_path(path)?;

    for result in rdr.deserialize() {
        let trade: Trade = result?;
        trades.push(trade);
    }

    Ok(trades)
}

fn generate_sample_trades(month: &str) -> Vec<Trade> {
    // Generate sample trades for demonstration
    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use gold_quant_trading::types::Signal;

    let mut trades = Vec::new();
    let base_date = NaiveDate::parse_from_str(&format!("{}-15", month), "%Y-%m-%d")
        .unwrap_or_else(|_| Utc::now().date_naive());

    // Generate 20 sample trades
    for i in 0..20 {
        let entry_time = NaiveDateTime::new(
            base_date + chrono::Duration::days((i % 28) as i64),
            NaiveTime::from_hms_opt(9 + (i % 12) as u32, 0, 0).unwrap(),
        );

        let is_winner = i % 3 != 0; // 66% win rate
        let pnl = if is_winner {
            25.0 + (i as f64 * 2.5)
        } else {
            -15.0 - (i as f64 * 1.5)
        };

        trades.push(Trade {
            entry_price: 2050.0 + (i as f64 * 0.5),
            exit_price: if is_winner { 2052.0 } else { 2048.0 } + (i as f64 * 0.5),
            size: 0.5,
            signal: if i % 2 == 0 { Signal::Buy } else { Signal::Sell },
            pnl,
            pnl_pct: (pnl / (2050.0 * 0.5)) * 100.0,
            entry_time: entry_time.and_utc(),
            exit_time: (entry_time + chrono::Duration::hours(4)).and_utc(),
            duration_hours: 4.0,
            commission: 2.0,
            slippage: 0.5,
            strategy: if i % 3 == 0 {
                "Gold Momentum".to_string()
            } else if i % 3 == 1 {
                "Mean Reversion".to_string()
            } else {
                "Ensemble".to_string()
            },
        });
    }

    trades
}

fn print_review(review: &gold_quant_trading::learning::MonthlyReview) {
    println!("═══════════════════════════════════════════════════════════");
    println!("  PERFORMANCE SUMMARY - {}", review.month);
    println!("═══════════════════════════════════════════════════════════\n");

    // Overall Statistics
    println!("📈 Overall Statistics:");
    println!("   Total Trades:     {}", review.total_trades);
    println!("   Winning Trades:   {} ({:.1}%)", review.winning_trades, review.win_rate);
    println!("   Losing Trades:    {}", review.losing_trades);
    println!("   Total P&L:        ${:.2}", review.total_pnl);
    println!("   Sharpe Ratio:     {:.2}", review.sharpe_ratio);
    println!("   Max Drawdown:     {:.2}%\n", review.max_drawdown);

    // Performance Rating
    let rating = if review.win_rate >= 60.0 && review.total_pnl > 200.0 {
        "🏆 EXCELLENT"
    } else if review.win_rate >= 55.0 && review.total_pnl > 100.0 {
        "✅ GOOD"
    } else if review.win_rate >= 50.0 && review.total_pnl > 0.0 {
        "✓ ACCEPTABLE"
    } else if review.total_pnl > 0.0 {
        "⚠️  NEEDS IMPROVEMENT"
    } else {
        "❌ POOR"
    };
    println!("📊 Monthly Rating: {}\n", rating);

    // Strategy Performance
    println!("═══════════════════════════════════════════════════════════");
    println!("  STRATEGY PERFORMANCE");
    println!("═══════════════════════════════════════════════════════════\n");

    if !review.strategy_performance.is_empty() {
        println!("┌─────────────────────────┬────────┬──────────┬────────────┐");
        println!("│ Strategy                │ Trades │ Win Rate │ Total P&L  │");
        println!("├─────────────────────────┼────────┼──────────┼────────────┤");

        let mut strategies: Vec<_> = review.strategy_performance.iter().collect();
        strategies.sort_by(|a, b| b.1.total_pnl.partial_cmp(&a.1.total_pnl).unwrap());

        for (name, metrics) in strategies {
            let indicator = if metrics.total_pnl > 0.0 { "✅" } else { "❌" };
            println!("│ {:<23} │ {:>6} │ {:>7.1}% │ {:>9.2} {} │",
                truncate(name, 23),
                metrics.trades,
                metrics.win_rate,
                metrics.total_pnl,
                indicator
            );
        }
        println!("└─────────────────────────┴────────┴──────────┴────────────┘\n");

        println!("🏆 Best Strategy:  {}", review.best_strategy);
        println!("📉 Worst Strategy: {}\n", review.worst_strategy);
    }

    // Temporal Analysis
    println!("═══════════════════════════════════════════════════════════");
    println!("  TIME-BASED PERFORMANCE");
    println!("═══════════════════════════════════════════════════════════\n");

    if !review.best_trading_hours.is_empty() {
        println!("⏰ Best Trading Hours:   {}:00 UTC",
            review.best_trading_hours.iter()
                .map(|h| h.to_string())
                .collect::<Vec<_>>()
                .join(", "));
    }
    if !review.worst_trading_hours.is_empty() {
        println!("🚫 Worst Trading Hours:  {}:00 UTC\n",
            review.worst_trading_hours.iter()
                .map(|h| h.to_string())
                .collect::<Vec<_>>()
                .join(", "));
    }

    if !review.best_day_of_week.is_empty() {
        println!("📅 Best Day:             {}", review.best_day_of_week);
        println!("📅 Worst Day:            {}\n", review.worst_day_of_week);
    }

    // Failure Analysis
    if !review.failure_breakdown.is_empty() {
        println!("═══════════════════════════════════════════════════════════");
        println!("  FAILURE ANALYSIS");
        println!("═══════════════════════════════════════════════════════════\n");

        println!("🔍 Loss Breakdown:");
        for (failure_type, count) in &review.failure_breakdown {
            println!("   {:?}: {} trades", failure_type, count);
        }
        println!();
    }

    // Key Insights
    if !review.key_insights.is_empty() {
        println!("═══════════════════════════════════════════════════════════");
        println!("  KEY INSIGHTS");
        println!("═══════════════════════════════════════════════════════════\n");

        for (i, insight) in review.key_insights.iter().enumerate() {
            println!("{}. {}", i + 1, insight);
        }
        println!();
    }

    // Action Items
    if !review.action_items.is_empty() {
        println!("═══════════════════════════════════════════════════════════");
        println!("  ACTION ITEMS");
        println!("═══════════════════════════════════════════════════════════\n");

        let critical: Vec<_> = review.action_items.iter()
            .filter(|a| a.priority == Priority::Critical)
            .collect();
        let high: Vec<_> = review.action_items.iter()
            .filter(|a| a.priority == Priority::High)
            .collect();
        let medium: Vec<_> = review.action_items.iter()
            .filter(|a| a.priority == Priority::Medium)
            .collect();

        if !critical.is_empty() {
            println!("🔴 CRITICAL (Fix Immediately):");
            for action in critical {
                print_action_item(action);
            }
        }

        if !high.is_empty() {
            println!("🟠 HIGH PRIORITY (Fix This Week):");
            for action in high {
                print_action_item(action);
            }
        }

        if !medium.is_empty() {
            println!("🟡 MEDIUM PRIORITY (Fix This Month):");
            for action in medium {
                print_action_item(action);
            }
        }
    }

    println!("═══════════════════════════════════════════════════════════\n");
}

fn print_action_item(action: &gold_quant_trading::learning::ActionItem) {
    println!("\n   📋 {}", action.description);
    println!("      Category: {:?}", action.category);
    println!("      Expected Impact: {}", action.expected_impact);
    if !action.implementation_steps.is_empty() {
        println!("      Steps:");
        for step in &action.implementation_steps {
            println!("        • {}", step);
        }
    }
}

fn save_report(review: &gold_quant_trading::learning::MonthlyReview, month: &str) -> Result<()> {
    fs::create_dir_all("reports")?;

    let report_path = format!("reports/monthly_review_{}.md", month);
    let mut content = String::new();

    content.push_str(&format!("# Monthly Trading Review - {}\n\n", review.month));
    content.push_str(&format!("Generated: {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

    content.push_str("## Performance Summary\n\n");
    content.push_str(&format!("- **Total Trades**: {}\n", review.total_trades));
    content.push_str(&format!("- **Win Rate**: {:.1}%\n", review.win_rate));
    content.push_str(&format!("- **Total P&L**: ${:.2}\n", review.total_pnl));
    content.push_str(&format!("- **Sharpe Ratio**: {:.2}\n", review.sharpe_ratio));
    content.push_str(&format!("- **Max Drawdown**: {:.2}%\n\n", review.max_drawdown));

    content.push_str("## Strategy Performance\n\n");
    content.push_str("| Strategy | Trades | Win Rate | Total P&L | Avg P&L |\n");
    content.push_str("|----------|--------|----------|-----------|----------|\n");

    for (name, metrics) in &review.strategy_performance {
        content.push_str(&format!("| {} | {} | {:.1}% | ${:.2} | ${:.2} |\n",
            name, metrics.trades, metrics.win_rate, metrics.total_pnl, metrics.avg_pnl));
    }

    content.push_str("\n## Key Insights\n\n");
    for insight in &review.key_insights {
        content.push_str(&format!("- {}\n", insight));
    }

    content.push_str("\n## Action Items\n\n");
    for action in &review.action_items {
        content.push_str(&format!("### {:?} Priority: {}\n\n", action.priority, action.description));
        content.push_str(&format!("**Expected Impact**: {}\n\n", action.expected_impact));
        content.push_str("**Steps**:\n");
        for step in &action.implementation_steps {
            content.push_str(&format!("1. {}\n", step));
        }
        content.push_str("\n");
    }

    fs::write(&report_path, content)?;

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:<width$}", s, width = max_len)
    } else {
        format!("{:<width$}", format!("{}...", &s[..max_len-3]), width = max_len)
    }
}
