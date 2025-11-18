"""Setup script for the Gold/USD Trading System."""

from setuptools import setup, find_packages
from pathlib import Path

# Read the README file
readme_file = Path(__file__).parent / "README.md"
long_description = readme_file.read_text() if readme_file.exists() else ""

setup(
    name="gold-quant-trading",
    version="1.0.0",
    author="Autonomous Trading Team",
    description="Autonomous quantitative trading system for Gold/USD",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/yourusername/quant-trading",
    packages=find_packages(),
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "Intended Audience :: Financial and Insurance Industry",
        "Topic :: Office/Business :: Financial :: Investment",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
    ],
    python_requires=">=3.8",
    install_requires=[
        "pandas>=2.1.4",
        "numpy>=1.26.3",
        "scipy>=1.11.4",
        "yfinance>=0.2.35",
        "alpha-vantage>=2.3.1",
        "pandas-datareader>=0.10.0",
        "requests>=2.31.0",
        "ta-lib>=0.4.28",
        "pandas-ta>=0.3.14b0",
        "scikit-learn>=1.3.2",
        "backtrader>=1.9.78.123",
        "matplotlib>=3.8.2",
        "plotly>=5.18.0",
        "seaborn>=0.13.1",
        "sqlalchemy>=2.0.25",
        "python-dotenv>=1.0.0",
        "pyyaml>=6.0.1",
        "schedule>=1.2.0",
        "loguru>=0.7.2",
        "pytest>=7.4.3",
        "pytest-cov>=4.1.0",
    ],
    entry_points={
        "console_scripts": [
            "gold-backtest=run_backtest:main",
            "gold-trade=run_autonomous_trader:main",
        ],
    },
    include_package_data=True,
    package_data={
        "": ["config/*.yaml"],
    },
    keywords="trading, quantitative, gold, forex, algorithmic-trading, backtesting",
    project_urls={
        "Bug Reports": "https://github.com/yourusername/quant-trading/issues",
        "Source": "https://github.com/yourusername/quant-trading",
    },
)
