"""Risk management module for controlling trading risk."""

from typing import Dict, Any, Optional
from datetime import datetime, timedelta
from loguru import logger
import pandas as pd


class RiskManager:
    """
    Manages trading risk including position sizing, stop losses, and drawdown limits.
    """

    def __init__(self, config: Dict[str, Any]):
        """
        Initialize the risk manager.

        Args:
            config: Configuration dictionary
        """
        self.config = config
        self.risk_config = config.get('risk', {})
        self.trading_config = config.get('trading', {})

        # Risk parameters
        self.max_drawdown_pct = self.risk_config.get('max_drawdown_pct', 15)
        self.stop_loss_pct = self.risk_config.get('stop_loss_pct', 2)
        self.take_profit_pct = self.risk_config.get('take_profit_pct', 5)
        self.risk_per_trade_pct = self.risk_config.get('risk_per_trade_pct', 1)
        self.max_daily_loss_pct = self.risk_config.get('max_daily_loss_pct', 5)
        self.trailing_stop_pct = self.risk_config.get('trailing_stop_pct', 1.5)

        # Trading parameters
        self.initial_capital = self.trading_config.get('initial_capital', 100000)
        self.max_positions = self.trading_config.get('max_positions', 3)

        # State tracking
        self.current_capital = self.initial_capital
        self.peak_capital = self.initial_capital
        self.daily_start_capital = self.initial_capital
        self.open_positions = []
        self.daily_trades = []
        self.last_reset_date = datetime.now().date()

        logger.info(f"RiskManager initialized with capital: ${self.initial_capital:,.2f}")
        logger.info(f"Risk parameters: Max DD: {self.max_drawdown_pct}%, "
                   f"Stop Loss: {self.stop_loss_pct}%, "
                   f"Risk per trade: {self.risk_per_trade_pct}%")

    def can_trade(self, signal: int) -> bool:
        """
        Check if trading is allowed based on risk parameters.

        Args:
            signal: Trading signal (1 for buy, -1 for sell)

        Returns:
            True if trading is allowed, False otherwise
        """
        # Reset daily tracking if new day
        self._check_daily_reset()

        # Check if we've hit max drawdown
        if not self._check_max_drawdown():
            logger.warning("Trading halted: Maximum drawdown limit reached")
            return False

        # Check if we've hit max daily loss
        if not self._check_max_daily_loss():
            logger.warning("Trading halted: Maximum daily loss limit reached")
            return False

        # Check if we have capacity for new positions
        if signal != 0 and len(self.open_positions) >= self.max_positions:
            logger.warning(f"Trading halted: Maximum positions ({self.max_positions}) reached")
            return False

        return True

    def calculate_position_size(
        self,
        signal: int,
        entry_price: float
    ) -> float:
        """
        Calculate position size based on risk parameters.

        Args:
            signal: Trading signal (1 for buy, -1 for sell)
            entry_price: Entry price for the position

        Returns:
            Position size in number of units
        """
        if signal == 0:
            return 0

        # Calculate risk amount
        risk_amount = self.current_capital * (self.risk_per_trade_pct / 100)

        # Calculate position size based on stop loss
        stop_loss_distance = entry_price * (self.stop_loss_pct / 100)
        position_size = risk_amount / stop_loss_distance

        # Alternatively, use fixed percentage of capital
        max_position_value = self.current_capital * (self.trading_config.get('position_size', 0.1))
        max_position_units = max_position_value / entry_price

        # Use the smaller of the two
        position_size = min(position_size, max_position_units)

        logger.debug(f"Calculated position size: {position_size:.4f} units at ${entry_price:.2f}")
        return position_size

    def calculate_stop_loss(self, entry_price: float, signal: int) -> float:
        """
        Calculate stop loss price.

        Args:
            entry_price: Entry price
            signal: Trading signal (1 for long, -1 for short)

        Returns:
            Stop loss price
        """
        if signal == 1:  # Long position
            stop_loss = entry_price * (1 - self.stop_loss_pct / 100)
        else:  # Short position
            stop_loss = entry_price * (1 + self.stop_loss_pct / 100)

        return stop_loss

    def calculate_take_profit(self, entry_price: float, signal: int) -> float:
        """
        Calculate take profit price.

        Args:
            entry_price: Entry price
            signal: Trading signal (1 for long, -1 for short)

        Returns:
            Take profit price
        """
        if signal == 1:  # Long position
            take_profit = entry_price * (1 + self.take_profit_pct / 100)
        else:  # Short position
            take_profit = entry_price * (1 - self.take_profit_pct / 100)

        return take_profit

    def open_position(
        self,
        signal: int,
        entry_price: float,
        size: float,
        timestamp: datetime
    ) -> Dict[str, Any]:
        """
        Record opening a new position.

        Args:
            signal: Trading signal
            entry_price: Entry price
            size: Position size
            timestamp: Entry timestamp

        Returns:
            Position dictionary
        """
        stop_loss = self.calculate_stop_loss(entry_price, signal)
        take_profit = self.calculate_take_profit(entry_price, signal)

        position = {
            'id': len(self.open_positions) + 1,
            'signal': signal,
            'entry_price': entry_price,
            'size': size,
            'stop_loss': stop_loss,
            'take_profit': take_profit,
            'trailing_stop': None,
            'entry_time': timestamp,
            'peak_price': entry_price
        }

        self.open_positions.append(position)
        logger.info(f"Position opened: {signal} {size:.4f} units @ ${entry_price:.2f}")
        logger.info(f"  Stop Loss: ${stop_loss:.2f}, Take Profit: ${take_profit:.2f}")

        return position

    def update_positions(self, current_price: float, timestamp: datetime) -> list:
        """
        Update all open positions and check for stop loss/take profit triggers.

        Args:
            current_price: Current market price
            timestamp: Current timestamp

        Returns:
            List of positions that should be closed
        """
        positions_to_close = []

        for position in self.open_positions:
            # Update trailing stop
            if position['signal'] == 1:  # Long position
                # Update peak price
                if current_price > position['peak_price']:
                    position['peak_price'] = current_price
                    # Update trailing stop
                    trailing_stop = current_price * (1 - self.trailing_stop_pct / 100)
                    if position['trailing_stop'] is None or trailing_stop > position['trailing_stop']:
                        position['trailing_stop'] = trailing_stop
                        logger.debug(f"Trailing stop updated to ${trailing_stop:.2f}")

                # Check exit conditions
                if position['trailing_stop'] and current_price <= position['trailing_stop']:
                    logger.info(f"Trailing stop triggered at ${current_price:.2f}")
                    positions_to_close.append(position)
                elif current_price <= position['stop_loss']:
                    logger.info(f"Stop loss triggered at ${current_price:.2f}")
                    positions_to_close.append(position)
                elif current_price >= position['take_profit']:
                    logger.info(f"Take profit triggered at ${current_price:.2f}")
                    positions_to_close.append(position)

            else:  # Short position
                # Update peak price (lowest for short)
                if current_price < position['peak_price']:
                    position['peak_price'] = current_price
                    # Update trailing stop
                    trailing_stop = current_price * (1 + self.trailing_stop_pct / 100)
                    if position['trailing_stop'] is None or trailing_stop < position['trailing_stop']:
                        position['trailing_stop'] = trailing_stop
                        logger.debug(f"Trailing stop updated to ${trailing_stop:.2f}")

                # Check exit conditions
                if position['trailing_stop'] and current_price >= position['trailing_stop']:
                    logger.info(f"Trailing stop triggered at ${current_price:.2f}")
                    positions_to_close.append(position)
                elif current_price >= position['stop_loss']:
                    logger.info(f"Stop loss triggered at ${current_price:.2f}")
                    positions_to_close.append(position)
                elif current_price <= position['take_profit']:
                    logger.info(f"Take profit triggered at ${current_price:.2f}")
                    positions_to_close.append(position)

        return positions_to_close

    def close_position(
        self,
        position: Dict[str, Any],
        exit_price: float,
        timestamp: datetime
    ) -> Dict[str, Any]:
        """
        Close a position and update capital.

        Args:
            position: Position dictionary
            exit_price: Exit price
            timestamp: Exit timestamp

        Returns:
            Trade result dictionary
        """
        # Calculate P&L
        if position['signal'] == 1:  # Long position
            pnl = (exit_price - position['entry_price']) * position['size']
        else:  # Short position
            pnl = (position['entry_price'] - exit_price) * position['size']

        pnl_pct = (pnl / (position['entry_price'] * position['size'])) * 100

        # Update capital
        self.current_capital += pnl
        if self.current_capital > self.peak_capital:
            self.peak_capital = self.current_capital

        # Record trade
        trade = {
            'entry_price': position['entry_price'],
            'exit_price': exit_price,
            'size': position['size'],
            'signal': position['signal'],
            'pnl': pnl,
            'pnl_pct': pnl_pct,
            'entry_time': position['entry_time'],
            'exit_time': timestamp,
            'duration': timestamp - position['entry_time']
        }

        self.daily_trades.append(trade)

        # Remove from open positions
        self.open_positions.remove(position)

        logger.info(f"Position closed: P&L ${pnl:.2f} ({pnl_pct:.2f}%)")
        logger.info(f"Current capital: ${self.current_capital:,.2f}")

        return trade

    def _check_max_drawdown(self) -> bool:
        """Check if maximum drawdown limit has been reached."""
        current_drawdown = ((self.peak_capital - self.current_capital) / self.peak_capital) * 100
        return current_drawdown < self.max_drawdown_pct

    def _check_max_daily_loss(self) -> bool:
        """Check if maximum daily loss limit has been reached."""
        daily_loss = ((self.daily_start_capital - self.current_capital) / self.daily_start_capital) * 100
        return daily_loss < self.max_daily_loss_pct

    def _check_daily_reset(self):
        """Reset daily tracking if it's a new day."""
        today = datetime.now().date()
        if today > self.last_reset_date:
            self.daily_start_capital = self.current_capital
            self.daily_trades = []
            self.last_reset_date = today
            logger.info(f"Daily reset: Starting capital ${self.current_capital:,.2f}")

    def get_risk_summary(self) -> Dict[str, Any]:
        """
        Get current risk metrics summary.

        Returns:
            Dictionary with risk metrics
        """
        current_drawdown = ((self.peak_capital - self.current_capital) / self.peak_capital) * 100
        daily_pnl = self.current_capital - self.daily_start_capital
        daily_pnl_pct = (daily_pnl / self.daily_start_capital) * 100

        return {
            'current_capital': self.current_capital,
            'peak_capital': self.peak_capital,
            'drawdown_pct': current_drawdown,
            'daily_pnl': daily_pnl,
            'daily_pnl_pct': daily_pnl_pct,
            'open_positions': len(self.open_positions),
            'daily_trades': len(self.daily_trades),
            'can_trade': self.can_trade(1)  # Check if trading is allowed
        }
