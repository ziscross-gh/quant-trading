"""Data fetching module for Gold/USD prices."""

import yfinance as yf
import pandas as pd
from datetime import datetime, timedelta
from typing import Optional, Tuple
from pathlib import Path
import pickle
from loguru import logger


class DataFetcher:
    """
    Fetches and manages Gold/USD market data from various sources.
    """

    def __init__(self, config: dict):
        """
        Initialize the data fetcher.

        Args:
            config: Configuration dictionary containing data settings
        """
        self.config = config
        self.data_config = config.get('data', {})
        self.symbol = config['trading']['symbol']
        self.provider = self.data_config.get('provider', 'yfinance')
        self.cache_enabled = self.data_config.get('cache_enabled', True)
        self.cache_dir = Path(self.data_config.get('cache_dir', 'data/raw'))
        self.cache_dir.mkdir(parents=True, exist_ok=True)

        logger.info(f"DataFetcher initialized for {self.symbol} using {self.provider}")

    def fetch_historical_data(
        self,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        interval: str = '1h'
    ) -> pd.DataFrame:
        """
        Fetch historical Gold/USD price data.

        Args:
            start_date: Start date in 'YYYY-MM-DD' format
            end_date: End date in 'YYYY-MM-DD' format
            interval: Data interval (1m, 5m, 15m, 30m, 1h, 1d)

        Returns:
            DataFrame with OHLCV data
        """
        if start_date is None:
            lookback_days = self.data_config.get('lookback_days', 365)
            start_date = (datetime.now() - timedelta(days=lookback_days)).strftime('%Y-%m-%d')

        if end_date is None:
            end_date = datetime.now().strftime('%Y-%m-%d')

        logger.info(f"Fetching historical data from {start_date} to {end_date}, interval: {interval}")

        # Check cache first
        if self.cache_enabled:
            cached_data = self._load_from_cache(start_date, end_date, interval)
            if cached_data is not None:
                logger.info("Loaded data from cache")
                return cached_data

        # Fetch from provider
        if self.provider == 'yfinance':
            data = self._fetch_yfinance(start_date, end_date, interval)
        else:
            raise ValueError(f"Unsupported data provider: {self.provider}")

        # Process and clean data
        data = self._process_data(data)

        # Cache the data
        if self.cache_enabled:
            self._save_to_cache(data, start_date, end_date, interval)

        logger.info(f"Fetched {len(data)} data points")
        return data

    def fetch_live_data(self, period: str = '1d', interval: str = '1m') -> pd.DataFrame:
        """
        Fetch live/recent Gold/USD price data.

        Args:
            period: Time period (1d, 5d, 1mo, etc.)
            interval: Data interval (1m, 5m, 15m, 30m, 1h)

        Returns:
            DataFrame with recent OHLCV data
        """
        logger.debug(f"Fetching live data, period: {period}, interval: {interval}")

        if self.provider == 'yfinance':
            ticker = yf.Ticker(self.symbol)
            data = ticker.history(period=period, interval=interval)
        else:
            raise ValueError(f"Unsupported data provider: {self.provider}")

        data = self._process_data(data)
        return data

    def get_current_price(self) -> Tuple[float, datetime]:
        """
        Get the current Gold/USD price.

        Returns:
            Tuple of (price, timestamp)
        """
        ticker = yf.Ticker(self.symbol)
        data = ticker.history(period='1d', interval='1m')

        if len(data) == 0:
            raise ValueError("No current price data available")

        latest = data.iloc[-1]
        price = latest['Close']
        timestamp = data.index[-1]

        logger.debug(f"Current price: ${price:.2f} at {timestamp}")
        return float(price), timestamp

    def _fetch_yfinance(
        self,
        start_date: str,
        end_date: str,
        interval: str
    ) -> pd.DataFrame:
        """
        Fetch data using yfinance.

        Args:
            start_date: Start date
            end_date: End date
            interval: Data interval

        Returns:
            Raw DataFrame from yfinance
        """
        ticker = yf.Ticker(self.symbol)
        data = ticker.history(start=start_date, end=end_date, interval=interval)

        if len(data) == 0:
            logger.warning(f"No data returned for {self.symbol}")

        return data

    def _process_data(self, data: pd.DataFrame) -> pd.DataFrame:
        """
        Process and clean the raw data.

        Args:
            data: Raw DataFrame

        Returns:
            Processed DataFrame
        """
        if len(data) == 0:
            return data

        # Rename columns to standard format
        column_mapping = {
            'Open': 'open',
            'High': 'high',
            'Low': 'low',
            'Close': 'close',
            'Volume': 'volume'
        }
        data = data.rename(columns=column_mapping)

        # Select only the columns we need
        required_columns = ['open', 'high', 'low', 'close', 'volume']
        available_columns = [col for col in required_columns if col in data.columns]
        data = data[available_columns]

        # Remove any NaN values
        data = data.dropna()

        # Ensure index is datetime
        if not isinstance(data.index, pd.DatetimeIndex):
            data.index = pd.to_datetime(data.index)

        # Sort by date
        data = data.sort_index()

        return data

    def _get_cache_filename(self, start_date: str, end_date: str, interval: str) -> Path:
        """Generate cache filename."""
        filename = f"{self.symbol}_{start_date}_{end_date}_{interval}.pkl"
        return self.cache_dir / filename

    def _save_to_cache(self, data: pd.DataFrame, start_date: str, end_date: str, interval: str):
        """Save data to cache."""
        cache_file = self._get_cache_filename(start_date, end_date, interval)
        with open(cache_file, 'wb') as f:
            pickle.dump(data, f)
        logger.debug(f"Data saved to cache: {cache_file}")

    def _load_from_cache(
        self,
        start_date: str,
        end_date: str,
        interval: str
    ) -> Optional[pd.DataFrame]:
        """Load data from cache if available and not stale."""
        cache_file = self._get_cache_filename(start_date, end_date, interval)

        if not cache_file.exists():
            return None

        # Check if cache is recent (less than 1 hour old for intraday, 1 day for daily)
        cache_age = datetime.now() - datetime.fromtimestamp(cache_file.stat().st_mtime)
        max_age = timedelta(hours=1) if interval in ['1m', '5m', '15m', '30m', '1h'] else timedelta(days=1)

        if cache_age > max_age:
            logger.debug("Cache is stale")
            return None

        try:
            with open(cache_file, 'rb') as f:
                data = pickle.load(f)
            return data
        except Exception as e:
            logger.warning(f"Failed to load cache: {e}")
            return None


if __name__ == "__main__":
    from src.utils.config_loader import load_config

    # Test data fetcher
    config = load_config()
    fetcher = DataFetcher(config)

    # Fetch historical data
    df = fetcher.fetch_historical_data(interval='1d')
    print(f"\nHistorical data shape: {df.shape}")
    print(df.head())
    print(df.tail())

    # Get current price
    price, timestamp = fetcher.get_current_price()
    print(f"\nCurrent Gold price: ${price:.2f} at {timestamp}")
