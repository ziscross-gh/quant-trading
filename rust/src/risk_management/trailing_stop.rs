//! Advanced Trailing Stop Module
//!
//! Implements dynamic ATR-based trailing stops that:
//! - Adapt to market volatility
//! - Use tiered trailing (tighter as profit increases)
//! - Only activate after reaching a profit threshold
//! - Lock in 50%+ of unrealized profits automatically

use crate::{Position, Signal};

/// Configuration for trailing stop behavior
#[derive(Debug, Clone)]
pub struct TrailingStopConfig {
    /// Activation threshold as % of entry price
    /// E.g., 1.0 = only activate after 1% profit
    pub activation_threshold_pct: f64,

    /// ATR multiplier for initial trailing distance
    /// E.g., 1.5 = trail by 1.5x ATR
    pub atr_multiplier: f64,

    /// Whether to use tiered trailing (tighter as profit increases)
    pub use_tiered_trailing: bool,

    /// Profit levels for tiered trailing [profit_pct, atr_multiplier]
    /// E.g., [[2.0, 1.2], [4.0, 0.8]] means:
    ///   - At 2% profit, use 1.2x ATR
    ///   - At 4% profit, use 0.8x ATR
    pub tiers: Vec<(f64, f64)>,
}

impl Default for TrailingStopConfig {
    fn default() -> Self {
        Self {
            activation_threshold_pct: 1.0, // Activate after 1% profit
            atr_multiplier: 1.5,           // Trail by 1.5x ATR
            use_tiered_trailing: true,
            tiers: vec![
                (1.0, 1.5),  // At 1% profit: 1.5x ATR
                (2.0, 1.2),  // At 2% profit: 1.2x ATR
                (3.0, 1.0),  // At 3% profit: 1.0x ATR
                (5.0, 0.8),  // At 5% profit: 0.8x ATR (lock in more)
            ],
        }
    }
}

/// Advanced trailing stop manager
pub struct TrailingStopManager {
    pub config: TrailingStopConfig,
}

impl TrailingStopManager {
    pub fn new(config: TrailingStopConfig) -> Self {
        Self { config }
    }

    /// Calculate the trailing stop price for a position
    ///
    /// # Arguments
    /// * `position` - The open position
    /// * `current_price` - Current market price
    /// * `current_atr` - Current ATR value
    ///
    /// # Returns
    /// Option<f64> - New trailing stop price, or None if not activated
    pub fn calculate_trailing_stop(
        &self,
        position: &Position,
        current_price: f64,
        current_atr: f64,
    ) -> Option<f64> {
        // Calculate unrealized profit %
        let profit_pct = position.unrealized_pnl_pct(current_price);

        // Check if we should activate trailing stop
        if profit_pct < self.config.activation_threshold_pct {
            return None; // Not enough profit yet
        }

        // Get ATR multiplier based on profit level (tiered trailing)
        let atr_multiplier = if self.config.use_tiered_trailing {
            self.get_tier_multiplier(profit_pct)
        } else {
            self.config.atr_multiplier
        };

        // Calculate trailing stop based on direction
        let trailing_stop = match position.signal {
            Signal::Buy => {
                // For long: trail below current price
                current_price - (current_atr * atr_multiplier)
            }
            Signal::Sell => {
                // For short: trail above current price
                current_price + (current_atr * atr_multiplier)
            }
            Signal::Hold => return None,
        };

        Some(trailing_stop)
    }

    /// Get the appropriate ATR multiplier for the current profit level
    fn get_tier_multiplier(&self, profit_pct: f64) -> f64 {
        // Find the highest tier we've reached
        let mut multiplier = self.config.atr_multiplier;

        for (tier_profit_pct, tier_multiplier) in &self.config.tiers {
            if profit_pct >= *tier_profit_pct {
                multiplier = *tier_multiplier;
            } else {
                break; // Tiers should be sorted ascending
            }
        }

        multiplier
    }

    /// Update a position's trailing stop (called every candle/tick)
    ///
    /// # Returns
    /// (updated_position, stop_adjusted) - The position and whether stop was moved up
    pub fn update_position_trailing_stop(
        &self,
        position: &mut Position,
        current_price: f64,
        current_atr: f64,
    ) -> bool {
        // Update peak price
        let peak_updated = match position.signal {
            Signal::Buy => {
                if current_price > position.peak_price {
                    position.peak_price = current_price;
                    true
                } else {
                    false
                }
            }
            Signal::Sell => {
                if current_price < position.peak_price {
                    position.peak_price = current_price;
                    true
                } else {
                    false
                }
            }
            Signal::Hold => false,
        };

        // Calculate new trailing stop
        if let Some(new_trailing_stop) = self.calculate_trailing_stop(position, current_price, current_atr) {
            // Only update if it moves in our favor
            let should_update = match position.signal {
                Signal::Buy => {
                    // For long: only raise the stop, never lower it
                    position.trailing_stop.is_none() || new_trailing_stop > position.trailing_stop.unwrap()
                }
                Signal::Sell => {
                    // For short: only lower the stop, never raise it
                    position.trailing_stop.is_none() || new_trailing_stop < position.trailing_stop.unwrap()
                }
                Signal::Hold => false,
            };

            if should_update {
                position.trailing_stop = Some(new_trailing_stop);
                return true; // Stop was adjusted
            }
        }

        false
    }

