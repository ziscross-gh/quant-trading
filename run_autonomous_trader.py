#!/usr/bin/env python3
"""
Run the autonomous Gold/USD trading system.

Usage:
    python run_autonomous_trader.py
"""

import sys
import argparse
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent))

from src.utils.config_loader import load_config
from src.utils.logger import setup_logger
from src.strategies.gold_momentum_strategy import GoldMomentumStrategy
from src.execution.autonomous_trader import AutonomousTrader


def main():
    """Main autonomous trader execution."""
    # Parse command line arguments
    parser = argparse.ArgumentParser(
        description='Run autonomous Gold/USD trading system'
    )
    parser.add_argument(
        '--mode',
        choices=['paper', 'live'],
        default='paper',
        help='Trading mode: paper (simulation) or live (real money)'
    )
    parser.add_argument(
        '--config',
        type=str,
        default=None,
        help='Path to custom configuration file'
    )
    parser.add_argument(
        '--once',
        action='store_true',
        help='Run once and exit (for testing)'
    )

    args = parser.parse_args()

    # Load configuration
    print("Loading configuration...")
    config = load_config(args.config)

    # Override trading mode if specified
    if args.mode:
        config['trading']['trading_mode'] = args.mode

    # Setup logger
    logger = setup_logger(
        log_level=config['logging']['level'],
        log_dir=config['logging']['log_dir'],
        log_name='autonomous_trader'
    )

    # Verify trading mode
    trading_mode = config['trading']['trading_mode']
    if trading_mode == 'live':
        logger.warning("="*60)
        logger.warning("LIVE TRADING MODE SELECTED")
        logger.warning("Real money will be at risk!")
        logger.warning("="*60)
        response = input("Are you sure you want to proceed? (yes/no): ")
        if response.lower() != 'yes':
            logger.info("Exiting...")
            sys.exit(0)
    else:
        logger.info("Running in PAPER TRADING mode (simulation)")

    # Initialize strategy
    logger.info("Initializing trading strategy...")
    strategy = GoldMomentumStrategy(config)

    # Initialize autonomous trader
    logger.info("Initializing autonomous trader...")
    trader = AutonomousTrader(strategy, config)

    # Run
    if args.once:
        logger.info("Running single iteration (test mode)...")
        result = trader.run_once()
        logger.info(f"Iteration result: {result}")
    else:
        logger.info("Starting autonomous trading system...")
        logger.info("Press Ctrl+C to stop")
        trader.start()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nTrading system stopped by user")
        sys.exit(0)
    except Exception as e:
        print(f"\nError running autonomous trader: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
