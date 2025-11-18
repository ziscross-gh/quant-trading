"""Gold/USD momentum trading strategy."""

import pandas as pd
import numpy as np
from typing import Dict, Any
from loguru import logger
from .base_strategy import BaseStrategy


class GoldMomentumStrategy(BaseStrategy):
    """
    Momentum-based trading strategy for Gold/USD using multiple technical indicators:
    - Moving Average Crossover (Fast MA vs Slow MA)
    - RSI (Relative Strength Index) for overbought/oversold conditions
    - Bollinger Bands for volatility and trend confirmation
    - Volume confirmation
    """

    def __init__(self, config: Dict[str, Any]):
        """
        Initialize the Gold momentum strategy.

        Args:
            config: Configuration dictionary
        """
        super().__init__(config)

        # Get strategy parameters from config
        self.fast_ma = self.strategy_config.get('fast_ma', 20)
        self.slow_ma = self.strategy_config.get('slow_ma', 50)
        self.rsi_period = self.strategy_config.get('rsi_period', 14)
        self.rsi_oversold = self.strategy_config.get('rsi_oversold', 30)
        self.rsi_overbought = self.strategy_config.get('rsi_overbought', 70)
        self.bb_period = self.strategy_config.get('bb_period', 20)
        self.bb_std = self.strategy_config.get('bb_std', 2)
        self.volume_ma = self.strategy_config.get('volume_ma', 20)

        logger.info(f"GoldMomentumStrategy parameters: "
                   f"MA({self.fast_ma}/{self.slow_ma}), "
                   f"RSI({self.rsi_period}), "
                   f"BB({self.bb_period},{self.bb_std})")

    def calculate_indicators(self, data: pd.DataFrame) -> pd.DataFrame:
        """
        Calculate all technical indicators.

        Args:
            data: DataFrame with OHLCV data

        Returns:
            DataFrame with added indicator columns
        """
        df = data.copy()

        # Moving Averages
        df['ma_fast'] = df['close'].rolling(window=self.fast_ma).mean()
        df['ma_slow'] = df['close'].rolling(window=self.slow_ma).mean()

        # RSI
        df['rsi'] = self._calculate_rsi(df['close'], self.rsi_period)

        # Bollinger Bands
        df['bb_middle'] = df['close'].rolling(window=self.bb_period).mean()
        bb_std = df['close'].rolling(window=self.bb_period).std()
        df['bb_upper'] = df['bb_middle'] + (bb_std * self.bb_std)
        df['bb_lower'] = df['bb_middle'] - (bb_std * self.bb_std)
        df['bb_width'] = (df['bb_upper'] - df['bb_lower']) / df['bb_middle']

        # Volume indicators
        df['volume_ma'] = df['volume'].rolling(window=self.volume_ma).mean()
        df['volume_ratio'] = df['volume'] / df['volume_ma']

        # Price momentum
        df['momentum'] = df['close'].pct_change(periods=10) * 100

        # Trend strength
        df['trend_strength'] = (df['ma_fast'] - df['ma_slow']) / df['ma_slow'] * 100

        return df

    def _calculate_rsi(self, prices: pd.Series, period: int) -> pd.Series:
        """
        Calculate Relative Strength Index.

        Args:
            prices: Price series
            period: RSI period

        Returns:
            RSI values
        """
        delta = prices.diff()
        gain = (delta.where(delta > 0, 0)).rolling(window=period).mean()
        loss = (-delta.where(delta < 0, 0)).rolling(window=period).mean()

        rs = gain / loss
        rsi = 100 - (100 / (1 + rs))
        return rsi

    def generate_signal(self, data: pd.DataFrame) -> int:
        """
        Generate trading signal based on multiple indicators.

        Args:
            data: DataFrame with OHLCV data and indicators

        Returns:
            Signal: 1 (buy), -1 (sell), 0 (hold)
        """
        if len(data) < self.slow_ma:
            logger.warning("Insufficient data for signal generation")
            return 0

        # Get latest values
        latest = data.iloc[-1]
        prev = data.iloc[-2]

        # Check for NaN values
        required_fields = ['ma_fast', 'ma_slow', 'rsi', 'bb_upper', 'bb_lower', 'volume_ratio']
        if any(pd.isna(latest[field]) for field in required_fields):
            logger.warning("NaN values in indicators, skipping signal")
            return 0

        # Initialize signal conditions
        buy_signals = 0
        sell_signals = 0

        # 1. Moving Average Crossover
        if latest['ma_fast'] > latest['ma_slow'] and prev['ma_fast'] <= prev['ma_slow']:
            buy_signals += 2  # Strong buy signal
            logger.debug("Buy signal: MA crossover (fast > slow)")
        elif latest['ma_fast'] < latest['ma_slow'] and prev['ma_fast'] >= prev['ma_slow']:
            sell_signals += 2  # Strong sell signal
            logger.debug("Sell signal: MA crossover (fast < slow)")

        # 2. RSI Conditions
        if latest['rsi'] < self.rsi_oversold:
            buy_signals += 1
            logger.debug(f"Buy signal: RSI oversold ({latest['rsi']:.2f})")
        elif latest['rsi'] > self.rsi_overbought:
            sell_signals += 1
            logger.debug(f"Sell signal: RSI overbought ({latest['rsi']:.2f})")

        # 3. Bollinger Bands
        if latest['close'] < latest['bb_lower'] and latest['close'] > prev['close']:
            buy_signals += 1
            logger.debug("Buy signal: Price bouncing from lower BB")
        elif latest['close'] > latest['bb_upper'] and latest['close'] < prev['close']:
            sell_signals += 1
            logger.debug("Sell signal: Price rejecting at upper BB")

        # 4. Volume Confirmation
        if latest['volume_ratio'] > 1.5:  # High volume
            if buy_signals > sell_signals:
                buy_signals += 1
                logger.debug("Volume confirms buy signal")
            elif sell_signals > buy_signals:
                sell_signals += 1
                logger.debug("Volume confirms sell signal")

        # 5. Trend Strength
        if abs(latest['trend_strength']) > 2:  # Strong trend
            if latest['trend_strength'] > 0:
                buy_signals += 1
            else:
                sell_signals += 1

        # Decision logic
        signal = 0
        if buy_signals >= 3 and buy_signals > sell_signals:
            signal = 1
            logger.info(f"BUY signal generated (buy_signals: {buy_signals}, sell_signals: {sell_signals})")
        elif sell_signals >= 3 and sell_signals > buy_signals:
            signal = -1
            logger.info(f"SELL signal generated (buy_signals: {buy_signals}, sell_signals: {sell_signals})")
        else:
            logger.debug(f"HOLD signal (buy_signals: {buy_signals}, sell_signals: {sell_signals})")

        return signal

    def get_indicator_summary(self, data: pd.DataFrame) -> Dict[str, Any]:
        """
        Get a summary of current indicator values.

        Args:
            data: DataFrame with indicators

        Returns:
            Dictionary with indicator values
        """
        if len(data) == 0:
            return {}

        latest = data.iloc[-1]

        return {
            'price': latest['close'],
            'ma_fast': latest['ma_fast'],
            'ma_slow': latest['ma_slow'],
            'rsi': latest['rsi'],
            'bb_upper': latest['bb_upper'],
            'bb_middle': latest['bb_middle'],
            'bb_lower': latest['bb_lower'],
            'volume_ratio': latest['volume_ratio'],
            'momentum': latest['momentum'],
            'trend_strength': latest['trend_strength']
        }


if __name__ == "__main__":
    from src.utils.config_loader import load_config
    from src.data.data_fetcher import DataFetcher

    # Test the strategy
    config = load_config()
    strategy = GoldMomentumStrategy(config)
    fetcher = DataFetcher(config)

    # Fetch data
    data = fetcher.fetch_historical_data(interval='1d')
    print(f"Data shape: {data.shape}")

    # Run strategy
    result = strategy.run(data)
    print(f"\nStrategy result: {result}")

    # Get indicator summary
    summary = strategy.get_indicator_summary(strategy.calculate_indicators(data))
    print(f"\nIndicator summary:")
    for key, value in summary.items():
        if isinstance(value, float):
            print(f"  {key}: {value:.2f}")
        else:
            print(f"  {key}: {value}")
