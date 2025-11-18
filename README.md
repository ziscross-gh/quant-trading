# 🏆 Autonomous Gold/USD Quantitative Trading System

A sophisticated, fully autonomous quantitative trading system for Gold/USD (XAU/USD) available in both **Python** and **Rust**. This system implements momentum-based strategies with comprehensive risk management, backtesting capabilities, and autonomous execution.

## 🔥 **NEW: Rust Implementation Available!**

A high-performance Rust version is now available in the `rust/` directory with **10-100x performance improvements**!
- ⚡ Blazing fast backtesting and strategy execution
- 🦀 Memory-safe with Rust's ownership system
- 🚀 Async/await based concurrent operations
- 📦 See `rust/README.md` for details

---

## Python Version

This directory contains the Python implementation. For the high-performance Rust version, see `rust/` directory.

## ✨ Features

- **Autonomous Trading**: Continuously monitors markets and executes trades automatically
- **Advanced Strategy**: Momentum-based strategy using multiple technical indicators:
  - Moving Average Crossover (Fast/Slow MA)
  - Relative Strength Index (RSI)
  - Bollinger Bands
  - Volume confirmation
  - Trend strength analysis
- **Comprehensive Risk Management**:
  - Position sizing based on risk parameters
  - Stop loss and take profit levels
  - Trailing stops
  - Maximum drawdown limits
  - Daily loss limits
- **Backtesting Framework**: Test strategies on historical data with detailed performance metrics
- **Paper Trading**: Test the system in simulation mode before going live
- **Configurable**: YAML-based configuration for easy customization
- **Logging**: Comprehensive logging with daily rotation and error tracking

## 📋 Requirements

- Python 3.8+
- See `requirements.txt` for all dependencies

## 🚀 Quick Start

### 1. Installation

```bash
# Clone the repository
git clone <repository-url>
cd quant-trading

# Create virtual environment (recommended)
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install dependencies
pip install -r requirements.txt
```

### 2. Configuration

Copy the example environment file and configure your API keys:

```bash
cp .env.example .env
# Edit .env with your API keys
```

Customize the trading parameters in `config/config.yaml`:
- Adjust strategy parameters (MA periods, RSI thresholds, etc.)
- Set risk management limits
- Configure capital and position sizing
- Set data source and intervals

### 3. Run Backtest

Test the strategy on historical data:

```bash
python run_backtest.py
```

This will:
- Fetch historical Gold/USD data
- Run the strategy on past data
- Generate performance metrics
- Save results to `results/` directory
- Display equity curve and trading signals

### 4. Run Autonomous Trader

#### Paper Trading (Simulation)

```bash
python run_autonomous_trader.py --mode paper
```

#### Live Trading (Real Money - Use with Caution!)

```bash
python run_autonomous_trader.py --mode live
```

#### Single Iteration (Testing)

```bash
python run_autonomous_trader.py --once
```

## 📁 Project Structure

```
quant-trading/
├── src/
│   ├── data/                 # Data fetching and caching
│   │   ├── data_fetcher.py   # Market data acquisition
│   │   └── data_cache.py     # Data caching utilities
│   ├── strategies/           # Trading strategies
│   │   ├── base_strategy.py  # Base strategy class
│   │   └── gold_momentum_strategy.py  # Gold momentum strategy
│   ├── backtesting/          # Backtesting framework
│   │   ├── backtest_engine.py  # Backtesting engine
│   │   └── performance_metrics.py  # Performance calculations
│   ├── execution/            # Order execution
│   │   ├── autonomous_trader.py  # Autonomous trading engine
│   │   └── order_executor.py     # Order execution logic
│   ├── risk_management/      # Risk management
│   │   ├── risk_manager.py   # Risk control system
│   │   └── position_sizer.py # Position sizing utilities
│   └── utils/                # Utilities
│       ├── config_loader.py  # Configuration management
│       └── logger.py         # Logging setup
├── config/
│   └── config.yaml           # Main configuration file
├── data/                     # Data storage
│   ├── raw/                  # Raw market data
│   └── processed/            # Processed data
├── logs/                     # Log files
├── results/                  # Backtest results
├── notebooks/                # Jupyter notebooks for analysis
├── tests/                    # Unit tests
├── run_backtest.py          # Backtest runner
├── run_autonomous_trader.py # Autonomous trader runner
├── requirements.txt         # Python dependencies
├── .env.example            # Example environment variables
└── README.md               # This file
```

## 🎯 Strategy Overview

### Gold Momentum Strategy

The strategy uses multiple technical indicators to identify momentum in Gold/USD prices:

1. **Moving Average Crossover**:
   - Fast MA (20-period) crosses Slow MA (50-period)
   - Generates primary trend signals

