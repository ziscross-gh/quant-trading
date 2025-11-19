-- Create trades table
CREATE TABLE trades (
    id SERIAL PRIMARY KEY,
    entry_time TIMESTAMPTZ NOT NULL,
    exit_time TIMESTAMPTZ NOT NULL,
    signal VARCHAR(10) NOT NULL,
    entry_price DECIMAL(10, 2) NOT NULL,
    exit_price DECIMAL(10, 2) NOT NULL,
    size DECIMAL(10, 4) NOT NULL,
    pnl DECIMAL(10, 2) NOT NULL,
    pnl_pct DECIMAL(6, 2) NOT NULL,
    duration_hours DECIMAL(10, 2) NOT NULL,
    strategy VARCHAR(50),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX idx_trades_entry_time ON trades(entry_time);
CREATE INDEX idx_trades_strategy ON trades(strategy);
CREATE INDEX idx_trades_pnl ON trades(pnl);
