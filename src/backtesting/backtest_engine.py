"""Backtesting engine for testing trading strategies."""

import pandas as pd
import numpy as np
from typing import Dict, Any, List
from datetime import datetime
from loguru import logger
from ..strategies.base_strategy import BaseStrategy
from ..risk_management.risk_manager import RiskManager
from .performance_metrics import PerformanceMetrics


class BacktestEngine:
    """
    Backtesting engine for evaluating trading strategies on historical data.
    """

    def __init__(
        self,
        strategy: BaseStrategy,
        config: Dict[str, Any]
    ):
        """
        Initialize the backtest engine.

        Args:
            strategy: Trading strategy instance
            config: Configuration dictionary
        """
        self.strategy = strategy
        self.config = config
        self.backtest_config = config.get('backtest', {})

        # Backtest parameters
        self.commission = self.backtest_config.get('commission', 0.001)
        self.slippage = self.backtest_config.get('slippage', 0.0005)

        # Initialize risk manager
        self.risk_manager = RiskManager(config)

        # Results tracking
        self.trades = []
        self.equity_curve = []
        self.signals = []

        logger.info(f"BacktestEngine initialized for strategy: {strategy.name}")
        logger.info(f"Commission: {self.commission*100:.3f}%, Slippage: {self.slippage*100:.3f}%")

    def run(self, data: pd.DataFrame) -> Dict[str, Any]:
        """
        Run backtest on historical data.

        Args:
            data: DataFrame with OHLCV data

        Returns:
            Dictionary with backtest results
        """
        logger.info(f"Starting backtest from {data.index[0]} to {data.index[-1]}")
        logger.info(f"Total data points: {len(data)}")

        # Reset state
        self.trades = []
        self.equity_curve = []
        self.signals = []
        self.risk_manager.current_capital = self.risk_manager.initial_capital
        self.risk_manager.peak_capital = self.risk_manager.initial_capital
        self.risk_manager.open_positions = []

        # Calculate indicators for entire dataset
        data_with_indicators = self.strategy.calculate_indicators(data)

        # Get the minimum required lookback period
        lookback = max(
            self.strategy.strategy_config.get('slow_ma', 50),
            self.strategy.strategy_config.get('rsi_period', 14)
        )

        # Iterate through data
        for i in range(lookback, len(data_with_indicators)):
            # Get data up to current point
            current_data = data_with_indicators.iloc[:i+1]
            current_price = current_data['close'].iloc[-1]
            current_time = current_data.index[-1]

            # Update existing positions
            positions_to_close = self.risk_manager.update_positions(current_price, current_time)

            # Close triggered positions
            for position in positions_to_close:
                exit_price = self._apply_slippage(current_price, position['signal'] * -1)
                trade = self.risk_manager.close_position(position, exit_price, current_time)
                trade['commission'] = self._calculate_commission(exit_price, position['size'])
                trade['slippage'] = abs(exit_price - current_price)
                self.trades.append(trade)

            # Generate signal
            signal = self.strategy.generate_signal(current_data)
            self.signals.append({
                'timestamp': current_time,
                'signal': signal,
                'price': current_price
            })

            # Check if we can trade
            if signal != 0 and self.risk_manager.can_trade(signal):
                # Calculate position size
                entry_price = self._apply_slippage(current_price, signal)
                position_size = self.risk_manager.calculate_position_size(signal, entry_price)

                if position_size > 0:
                    # Open position
                    position = self.risk_manager.open_position(
                        signal, entry_price, position_size, current_time
                    )

                    # Record entry trade (for tracking)
                    logger.debug(f"Position opened at {current_time}: {signal} @ ${entry_price:.2f}")

            # Record equity
            equity = self._calculate_equity(current_price)
            self.equity_curve.append({
                'timestamp': current_time,
                'equity': equity,
                'price': current_price
            })

        # Close any remaining positions at the end
        final_price = data_with_indicators['close'].iloc[-1]
        final_time = data_with_indicators.index[-1]

        for position in list(self.risk_manager.open_positions):
            exit_price = self._apply_slippage(final_price, position['signal'] * -1)
            trade = self.risk_manager.close_position(position, exit_price, final_time)
            trade['commission'] = self._calculate_commission(exit_price, position['size'])
            trade['slippage'] = abs(exit_price - final_price)
            self.trades.append(trade)
            logger.info(f"Position closed at end of backtest: ${trade['pnl']:.2f}")

        # Calculate performance metrics
        metrics = self._calculate_metrics()

        logger.info(f"Backtest completed. Total trades: {len(self.trades)}")
        logger.info(f"Final capital: ${self.risk_manager.current_capital:,.2f}")

        return {
            'metrics': metrics,
            'trades': self.trades,
            'equity_curve': pd.DataFrame(self.equity_curve),
            'signals': pd.DataFrame(self.signals),
            'final_capital': self.risk_manager.current_capital
        }

    def _apply_slippage(self, price: float, signal: int) -> float:
        """
        Apply slippage to entry/exit price.

        Args:
            price: Original price
            signal: Trading signal (1 for buy, -1 for sell)

        Returns:
            Adjusted price with slippage
        """
        if signal == 1:  # Buying - pay more
            return price * (1 + self.slippage)
        elif signal == -1:  # Selling - receive less
            return price * (1 - self.slippage)
        return price

    def _calculate_commission(self, price: float, size: float) -> float:
        """
        Calculate commission cost.

        Args:
            price: Trade price
            size: Position size

        Returns:
            Commission cost
        """
        return price * size * self.commission

    def _calculate_equity(self, current_price: float) -> float:
        """
        Calculate current total equity including open positions.

        Args:
            current_price: Current market price

        Returns:
            Total equity
        """
        equity = self.risk_manager.current_capital

        # Add unrealized P&L from open positions
        for position in self.risk_manager.open_positions:
            if position['signal'] == 1:  # Long
                unrealized_pnl = (current_price - position['entry_price']) * position['size']
            else:  # Short
                unrealized_pnl = (position['entry_price'] - current_price) * position['size']
            equity += unrealized_pnl

        return equity

    def _calculate_metrics(self) -> Dict[str, Any]:
        """
        Calculate performance metrics from backtest results.

        Returns:
            Dictionary of performance metrics
        """
        if len(self.trades) == 0:
            logger.warning("No trades executed during backtest")
            return {}

        trades_df = pd.DataFrame(self.trades)
        equity_df = pd.DataFrame(self.equity_curve)

        # Use PerformanceMetrics class
        perf = PerformanceMetrics(
            trades_df,
            equity_df,
            self.risk_manager.initial_capital
        )

        return perf.calculate_all_metrics()

    def get_trade_summary(self) -> pd.DataFrame:
        """
        Get summary of all trades.

        Returns:
            DataFrame with trade details
        """
        if len(self.trades) == 0:
            return pd.DataFrame()

        return pd.DataFrame(self.trades)

    def plot_results(self):
        """
        Plot backtest results (equity curve, drawdown, etc.).
        """
        try:
            import matplotlib.pyplot as plt

            if len(self.equity_curve) == 0:
                logger.warning("No equity data to plot")
                return

            equity_df = pd.DataFrame(self.equity_curve)
            equity_df.set_index('timestamp', inplace=True)

            fig, axes = plt.subplots(2, 1, figsize=(12, 8))

            # Equity curve
            axes[0].plot(equity_df.index, equity_df['equity'], label='Equity', linewidth=2)
            axes[0].set_title('Equity Curve')
            axes[0].set_ylabel('Equity ($)')
            axes[0].legend()
            axes[0].grid(True, alpha=0.3)

            # Price chart with signals
            axes[1].plot(equity_df.index, equity_df['price'], label='Gold Price', linewidth=1)

            # Add buy/sell signals
            signals_df = pd.DataFrame(self.signals)
            if len(signals_df) > 0:
                buy_signals = signals_df[signals_df['signal'] == 1]
                sell_signals = signals_df[signals_df['signal'] == -1]

                axes[1].scatter(buy_signals['timestamp'], buy_signals['price'],
                              marker='^', color='green', s=100, label='Buy', alpha=0.7)
                axes[1].scatter(sell_signals['timestamp'], sell_signals['price'],
                              marker='v', color='red', s=100, label='Sell', alpha=0.7)

            axes[1].set_title('Gold Price with Trading Signals')
            axes[1].set_xlabel('Date')
            axes[1].set_ylabel('Price ($)')
            axes[1].legend()
            axes[1].grid(True, alpha=0.3)

            plt.tight_layout()
            plt.show()

        except ImportError:
            logger.warning("matplotlib not available for plotting")
