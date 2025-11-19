#!/bin/bash
# Quick setup script for Gold/USD Trading Bot

set -e

echo "🚀 Gold/USD Autonomous Trading Bot - Setup Script"
echo "=================================================="
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed"
    echo "Install from: https://rustup.rs/"
    exit 1
fi

echo "✅ Rust is installed"

# Check if PostgreSQL is installed
if command -v psql &> /dev/null; then
    echo "✅ PostgreSQL is installed"
else
    echo "⚠️  PostgreSQL not found (optional, but recommended)"
    echo "Install: sudo apt install postgresql (Ubuntu) or brew install postgresql (macOS)"
fi

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    echo ""
    echo "📝 Creating .env file from example..."
    cp .env.example .env
    echo "✅ Created .env file"
    echo "⚠️  Please edit .env and add your API keys!"
    echo ""
else
    echo "✅ .env file already exists"
fi

# Build the project
echo ""
echo "🔨 Building project (this may take a few minutes)..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
else
    echo "❌ Build failed"
    exit 1
fi

# Check if database URL is set
if grep -q "DATABASE_URL=postgresql://localhost/trading_db" .env 2>/dev/null; then
    echo ""
    echo "💾 Setting up database..."

    # Check if sqlx-cli is installed
    if ! command -v sqlx &> /dev/null; then
        echo "Installing sqlx-cli..."
        cargo install sqlx-cli --no-default-features --features postgres
    fi

    # Try to create database
    if command -v createdb &> /dev/null; then
        createdb trading_db 2>/dev/null || echo "Database might already exist"
    fi

    # Run migrations
    if [ -d migrations ]; then
        echo "Running database migrations..."
        sqlx migrate run || echo "⚠️  Migration failed - you may need to set up PostgreSQL"
    fi
fi

echo ""
echo "=================================================="
echo "✅ Setup Complete!"
echo "=================================================="
echo ""
echo "Next steps:"
echo "1. Edit .env file with your API keys"
echo "   nano .env"
echo ""
echo "2. Get OANDA demo account (free):"
echo "   https://www.oanda.com/demo-account/"
echo ""
echo "3. Run backtest:"
echo "   cargo run --bin backtest --release"
echo ""
echo "4. Start paper trading:"
echo "   cargo run --bin trade --release"
echo ""
echo "📖 Full setup guide: ../SETUP.md"
echo ""
