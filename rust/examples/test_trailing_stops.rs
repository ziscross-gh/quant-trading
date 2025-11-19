//! Demonstration of ATR-based trailing stops

use gold_quant_trading::{
    risk_management::{TrailingStopConfig, TrailingStopManager},
    Position, Signal,
};
use chrono::Utc;

fn main() {
    println!("\n=== ATR-Based Trailing Stops Demonstration ===\n");

    // Create trailing stop configuration
    let config = TrailingStopConfig {
        activation_threshold_pct: 1.0,  // Activate after 1% profit
        atr_multiplier: 1.5,            // Initial trailing distance
        use_tiered_trailing: true,
        tiers: vec![
            (1.0, 1.5),  // At 1% profit: 1.5x ATR
            (2.0, 1.2),  // At 2% profit: 1.2x ATR
            (3.0, 1.0),  // At 3% profit: 1.0x ATR
            (5.0, 0.8),  // At 5% profit: 0.8x ATR
        ],
    };

    let manager = TrailingStopManager::new(config);

    // Create a long position at $2000
    let mut position = Position::new(
        1,
        Signal::Buy,
        2000.0,
        0.5,
        1960.0,  // Stop loss at -2%
        2100.0,  // Take profit at +5%
        Utc::now(),
    );

    let atr = 20.0; // $20 ATR

    println!("Initial Position:");
    println!("  Entry Price: ${:.2}", position.entry_price);
    println!("  Size: {:.2} oz", position.size);
    println!("  Stop Loss: ${:.2}", position.stop_loss);
    println!("  Take Profit: ${:.2}", position.take_profit);
    println!("  Current ATR: ${:.2}\n", atr);

    // Simulate price movement
    let price_scenarios = vec![
        (2000.0, "Entry - No profit yet"),
        (2010.0, "Small move up (+0.5%)"),
        (2025.0, "Trailing activated (+1.25%)"),
        (2050.0, "Strong move up (+2.5%)"),
        (2080.0, "Excellent run (+4.0%)"),
        (2120.0, "Big winner (+6.0%)"),
        (2100.0, "Pullback (-0.94%)"),
        (2090.0, "More pullback (-1.42%)"),
    ];

    for (price, description) in price_scenarios {
        println!("--- {} ---", description);
        println!("Price: ${:.2}", price);

        let adjusted = manager.update_position_trailing_stop(&mut position, price, atr);

        if adjusted {
            println!("  ✓ Trailing stop ADJUSTED");
        }

        let status = manager.get_status_description(&position, price);
        println!("  {}", status);

        if manager.is_trailing_stop_hit(&position, price) {
            println!("  🛑 TRAILING STOP HIT - Position would be closed");
            let final_pnl = position.unrealized_pnl(price);
            println!("  Final P&L: ${:.2} ({:.2}%)", final_pnl, position.unrealized_pnl_pct(price));
            break;
        }

        let unrealized = position.unrealized_pnl(price);
        println!("  Unrealized P&L: ${:.2} ({:.2}%)\n", unrealized, position.unrealized_pnl_pct(price));
    }

    println!("\n=== Demonstration Complete ===");
    println!("\nKey Benefits:");
    println!("  ✓ ATR-based trailing adapts to market volatility");
    println!("  ✓ Tiered trailing tightens as profit increases");
    println!("  ✓ Only activates after reaching profit threshold");
    println!("  ✓ Locks in 50%+ of unrealized profits automatically\n");
}
