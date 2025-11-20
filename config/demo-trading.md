# Demo Trading Configuration Guide

This guide helps you set up the Gold/USD autonomous trading system with your demo account and Telegram notifications.

## Quick Start

### 1. Set Up Telegram Bot

1. Open Telegram and search for `@BotFather`
2. Send `/newbot` and follow prompts to create a bot
3. Copy the bot token (looks like `123456789:ABCdefGHIjklMNOpqrsTUVwxyz`)
4. Message your new bot to start a conversation
5. Get your chat ID by messaging `@userinfobot` or visiting:
   ```
   https://api.telegram.org/bot<YOUR_TOKEN>/getUpdates
   ```

### 2. Set Environment Variables

```bash
export TELEGRAM_BOT_TOKEN="your_bot_token_here"
export TELEGRAM_CHAT_ID="your_chat_id_here"
```

### 3. Run Demo Trading

```bash
# Basic demo trading with Telegram notifications
cargo run --release --bin trade -- --mode demo

# With custom alert thresholds
cargo run --release --bin trade -- \
  --mode demo \
  --max-drawdown 15 \
  --max-losses 5 \
  --min-win-rate 40
```

## Configuration Options

### Command Line Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `--mode` | Trading mode: paper, demo, live | paper |
| `--config` | Path to config file | config/config.yaml |
| `--telegram-token` | Telegram bot token | env var |
| `--telegram-chat` | Telegram chat ID | env var |
| `--max-drawdown` | Max drawdown % before alert | 10.0 |
| `--max-losses` | Max consecutive losses before alert | 3 |
| `--min-win-rate` | Min win rate % before alert | 45.0 |
| `--once` | Run single iteration and exit | false |

### Environment Variables

```bash
# Required for Telegram notifications
TELEGRAM_BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrsTUVwxyz
TELEGRAM_CHAT_ID=123456789

# Optional - OANDA demo account
OANDA_API_TOKEN=your_practice_api_token
OANDA_ACCOUNT_ID=001-004-1234567-001

# Optional - Database for trade history
DATABASE_URL=postgresql://localhost/trading_db
```

## Telegram Notifications

You will receive notifications for:

### Trade Entry
```
📈 TRADE ENTRY

Signal: BUY
Entry: $1850.50
Size: 0.1234 oz
Stop Loss: $1840.00
Take Profit: $1870.00
```

### Trade Exit
```
💰 TRADE EXIT

Signal: BUY
Entry: $1850.50
Exit: $1865.00
P&L: +$15.00 (+0.81%)
```

### Alerts
```
⚠️ High Drawdown

Current drawdown 12.5% exceeds threshold 10.0%
```

### Session Summary
```
ℹ️ Trading System Stopped

Total Trades: 15
Win Rate: 53.3%
Total P&L: $125.50 (+2.5%)
Max Drawdown: 8.2%
```

## Recommended Demo Settings

For safe demo trading, use these conservative settings:

```bash
cargo run --release --bin trade -- \
  --mode demo \
  --max-drawdown 10 \
  --max-losses 3 \
  --min-win-rate 45
```

This will alert you when:
- Drawdown exceeds 10%
- 3 consecutive losing trades occur
- Win rate drops below 45%

## Monitoring Dashboard

While running, you'll see:
- Current Gold price
- Market regime detection
- Economic calendar events
- Position status and P&L
- Open positions with unrealized P&L

## Troubleshooting

### No Telegram notifications
1. Verify bot token is correct
2. Check chat ID (should be numeric)
3. Make sure you've messaged the bot first
4. Test manually:
   ```bash
   curl "https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=Test"
   ```

### Connection errors
1. Check internet connectivity
2. Verify OANDA credentials if using live data
3. Check firewall/proxy settings

### High CPU usage
- Normal during market hours
- Reduce check interval if needed:
  ```yaml
  # In config/config.yaml
  execution:
    check_interval: 300  # 5 minutes
  ```

## Files

- `config/config.yaml` - Main configuration
- `.env` - Environment variables (create from .env.example)
- `config/demo-trading.md` - This guide

## Support

For issues:
1. Check logs for error messages
2. Verify all environment variables
3. Test Telegram connection manually
4. Report issues: https://github.com/anthropics/claude-code/issues
