"""Autonomous trading engine that runs continuously."""

import time
import schedule
from datetime import datetime
from typing import Dict, Any
from loguru import logger
from ..data.data_fetcher import DataFetcher
from ..strategies.base_strategy import BaseStrategy
from ..risk_management.risk_manager import RiskManager
from .order_executor import OrderExecutor


class AutonomousTrader:
    """
    Autonomous trading system that continuously monitors markets and executes trades.
    """

    def __init__(
        self,
        strategy: BaseStrategy,
        config: Dict[str, Any]
    ):
        """
        Initialize the autonomous trader.

        Args:
            strategy: Trading strategy instance
            config: Configuration dictionary
        """
        self.strategy = strategy
        self.config = config
        self.execution_config = config.get('execution', {})

        # Initialize components
        self.data_fetcher = DataFetcher(config)
        self.risk_manager = RiskManager(config)
        self.order_executor = OrderExecutor(config)

        # Execution parameters
        self.check_interval = self.execution_config.get('check_interval', 60)
        self.trading_mode = config['trading'].get('trading_mode', 'paper')

        # State
        self.is_running = False
        self.last_check_time = None
        self.iteration_count = 0

        logger.info(f"AutonomousTrader initialized in {self.trading_mode} mode")
        logger.info(f"Check interval: {self.check_interval} seconds")

    def start(self):
        """Start the autonomous trading system."""
        logger.info("="*60)
        logger.info("AUTONOMOUS GOLD/USD TRADING SYSTEM STARTED")
        logger.info("="*60)
        logger.info(f"Strategy: {self.strategy.name}")
        logger.info(f"Mode: {self.trading_mode}")
        logger.info(f"Initial Capital: ${self.risk_manager.initial_capital:,.2f}")
        logger.info("="*60)

        self.is_running = True

        try:
            # Run initial check immediately
            self._trading_loop()

            # Schedule periodic checks
            schedule.every(self.check_interval).seconds.do(self._trading_loop)

            # Main loop
            while self.is_running:
                schedule.run_pending()
                time.sleep(1)

        except KeyboardInterrupt:
            logger.info("Received interrupt signal, shutting down...")
            self.stop()
        except Exception as e:
            logger.error(f"Fatal error in trading loop: {e}", exc_info=True)
            self.stop()

    def stop(self):
        """Stop the autonomous trading system."""
        logger.info("Stopping autonomous trader...")

        # Close all open positions
        if len(self.risk_manager.open_positions) > 0:
            logger.info(f"Closing {len(self.risk_manager.open_positions)} open positions...")
            try:
                current_price, _ = self.data_fetcher.get_current_price()
                for position in list(self.risk_manager.open_positions):
                    self.order_executor.close_position(position, current_price, datetime.now())
                    self.risk_manager.close_position(position, current_price, datetime.now())
            except Exception as e:
                logger.error(f"Error closing positions: {e}")

        self.is_running = False
        self._print_session_summary()
        logger.info("Autonomous trader stopped")

    def _trading_loop(self):
        """Main trading logic executed on each iteration."""
        try:
            self.iteration_count += 1
            self.last_check_time = datetime.now()

            logger.info(f"\n{'='*60}")
            logger.info(f"Iteration #{self.iteration_count} - {self.last_check_time}")
            logger.info(f"{'='*60}")

            # Fetch latest data
            logger.info("Fetching latest market data...")
            data = self.data_fetcher.fetch_live_data(
                period='5d',
                interval=self.config['data'].get('interval', '1h')
            )

            if len(data) == 0:
                logger.warning("No data received, skipping iteration")
                return

            current_price = data['close'].iloc[-1]
            logger.info(f"Current Gold price: ${current_price:.2f}")

            # Update existing positions
            self._update_positions(current_price)

            # Generate trading signal
            logger.info("Evaluating strategy...")
            data_with_indicators = self.strategy.calculate_indicators(data)
            signal = self.strategy.generate_signal(data_with_indicators)

            logger.info(f"Signal: {self.strategy.get_signal_description(signal)}")

            # Check if we can trade
            if signal != 0 and self.risk_manager.can_trade(signal):
                self._execute_trade(signal, current_price)
            elif signal != 0:
                logger.warning("Trading signal generated but trading not allowed (risk limits)")

            # Print current status
            self._print_status(current_price)

        except Exception as e:
            logger.error(f"Error in trading loop: {e}", exc_info=True)

    def _update_positions(self, current_price: float):
        """
        Update existing positions and close if stop loss/take profit triggered.

        Args:
            current_price: Current market price
        """
        if len(self.risk_manager.open_positions) == 0:
            return

        logger.info(f"Updating {len(self.risk_manager.open_positions)} open positions...")

        positions_to_close = self.risk_manager.update_positions(
            current_price,
            datetime.now()
        )

        # Close triggered positions
        for position in positions_to_close:
            logger.info(f"Closing position due to trigger: {position['id']}")
            trade = self.risk_manager.close_position(
                position,
                current_price,
                datetime.now()
            )

            # Execute actual order (if in live mode)
            if self.trading_mode == 'live':
                self.order_executor.close_position(position, current_price, datetime.now())

            logger.info(f"Position closed: P&L ${trade['pnl']:.2f} ({trade['pnl_pct']:.2f}%)")

    def _execute_trade(self, signal: int, current_price: float):
        """
        Execute a trade based on signal.

        Args:
            signal: Trading signal
            current_price: Current market price
        """
        logger.info(f"Executing {self.strategy.get_signal_description(signal)} order...")

        # Calculate position size
        position_size = self.risk_manager.calculate_position_size(signal, current_price)

        if position_size <= 0:
            logger.warning("Position size is zero, skipping trade")
            return

        # Open position
        position = self.risk_manager.open_position(
            signal,
            current_price,
            position_size,
            datetime.now()
        )

        logger.info(f"Position opened: {signal} {position_size:.4f} units @ ${current_price:.2f}")
        logger.info(f"  Stop Loss: ${position['stop_loss']:.2f}")
        logger.info(f"  Take Profit: ${position['take_profit']:.2f}")

        # Execute actual order (if in live mode)
        if self.trading_mode == 'live':
            try:
                self.order_executor.open_position(position)
                logger.info("Live order executed successfully")
            except Exception as e:
                logger.error(f"Failed to execute live order: {e}")
                # Rollback position
                self.risk_manager.open_positions.remove(position)

    def _print_status(self, current_price: float):
        """
        Print current status summary.

        Args:
            current_price: Current market price
        """
        risk_summary = self.risk_manager.get_risk_summary()

        logger.info("\n--- Current Status ---")
        logger.info(f"Current Capital: ${risk_summary['current_capital']:,.2f}")
        logger.info(f"Peak Capital: ${risk_summary['peak_capital']:,.2f}")
        logger.info(f"Drawdown: {risk_summary['drawdown_pct']:.2f}%")
        logger.info(f"Daily P&L: ${risk_summary['daily_pnl']:,.2f} ({risk_summary['daily_pnl_pct']:.2f}%)")
        logger.info(f"Open Positions: {risk_summary['open_positions']}")
        logger.info(f"Daily Trades: {risk_summary['daily_trades']}")
        logger.info(f"Trading Allowed: {risk_summary['can_trade']}")

        # Print open positions
        if len(self.risk_manager.open_positions) > 0:
            logger.info("\n--- Open Positions ---")
            for pos in self.risk_manager.open_positions:
                unrealized_pnl = 0
                if pos['signal'] == 1:  # Long
                    unrealized_pnl = (current_price - pos['entry_price']) * pos['size']
                else:  # Short
                    unrealized_pnl = (pos['entry_price'] - current_price) * pos['size']

                unrealized_pnl_pct = (unrealized_pnl / (pos['entry_price'] * pos['size'])) * 100

                logger.info(f"Position #{pos['id']}: {pos['signal']} {pos['size']:.4f} units @ ${pos['entry_price']:.2f}")
                logger.info(f"  Unrealized P&L: ${unrealized_pnl:.2f} ({unrealized_pnl_pct:.2f}%)")
                logger.info(f"  Stop Loss: ${pos['stop_loss']:.2f}, Take Profit: ${pos['take_profit']:.2f}")

    def _print_session_summary(self):
        """Print summary of trading session."""
        logger.info("\n" + "="*60)
        logger.info("TRADING SESSION SUMMARY")
        logger.info("="*60)

        risk_summary = self.risk_manager.get_risk_summary()

        logger.info(f"Initial Capital: ${self.risk_manager.initial_capital:,.2f}")
        logger.info(f"Final Capital: ${risk_summary['current_capital']:,.2f}")

        total_pnl = risk_summary['current_capital'] - self.risk_manager.initial_capital
        total_pnl_pct = (total_pnl / self.risk_manager.initial_capital) * 100

        logger.info(f"Total P&L: ${total_pnl:,.2f} ({total_pnl_pct:.2f}%)")
        logger.info(f"Max Drawdown: {risk_summary['drawdown_pct']:.2f}%")
        logger.info(f"Total Iterations: {self.iteration_count}")

        logger.info("="*60)

    def run_once(self) -> Dict[str, Any]:
        """
        Run a single iteration of the trading loop (useful for testing).

        Returns:
            Dictionary with iteration results
        """
        self._trading_loop()

        risk_summary = self.risk_manager.get_risk_summary()

        return {
            'timestamp': self.last_check_time,
            'iteration': self.iteration_count,
            'capital': risk_summary['current_capital'],
            'open_positions': risk_summary['open_positions'],
            'daily_pnl': risk_summary['daily_pnl']
        }


if __name__ == "__main__":
    from src.utils.config_loader import load_config
    from src.strategies.gold_momentum_strategy import GoldMomentumStrategy

    # Test autonomous trader
    config = load_config()
    strategy = GoldMomentumStrategy(config)
    trader = AutonomousTrader(strategy, config)

    # Run once for testing
    result = trader.run_once()
    print(f"\nIteration result: {result}")
