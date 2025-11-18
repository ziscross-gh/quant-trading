//! Position sizing utilities

/// Position sizer for calculating optimal position sizes
pub struct PositionSizer;

impl PositionSizer {
    /// Calculate position size as fixed percentage of capital
    pub fn fixed_percentage(capital: f64, percentage: f64) -> f64 {
        capital * (percentage / 100.0)
    }

    /// Calculate position size using Kelly Criterion
    pub fn kelly_criterion(
        capital: f64,
        win_rate: f64,
        avg_win: f64,
        avg_loss: f64,
    ) -> f64 {
        if avg_loss == 0.0 {
            return 0.0;
        }

        let win_loss_ratio = avg_win / avg_loss.abs();
        let kelly_pct = (win_rate * win_loss_ratio - (1.0 - win_rate)) / win_loss_ratio;

        // Use fractional Kelly (25% of full Kelly for safety)
        let kelly_pct = (kelly_pct * 0.25).clamp(0.0, 1.0);

        capital * kelly_pct
    }

    /// Calculate position size based on risk and stop loss
    pub fn risk_based(
        capital: f64,
        risk_per_trade: f64,
        entry_price: f64,
        stop_loss_price: f64,
    ) -> f64 {
        let risk_amount = capital * (risk_per_trade / 100.0);
        let risk_per_unit = (entry_price - stop_loss_price).abs();

        if risk_per_unit == 0.0 {
            return 0.0;
        }

        risk_amount / risk_per_unit
    }

    /// Calculate position size based on volatility
    pub fn volatility_based(capital: f64, target_risk: f64, volatility: f64) -> f64 {
        if volatility == 0.0 {
            return 0.0;
        }

        (capital * target_risk / 100.0) / volatility
    }
}
