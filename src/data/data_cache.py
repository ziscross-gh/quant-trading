"""Data caching utilities."""

import pandas as pd
from pathlib import Path
import pickle
from datetime import datetime
from typing import Optional
from loguru import logger


class DataCache:
    """
    Manages caching of market data to reduce API calls and improve performance.
    """

    def __init__(self, cache_dir: str = "data/raw"):
        """
        Initialize the data cache.

        Args:
            cache_dir: Directory for cache files
        """
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        logger.info(f"DataCache initialized at {self.cache_dir}")

    def save(self, key: str, data: pd.DataFrame):
        """
        Save data to cache.

        Args:
            key: Cache key identifier
            data: DataFrame to cache
        """
        cache_file = self.cache_dir / f"{key}.pkl"
        try:
            with open(cache_file, 'wb') as f:
                pickle.dump({
                    'data': data,
                    'timestamp': datetime.now()
                }, f)
            logger.debug(f"Data cached with key: {key}")
        except Exception as e:
            logger.error(f"Failed to cache data: {e}")

    def load(self, key: str, max_age_hours: int = 1) -> Optional[pd.DataFrame]:
        """
        Load data from cache if available and not stale.

        Args:
            key: Cache key identifier
            max_age_hours: Maximum age of cache in hours

        Returns:
            Cached DataFrame or None if not available/stale
        """
        cache_file = self.cache_dir / f"{key}.pkl"

        if not cache_file.exists():
            return None

        try:
            with open(cache_file, 'rb') as f:
                cached = pickle.load(f)

            # Check if cache is stale
            age = datetime.now() - cached['timestamp']
            if age.total_seconds() > max_age_hours * 3600:
                logger.debug(f"Cache stale for key: {key}")
                return None

            logger.debug(f"Loaded cache for key: {key}")
            return cached['data']

        except Exception as e:
            logger.error(f"Failed to load cache: {e}")
            return None

    def clear(self, key: Optional[str] = None):
        """
        Clear cache.

        Args:
            key: Specific key to clear, or None to clear all
        """
        if key:
            cache_file = self.cache_dir / f"{key}.pkl"
            if cache_file.exists():
                cache_file.unlink()
                logger.info(f"Cleared cache for key: {key}")
        else:
            for cache_file in self.cache_dir.glob("*.pkl"):
                cache_file.unlink()
            logger.info("Cleared all cache files")

    def list_cache_keys(self) -> list:
        """
        List all available cache keys.

        Returns:
            List of cache keys
        """
        return [f.stem for f in self.cache_dir.glob("*.pkl")]
