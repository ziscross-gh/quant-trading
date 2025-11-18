"""Base strategy class for all trading strategies."""

from abc import ABC, abstractmethod
import pandas as pd
from typing import Dict, Any, Optional
from loguru import logger


class BaseStrategy(ABC):
    """
    Abstract base class for trading strategies.
    """

    def __init__(self, config: Dict[str, Any]):
        """
        Initialize the base strategy.

        Args:
            config: Configuration dictionary
        """
        self.config = config
        self.strategy_config = config.get('strategy', {})
        self.name = self.strategy_config.get('name', self.__class__.__name__)
        self.position = 0  # Current position: 1 (long), -1 (short), 0 (neutral)
        self.last_signal = None
        logger.info(f"Strategy '{self.name}' initialized")

    @abstractmethod
    def calculate_indicators(self, data: pd.DataFrame) -> pd.DataFrame:
        """
        Calculate technical indicators for the strategy.

        Args:
            data: DataFrame with OHLCV data

        Returns:
            DataFrame with added indicator columns
        """
        pass

    @abstractmethod
    def generate_signal(self, data: pd.DataFrame) -> int:
        """
        Generate trading signal based on the data and indicators.

        Args:
            data: DataFrame with OHLCV data and indicators

        Returns:
            Signal: 1 (buy/long), -1 (sell/short), 0 (hold/no action)
        """
        pass

    def validate_signal(self, signal: int) -> bool:
        """
        Validate the generated signal.

        Args:
            signal: Trading signal

        Returns:
            True if signal is valid, False otherwise
        """
        if signal not in [-1, 0, 1]:
            logger.error(f"Invalid signal value: {signal}")
            return False
        return True

    def update_position(self, signal: int):
        """
        Update the current position based on signal.

        Args:
            signal: Trading signal
        """
        if signal != 0:
            self.position = signal
            logger.info(f"Position updated to: {self.position}")

    def get_position(self) -> int:
        """
        Get the current position.

        Returns:
            Current position
        """
        return self.position

    def reset(self):
        """Reset strategy state."""
        self.position = 0
        self.last_signal = None
        logger.info(f"Strategy '{self.name}' reset")

    def run(self, data: pd.DataFrame) -> Dict[str, Any]:
        """
        Run the complete strategy pipeline.

        Args:
            data: DataFrame with OHLCV data

        Returns:
            Dictionary with signal and metadata
        """
        # Calculate indicators
        data_with_indicators = self.calculate_indicators(data)

        # Generate signal
        signal = self.generate_signal(data_with_indicators)

        # Validate signal
        if not self.validate_signal(signal):
            signal = 0

        # Update position
        self.update_position(signal)

        result = {
            'signal': signal,
            'position': self.position,
            'timestamp': data.index[-1] if len(data) > 0 else None,
            'price': data['close'].iloc[-1] if len(data) > 0 else None,
            'strategy': self.name
        }

        self.last_signal = result
        return result

    def get_signal_description(self, signal: int) -> str:
        """
        Get human-readable description of signal.

        Args:
            signal: Trading signal

        Returns:
            Signal description
        """
        descriptions = {
            1: "BUY/LONG",
            -1: "SELL/SHORT",
            0: "HOLD/NO ACTION"
        }
        return descriptions.get(signal, "UNKNOWN")
