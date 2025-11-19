# Gold/USD Autonomous Trading Bot - Complete Setup Guide

This guide will walk you through setting up the autonomous Gold/USD trading system from scratch.

## Table of Contents
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [OANDA Broker Setup](#oanda-broker-setup)
- [Database Setup](#database-setup)
- [Telegram Notifications](#telegram-notifications)
- [Email Notifications](#email-notifications)
- [Configuration](#configuration)
- [Running the System](#running-the-system)
- [Monitoring](#monitoring)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Software
- **Rust** (1.70+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **PostgreSQL** (14+): For trade history and analytics
- **Git**: For cloning the repository

### Recommended
- **Telegram** account for notifications
- **OANDA** demo account (free) for paper trading

## Quick Start

### 1. Clone and Build

```bash
# Clone the repository
git clone https://github.com/yourusername/quant-trading.git
cd quant-trading/rust

# Build the project (optimized)
cargo build --release

# This will take a few minutes on first build
```

### 2. Set Up Environment Variables

Create a `.env` file in the `rust/` directory:

```bash
# Copy the example
cp .env.example .env

# Edit with your favorite editor
nano .env
```

Minimum configuration to start:

```bash
# Trading Mode
TRADING_MODE=paper

# OANDA Broker (get from https://www.oanda.com/demo-account/)
OANDA_API_TOKEN=your_practice_token_here
OANDA_ACCOUNT_ID=your_account_id_here

# Optional but recommended
DATABASE_URL=postgresql://localhost/trading_db
TELEGRAM_BOT_TOKEN=your_telegram_bot_token
TELEGRAM_CHAT_ID=your_chat_id
```

### 3. Run Backtest

```bash
cargo run --bin backtest --release

# You should see:
# ✅ Backtest complete!
# Total trades: 45
# Win rate: 62.2%
# Total PnL: $12,345.67
```

### 4. Start Paper Trading

```bash
cargo run --bin trade --release

# The bot will:
# - Fetch Gold/USD prices every hour
# - Analyze news sentiment
# - Generate trading signals
# - Execute trades via OANDA demo account
# - Send Telegram notifications
```

## OANDA Broker Setup

### Step 1: Create Demo Account

1. Go to https://www.oanda.com/demo-account/
2. Fill out the registration form
3. Verify your email
4. Log in to your account

### Step 2: Generate API Token

1. Log in to OANDA
2. Go to **Account Settings** → **API Access**
3. Click **Generate Personal Access Token**
4. Copy the token (you'll only see it once!)
5. Save it securely

### Step 3: Find Your Account ID

1. In OANDA dashboard, look for your **Account ID**
2. It looks like: `001-004-1234567-001`
3. Copy this ID

### Step 4: Configure in `.env`

```bash
# Practice (Demo) Environment
OANDA_API_TOKEN=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
OANDA_ACCOUNT_ID=001-004-1234567-001

# For live trading (when ready):
# TRADING_MODE=live
# Use your live API token and account ID
```

### Testing Connection

```bash
# Test OANDA connection
cargo run --bin trade --release

# You should see:
# INFO OANDA broker initialized in PRACTICE mode (account: 001-004-1234567-001)
# INFO Successfully connected to OANDA
```

## Database Setup

### Step 1: Install PostgreSQL

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
```

**macOS:**
```bash
brew install postgresql
brew services start postgresql
```

**Windows:**
Download from https://www.postgresql.org/download/windows/

### Step 2: Create Database

```bash
# Switch to postgres user (Linux)
sudo -u postgres psql

# Or just run psql (macOS/Windows)
psql postgres
```

```sql
-- Create database
CREATE DATABASE trading_db;

-- Create user (optional)
CREATE USER trader WITH PASSWORD 'your_password';
GRANT ALL PRIVILEGES ON DATABASE trading_db TO trader;

\q
```

### Step 3: Set Database URL

```bash
# In .env file
DATABASE_URL=postgresql://trader:your_password@localhost/trading_db

# Or for simpler local setup:
DATABASE_URL=postgresql://localhost/trading_db
```

### Step 4: Run Migrations

```bash
# Install sqlx CLI
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
cd rust
sqlx migrate run

# You should see:
# Applied 001_create_trades.sql
# Applied 002_create_equity_curve.sql
```

### Verify Database

```bash
psql trading_db

\dt  # List tables
# Should show: trades, equity_curve

SELECT * FROM trades LIMIT 5;
```

## Telegram Notifications

### Step 1: Create Bot

1. Open Telegram and search for **@BotFather**
2. Send `/newbot`
3. Choose a name: `My Trading Bot`
4. Choose a username: `my_gold_trading_bot`
5. Copy the API token

### Step 2: Get Chat ID

1. Send a message to your new bot
2. Visit: `https://api.telegram.org/bot<YOUR_BOT_TOKEN>/getUpdates`
3. Look for `"chat":{"id":123456789}`
4. Copy the chat ID

### Step 3: Configure

```bash
# In .env file
TELEGRAM_BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrsTUVwxyz
TELEGRAM_CHAT_ID=123456789
```

### Test Notifications

The bot will automatically send notifications when:
- ✅ New trade opened
- ✅ Trade closed with profit/loss
- ⚠️ High drawdown warning
- 🔴 System error
- ℹ️ Daily performance summary

## Email Notifications

### Step 1: Gmail Setup (Recommended)

1. Enable 2-Factor Authentication on your Google account
2. Generate an App Password:
   - Go to https://myaccount.google.com/apppasswords
   - Select "Mail" and your device
   - Copy the 16-character password

### Step 2: Configure

```bash
# In .env file
SMTP_SERVER=smtp.gmail.com
SMTP_USERNAME=your.email@gmail.com
SMTP_PASSWORD=your_16_char_app_password
EMAIL_FROM=your.email@gmail.com
EMAIL_TO=your.email@gmail.com
```

### For Other Email Providers

**Outlook/Hotmail:**
```bash
SMTP_SERVER=smtp-mail.outlook.com
```

**Yahoo:**
```bash
SMTP_SERVER=smtp.mail.yahoo.com
```

## Configuration

### Trading Configuration

Edit `config/config.yaml`:

```yaml
trading:
  symbol: "GC=F"                    # Gold futures
  symbol_alt: "XAUUSD"              # Alternative symbol
  initial_capital: 10000.0          # Starting capital
  trading_mode: paper               # paper or live

strategy:
  fast_ma: 20                       # Fast moving average
  slow_ma: 50                       # Slow moving average
  rsi_period: 14                    # RSI period
  rsi_oversold: 30                  # RSI oversold level
  rsi_overbought: 70                # RSI overbought level

risk:
  max_drawdown_pct: 15.0            # Stop trading at 15% drawdown
  stop_loss_pct: 2.0                # 2% stop loss per trade
  take_profit_pct: 5.0              # 5% take profit per trade
  risk_per_trade_pct: 1.0           # Risk 1% per trade

news:
  enabled: true                     # Enable news sentiment
  sentiment_weight: 0.3             # 30% weight on signals
  lookback_hours: 24                # Check last 24h of news
```

## Running the System

### Backtesting

```bash
# Run backtest with default config
cargo run --bin backtest --release

# Run with custom date range
# Edit config.yaml backtest section first
cargo run --bin backtest --release
```

### Paper Trading

```bash
# Start autonomous trader
cargo run --bin trade --release

# The system will:
# 1. Connect to OANDA demo account
# 2. Start monitoring Gold/USD prices
# 3. Fetch news and analyze sentiment
# 4. Generate trading signals
# 5. Execute trades
# 6. Send notifications
# 7. Log everything to database
```

### Live Trading (⚠️ Real Money!)

```bash
# In .env file
TRADING_MODE=live
OANDA_API_TOKEN=<your_live_token>
OANDA_ACCOUNT_ID=<your_live_account>

# Start with caution!
cargo run --bin trade --release
```

**Important:**
- Start small ($500-1000)
- Monitor closely for first week
- Review logs daily
- Adjust risk parameters conservatively

## Monitoring

### Real-time Logs

```bash
# Follow logs in real-time
tail -f logs/trading.log

# Filter for errors
tail -f logs/trading.log | grep ERROR

# Filter for trades
tail -f logs/trading.log | grep TRADE
```

### Database Queries

```bash
psql trading_db
```

```sql
-- Recent trades
SELECT * FROM trades ORDER BY entry_time DESC LIMIT 10;

-- Performance summary
SELECT
    COUNT(*) as total_trades,
    SUM(CASE WHEN pnl > 0 THEN 1 ELSE 0 END) as wins,
    ROUND(AVG(pnl), 2) as avg_pnl,
    ROUND(SUM(pnl), 2) as total_pnl
FROM trades;

-- Equity curve
SELECT timestamp, equity, price
FROM equity_curve
ORDER BY timestamp DESC
LIMIT 100;
```

### Telegram Status

Send `/status` to your bot to get:
- Current positions
- Today's P&L
- Account balance
- Win rate

## Troubleshooting

### "OANDA_API_TOKEN not set"

```bash
# Make sure .env file exists
ls -la .env

# Check it has the variable
cat .env | grep OANDA

# Load it manually
source .env
```

### "Database connection failed"

```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Test connection
psql -d trading_db -c "SELECT 1"

# Check DATABASE_URL format
echo $DATABASE_URL
```

### "No news fetched"

```bash
# Check internet connection
curl -I https://finance.yahoo.com

# Try with fewer symbols
# Edit config.yaml and reduce article_limit

# Check logs
tail -f logs/trading.log | grep "news"
```

### "Insufficient margin"

This means your account doesn't have enough funds:
1. Check account balance in OANDA
2. Reduce position_size in config
3. Increase initial_capital (demo account)

### "Telegram not working"

```bash
# Test bot token
curl "https://api.telegram.org/bot<YOUR_TOKEN>/getMe"

# Should return bot info

# Test sending message
curl -X POST "https://api.telegram.org/bot<YOUR_TOKEN>/sendMessage" \
  -d "chat_id=<YOUR_CHAT_ID>&text=Test"
```

## Next Steps

1. **Run for a week** in paper mode
2. **Review performance** daily
3. **Adjust parameters** based on results
4. **Add capital** gradually in live mode
5. **Monitor notifications** closely

## Support

- 📖 Documentation: `README.md`
- 🐛 Issues: [GitHub Issues](https://github.com/yourusername/quant-trading/issues)
- 💬 Discussions: [GitHub Discussions](https://github.com/yourusername/quant-trading/discussions)

## Safety Reminders

⚠️ **Never risk more than you can afford to lose**
⚠️ **Start with demo/paper trading**
⚠️ **Keep position sizes small**
⚠️ **Monitor daily for the first month**
⚠️ **Markets can be unpredictable**

---

Happy Trading! 📈✨