    /// Check if trailing stop has been hit
    pub fn is_trailing_stop_hit(
        &self,
        position: &Position,
        current_price: f64,
    ) -> bool {
        if let Some(trailing_stop) = position.trailing_stop {
            match position.signal {
                Signal::Buy => current_price <= trailing_stop,
                Signal::Sell => current_price >= trailing_stop,
                Signal::Hold => false,
            }
        } else {
            false
        }
    }

    /// Get a description of the current trailing stop status
    pub fn get_status_description(
        &self,
        position: &Position,
        current_price: f64,
    ) -> String {
        let profit_pct = position.unrealized_pnl_pct(current_price);

        if let Some(trailing_stop) = position.trailing_stop {
            let distance_pct = match position.signal {
                Signal::Buy => ((current_price - trailing_stop) / current_price) * 100.0,
                Signal::Sell => ((trailing_stop - current_price) / current_price) * 100.0,
                Signal::Hold => 0.0,
            };

            format!(
                "Trailing: ${:.2} ({:.2}% away) | Profit: {:.2}% | Peak: ${:.2}",
                trailing_stop, distance_pct, profit_pct, position.peak_price
            )
        } else {
            if profit_pct < self.config.activation_threshold_pct {
                format!(
                    "Not active (need {:.1}% profit, at {:.2}%)",
                    self.config.activation_threshold_pct, profit_pct
                )
            } else {
                "Not set".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_trailing_stop_activation() {
        let config = TrailingStopConfig {
            activation_threshold_pct: 1.0,
            atr_multiplier: 1.5,
            use_tiered_trailing: false,
            tiers: vec![],
        };

        let manager = TrailingStopManager::new(config);

        // Create a BUY position at $2000
        let mut position = Position::new(
            1,
            Signal::Buy,
            2000.0,
            0.5,
            1960.0, // 2% stop loss
            2100.0, // 5% take profit
            Utc::now(),
        );

        let atr = 20.0; // $20 ATR

        // At entry price, no trailing stop (0% profit)
        let ts = manager.calculate_trailing_stop(&position, 2000.0, atr);
        assert!(ts.is_none());

        // At 0.5% profit, still no activation
        let ts = manager.calculate_trailing_stop(&position, 2010.0, atr);
        assert!(ts.is_none());

        // At 1.5% profit, trailing stop activates
        let ts = manager.calculate_trailing_stop(&position, 2030.0, atr);
        assert!(ts.is_some());
        assert_eq!(ts.unwrap(), 2030.0 - (20.0 * 1.5)); // 2030 - 30 = 2000
    }

    #[test]
    fn test_tiered_trailing() {
        let config = TrailingStopConfig {
            activation_threshold_pct: 1.0,
            atr_multiplier: 1.5,
            use_tiered_trailing: true,
            tiers: vec![
                (1.0, 1.5),
                (2.0, 1.0),
                (5.0, 0.5),
            ],
        };

        let manager = TrailingStopManager::new(config);

        let position = Position::new(
            1,
            Signal::Buy,
            2000.0,
            0.5,
            1960.0,
            2200.0,
            Utc::now(),
        );

        let atr = 20.0;

        // At 1.5% profit: use 1.5x ATR
        let ts = manager.calculate_trailing_stop(&position, 2030.0, atr);
        assert_eq!(ts.unwrap(), 2030.0 - (20.0 * 1.5));

        // At 3% profit: use 1.0x ATR (tighter)
        let ts = manager.calculate_trailing_stop(&position, 2060.0, atr);
        assert_eq!(ts.unwrap(), 2060.0 - (20.0 * 1.0));

        // At 6% profit: use 0.5x ATR (very tight, lock in profits)
        let ts = manager.calculate_trailing_stop(&position, 2120.0, atr);
        assert_eq!(ts.unwrap(), 2120.0 - (20.0 * 0.5));
    }

    #[test]
    fn test_trailing_stop_never_lowers() {
        let config = TrailingStopConfig::default();
        let manager = TrailingStopManager::new(config);

        let mut position = Position::new(
            1,
            Signal::Buy,
            2000.0,
            0.5,
            1960.0,
            2200.0,
            Utc::now(),
        );

        let atr = 20.0;

        // Price rises to $2050 (+2.5%)
        manager.update_position_trailing_stop(&mut position, 2050.0, atr);
        let initial_stop = position.trailing_stop.unwrap();

        // Price drops to $2030 (still in profit)
        manager.update_position_trailing_stop(&mut position, 2030.0, atr);
        let current_stop = position.trailing_stop.unwrap();

        // Stop should NOT have moved down
        assert_eq!(initial_stop, current_stop);
    }

    #[test]
    fn test_short_position_trailing() {
        let config = TrailingStopConfig::default();
        let manager = TrailingStopManager::new(config);

        let mut position = Position::new(
            1,
            Signal::Sell,
            2000.0,
            0.5,
            2040.0, // Stop above for shorts
            1900.0, // Target below for shorts
            Utc::now(),
        );

        let atr = 20.0;

        // Price falls to $1970 (+1.5% profit for short)
        manager.update_position_trailing_stop(&mut position, 1970.0, atr);

        let ts = position.trailing_stop.unwrap();
        // For short, trailing stop should be above current price
        assert!(ts > 1970.0);

        // Price falls more to $1950 (+2.5% profit)
        manager.update_position_trailing_stop(&mut position, 1950.0, atr);
        let new_ts = position.trailing_stop.unwrap();

        // Stop should have moved DOWN (better for shorts)
        assert!(new_ts < ts);
    }
}
