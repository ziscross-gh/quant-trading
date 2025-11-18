"""Configuration loader for the trading system."""

import os
import yaml
from pathlib import Path
from typing import Dict, Any
from dotenv import load_dotenv


def load_config(config_path: str = None) -> Dict[str, Any]:
    """
    Load configuration from YAML file and environment variables.

    Args:
        config_path: Path to config YAML file. If None, uses default path.

    Returns:
        Dictionary containing configuration parameters.
    """
    # Load environment variables from .env file
    load_dotenv()

    # Determine config file path
    if config_path is None:
        project_root = Path(__file__).parent.parent.parent
        config_path = project_root / "config" / "config.yaml"

    # Load YAML configuration
    with open(config_path, 'r') as f:
        config = yaml.safe_load(f)

    # Replace environment variable placeholders
    config = _replace_env_vars(config)

    return config


def _replace_env_vars(config: Dict[str, Any]) -> Dict[str, Any]:
    """
    Recursively replace ${VAR_NAME} placeholders with environment variables.

    Args:
        config: Configuration dictionary.

    Returns:
        Configuration with environment variables substituted.
    """
    if isinstance(config, dict):
        return {k: _replace_env_vars(v) for k, v in config.items()}
    elif isinstance(config, list):
        return [_replace_env_vars(item) for item in config]
    elif isinstance(config, str) and config.startswith("${") and config.endswith("}"):
        var_name = config[2:-1]
        return os.getenv(var_name, "")
    else:
        return config


def get_project_root() -> Path:
    """Get the project root directory."""
    return Path(__file__).parent.parent.parent


def get_data_dir() -> Path:
    """Get the data directory path."""
    return get_project_root() / "data"


def get_logs_dir() -> Path:
    """Get the logs directory path."""
    return get_project_root() / "logs"


if __name__ == "__main__":
    # Test configuration loading
    config = load_config()
    print("Configuration loaded successfully:")
    print(yaml.dump(config, default_flow_style=False))