2. **RSI (Relative Strength Index)**:
   - Identifies overbought (>70) and oversold (<30) conditions
   - Confirms momentum and potential reversals

3. **Bollinger Bands**:
   - Measures volatility
   - Identifies breakout and mean reversion opportunities

4. **Volume Confirmation**:
   - Confirms signals with volume analysis
   - Filters false breakouts

5. **Trend Strength**:
   - Measures the strength of the current trend
   - Helps determine position sizing

### Signal Generation

Signals are generated based on a scoring system:
- Multiple indicators must align for a trade signal
- Buy signal requires 3+ bullish indicators
- Sell signal requires 3+ bearish indicators
- Otherwise, hold position

## 🛡️ Risk Management

The system implements comprehensive risk controls:

- **Position Sizing**: Based on risk per trade (default 1% of capital)
- **Stop Loss**: Automatic stop loss at 2% per trade
- **Take Profit**: Automatic take profit at 5% per trade
- **Trailing Stop**: Dynamic trailing stop at 1.5%
- **Maximum Drawdown**: System halts at 15% drawdown
- **Daily Loss Limit**: Maximum 5% loss per day
- **Position Limits**: Maximum 3 concurrent positions

## 📊 Performance Metrics

The backtesting framework calculates:

- **Return Metrics**: Total return, annualized return, daily returns
- **Risk Metrics**: Sharpe ratio, Sortino ratio, Calmar ratio, max drawdown
- **Win/Loss Metrics**: Win rate, profit factor, average win/loss
- **Trade Statistics**: Number of trades, consecutive wins/losses, trade duration

## 📈 Example Usage

### Backtest Example

```python
from src.utils.config_loader import load_config
from src.data.data_fetcher import DataFetcher
from src.strategies.gold_momentum_strategy import GoldMomentumStrategy
from src.backtesting.backtest_engine import BacktestEngine

# Load configuration
config = load_config()

# Initialize components
data_fetcher = DataFetcher(config)
strategy = GoldMomentumStrategy(config)
backtest = BacktestEngine(strategy, config)

# Fetch data and run backtest
data = data_fetcher.fetch_historical_data(start_date='2020-01-01', end_date='2024-12-31')
results = backtest.run(data)

# View results
print(results['metrics'])
backtest.plot_results()
```

### Live Trading Example

```python
from src.utils.config_loader import load_config
from src.strategies.gold_momentum_strategy import GoldMomentumStrategy
from src.execution.autonomous_trader import AutonomousTrader

# Load configuration
config = load_config()
config['trading']['trading_mode'] = 'paper'  # Use 'live' for real trading

# Initialize and start
strategy = GoldMomentumStrategy(config)
trader = AutonomousTrader(strategy, config)
trader.start()
```

## ⚙️ Configuration

Key configuration parameters in `config/config.yaml`:

```yaml
trading:
  symbol: "GC=F"           # Gold Futures
  initial_capital: 100000   # Starting capital
  position_size: 0.1        # 10% per trade
  trading_mode: "paper"     # paper or live

strategy:
  fast_ma: 20              # Fast moving average
  slow_ma: 50              # Slow moving average
  rsi_period: 14           # RSI period
  rsi_oversold: 30         # RSI oversold threshold
  rsi_overbought: 70       # RSI overbought threshold

risk:
  max_drawdown_pct: 15     # Maximum drawdown
  stop_loss_pct: 2         # Stop loss per trade
  take_profit_pct: 5       # Take profit per trade
  risk_per_trade_pct: 1    # Risk per trade
```

## 🔐 Security

- Store API keys in `.env` file (never commit this file)
- Use paper trading mode for testing
- Start with small capital in live mode
- Monitor the system regularly
- Set appropriate risk limits

## 📝 Logging

Logs are stored in the `logs/` directory:
- `trading_YYYY-MM-DD.log`: Daily trading logs
- `trading_errors_YYYY-MM-DD.log`: Error logs
- Automatic rotation and compression
- 30-day retention for regular logs, 90 days for errors

## 🧪 Testing

Run tests:

```bash
pytest tests/
```

## 🤝 Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## ⚠️ Disclaimer

**IMPORTANT**: This software is for educational purposes only. Trading involves substantial risk of loss. Past performance is not indicative of future results. The authors are not responsible for any financial losses incurred through the use of this software.

- Always test thoroughly in paper trading mode first
- Start with small amounts in live trading
- Never trade with money you cannot afford to lose
- Consult with a financial advisor before trading

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 📧 Contact

For questions, issues, or contributions, please open an issue on GitHub.

## 🙏 Acknowledgments

- Market data provided by Yahoo Finance
- Built with Python and various open-source libraries
- Inspired by quantitative trading research and best practices

---

**Happy Trading! 📈💰**
