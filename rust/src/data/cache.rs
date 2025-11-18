//! Data caching module

use crate::{MarketData, Result};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

/// Data cache for storing market data
pub struct DataCache {
    cache_dir: PathBuf,
}

impl DataCache {
    /// Create a new data cache
    pub fn new(cache_dir: String) -> Self {
        let path = PathBuf::from(cache_dir);
        fs::create_dir_all(&path).ok();

        info!("DataCache initialized at {:?}", path);

        Self { cache_dir: path }
    }

    /// Generate cache filename
    fn get_cache_filename(
        &self,
        symbol: &str,
        start_date: &str,
        end_date: &str,
        interval: &str,
    ) -> PathBuf {
        let filename = format!("{}_{}_{}_{}.json", symbol, start_date, end_date, interval);
        self.cache_dir.join(filename)
    }

    /// Save market data to cache
    pub fn save(
        &self,
        symbol: &str,
        start_date: &str,
        end_date: &str,
        interval: &str,
        data: &MarketData,
    ) -> Result<()> {
        let cache_file = self.get_cache_filename(symbol, start_date, end_date, interval);

        let json = serde_json::to_string_pretty(data)?;
        fs::write(&cache_file, json)?;

        debug!("Data saved to cache: {:?}", cache_file);
        Ok(())
    }

    /// Load market data from cache
    pub fn load(
        &self,
        symbol: &str,
        start_date: &str,
        end_date: &str,
        interval: &str,
    ) -> Result<Option<MarketData>> {
        let cache_file = self.get_cache_filename(symbol, start_date, end_date, interval);

        if !cache_file.exists() {
            return Ok(None);
        }

        // Check if cache is stale (older than 1 hour for intraday, 1 day for daily)
        let metadata = fs::metadata(&cache_file)?;
        let modified = metadata.modified()?;
        let age = std::time::SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default();

        let max_age = if interval == "1d" {
            std::time::Duration::from_secs(24 * 3600) // 1 day
        } else {
            std::time::Duration::from_secs(3600) // 1 hour
        };

        if age > max_age {
            debug!("Cache is stale");
            return Ok(None);
        }

        // Load from cache
        let contents = fs::read_to_string(&cache_file)?;
        let data: MarketData = serde_json::from_str(&contents)?;

        debug!("Loaded data from cache: {:?}", cache_file);
        Ok(Some(data))
    }

    /// Clear cache for a specific symbol
    pub fn clear(&self, symbol: &str) -> Result<()> {
        let pattern = format!("{}_*", symbol);

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with(&pattern) {
                    fs::remove_file(path)?;
                    info!("Cleared cache file: {}", filename);
                }
            }
        }

        Ok(())
    }

    /// Clear all cache files
    pub fn clear_all(&self) -> Result<()> {
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                fs::remove_file(path)?;
            }
        }

        info!("Cleared all cache files");
        Ok(())
    }
}
