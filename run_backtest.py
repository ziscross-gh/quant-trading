#!/usr/bin/env python3
"""
Run backtest for Gold/USD trading strategy.

Usage:
    python run_backtest.py
"""

import sys
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent))

from src.utils.config_loader import load_config
from src.utils.logger import setup_logger
from src.data.data_fetcher import DataFetcher
from src.strategies.gold_momentum_strategy import GoldMomentumStrategy
from src.backtesting.backtest_engine import BacktestEngine
from src.backtesting.performance_metrics import PerformanceMetrics


def main():
    """Main backtest execution."""
    # Load configuration
    print("Loading configuration...")
    config = load_config()

    # Setup logger
    logger = setup_logger(
        log_level=config['logging']['level'],
        log_dir=config['logging']['log_dir'],
        log_name='backtest'
    )

    logger.info("="*60)
    logger.info("GOLD/USD TRADING STRATEGY BACKTEST")
    logger.info("="*60)

    # Initialize components
    logger.info("Initializing components...")
    data_fetcher = DataFetcher(config)
    strategy = GoldMomentumStrategy(config)

    # Fetch historical data
    logger.info("Fetching historical data...")
    backtest_config = config.get('backtest', {})
    data = data_fetcher.fetch_historical_data(
        start_date=backtest_config.get('start_date'),
        end_date=backtest_config.get('end_date'),
        interval=config['data'].get('interval', '1d')
    )

    logger.info(f"Data loaded: {len(data)} periods")
    logger.info(f"Date range: {data.index[0]} to {data.index[-1]}")

    # Run backtest
    logger.info("Running backtest...")
    backtest = BacktestEngine(strategy, config)
    results = backtest.run(data)

    # Print performance metrics
    print("\n" + "="*60)
    print("BACKTEST RESULTS")
    print("="*60)

    metrics = results['metrics']
    for key, value in metrics.items():
        if isinstance(value, float):
            if 'pct' in key or 'rate' in key or 'ratio' in key:
                print(f"{key}: {value:.2f}")
            else:
                print(f"{key}: {value:,.2f}")
        else:
            print(f"{key}: {value}")

    # Save results
    logger.info("\nSaving backtest results...")
    results_dir = Path('results')
    results_dir.mkdir(exist_ok=True)

    # Save trades
    trades_df = backtest.get_trade_summary()
    if len(trades_df) > 0:
        trades_file = results_dir / 'backtest_trades.csv'
        trades_df.to_csv(trades_file)
        logger.info(f"Trades saved to: {trades_file}")

    # Save equity curve
    equity_df = results['equity_curve']
    if len(equity_df) > 0:
        equity_file = results_dir / 'backtest_equity_curve.csv'
        equity_df.to_csv(equity_file)
        logger.info(f"Equity curve saved to: {equity_file}")

    # Plot results (if matplotlib available)
    try:
        logger.info("Generating plots...")
        backtest.plot_results()
    except Exception as e:
        logger.warning(f"Could not generate plots: {e}")

    logger.info("\n" + "="*60)
    logger.info("Backtest completed successfully!")
    logger.info("="*60)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nBacktest interrupted by user")
        sys.exit(0)
    except Exception as e:
        print(f"\nError running backtest: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
