"""Position sizing utilities."""

from typing import Dict, Any
from loguru import logger


class PositionSizer:
    """
    Calculate optimal position sizes based on various methodologies.
    """

    def __init__(self, config: Dict[str, Any]):
        """
        Initialize position sizer.

        Args:
            config: Configuration dictionary
        """
        self.config = config
        self.risk_config = config.get('risk', {})

    def fixed_percentage(
        self,
        capital: float,
        percentage: float
    ) -> float:
        """
        Calculate position size as a fixed percentage of capital.

        Args:
            capital: Available capital
            percentage: Percentage of capital to allocate (0-100)

        Returns:
            Position value
        """
        return capital * (percentage / 100)

    def kelly_criterion(
        self,
        capital: float,
        win_rate: float,
        avg_win: float,
        avg_loss: float
    ) -> float:
        """
        Calculate position size using Kelly Criterion.

        Args:
            capital: Available capital
            win_rate: Historical win rate (0-1)
            avg_win: Average winning trade amount
            avg_loss: Average losing trade amount

        Returns:
            Position value
        """
        if avg_loss == 0:
            return 0

        win_loss_ratio = avg_win / abs(avg_loss)
        kelly_pct = (win_rate * win_loss_ratio - (1 - win_rate)) / win_loss_ratio

        # Use fractional Kelly (typically 25-50% of full Kelly for safety)
        kelly_pct = kelly_pct * 0.25

        # Ensure kelly_pct is between 0 and 1
        kelly_pct = max(0, min(1, kelly_pct))

        return capital * kelly_pct

    def risk_based(
        self,
        capital: float,
        risk_per_trade: float,
        entry_price: float,
        stop_loss_price: float
    ) -> float:
        """
        Calculate position size based on risk amount and stop loss.

        Args:
            capital: Available capital
            risk_per_trade: Maximum risk per trade as percentage (0-100)
            entry_price: Entry price
            stop_loss_price: Stop loss price

        Returns:
            Position size in units
        """
        risk_amount = capital * (risk_per_trade / 100)
        risk_per_unit = abs(entry_price - stop_loss_price)

        if risk_per_unit == 0:
            return 0

        position_size = risk_amount / risk_per_unit
        return position_size

    def volatility_based(
        self,
        capital: float,
        target_risk: float,
        volatility: float
    ) -> float:
        """
        Calculate position size based on volatility (ATR).

        Args:
            capital: Available capital
            target_risk: Target risk as percentage (0-100)
            volatility: Asset volatility (e.g., ATR)

        Returns:
            Position value
        """
        if volatility == 0:
            return 0

        position_value = (capital * target_risk / 100) / volatility
        return position_value
