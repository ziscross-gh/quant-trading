"""Performance metrics calculation for backtesting."""

import pandas as pd
import numpy as np
from typing import Dict, Any
from loguru import logger


class PerformanceMetrics:
    """
    Calculate various performance metrics for trading strategies.
    """

    def __init__(
        self,
        trades_df: pd.DataFrame,
        equity_df: pd.DataFrame,
        initial_capital: float
    ):
        """
        Initialize performance metrics calculator.

        Args:
            trades_df: DataFrame with trade details
            equity_df: DataFrame with equity curve
            initial_capital: Initial capital amount
        """
        self.trades_df = trades_df
        self.equity_df = equity_df
        self.initial_capital = initial_capital

    def calculate_all_metrics(self) -> Dict[str, Any]:
        """
        Calculate all performance metrics.

        Returns:
            Dictionary with all metrics
        """
        metrics = {}

        # Basic metrics
        metrics.update(self._calculate_basic_metrics())

        # Return metrics
        metrics.update(self._calculate_return_metrics())

        # Risk metrics
        metrics.update(self._calculate_risk_metrics())

        # Win/Loss metrics
        metrics.update(self._calculate_win_loss_metrics())

        # Additional metrics
        metrics.update(self._calculate_additional_metrics())

        return metrics

    def _calculate_basic_metrics(self) -> Dict[str, Any]:
        """Calculate basic trading metrics."""
        total_trades = len(self.trades_df)
        final_capital = self.equity_df['equity'].iloc[-1] if len(self.equity_df) > 0 else self.initial_capital

        return {
            'total_trades': total_trades,
            'initial_capital': self.initial_capital,
            'final_capital': final_capital,
            'net_profit': final_capital - self.initial_capital,
            'total_return_pct': ((final_capital - self.initial_capital) / self.initial_capital) * 100
        }

    def _calculate_return_metrics(self) -> Dict[str, Any]:
        """Calculate return-based metrics."""
        if len(self.equity_df) == 0:
            return {}

        equity = self.equity_df['equity']
        returns = equity.pct_change().dropna()

        # Annualized return
        total_days = (self.equity_df['timestamp'].iloc[-1] - self.equity_df['timestamp'].iloc[0]).days
        years = total_days / 365.25
        final_capital = equity.iloc[-1]

        if years > 0:
            annualized_return = (((final_capital / self.initial_capital) ** (1 / years)) - 1) * 100
        else:
            annualized_return = 0

        return {
            'annualized_return_pct': annualized_return,
            'avg_daily_return_pct': returns.mean() * 100 if len(returns) > 0 else 0,
            'std_daily_return_pct': returns.std() * 100 if len(returns) > 0 else 0
        }

    def _calculate_risk_metrics(self) -> Dict[str, Any]:
        """Calculate risk metrics."""
        if len(self.equity_df) == 0:
            return {}

        equity = self.equity_df['equity']
        returns = equity.pct_change().dropna()

        # Maximum drawdown
        peak = equity.expanding(min_periods=1).max()
        drawdown = (equity - peak) / peak
        max_drawdown = drawdown.min() * 100

        # Sharpe ratio (assuming risk-free rate of 2%)
        risk_free_rate = 0.02 / 252  # Daily risk-free rate
        if returns.std() > 0:
            sharpe_ratio = (returns.mean() - risk_free_rate) / returns.std() * np.sqrt(252)
        else:
            sharpe_ratio = 0

        # Sortino ratio (downside deviation)
        downside_returns = returns[returns < 0]
        if len(downside_returns) > 0 and downside_returns.std() > 0:
            sortino_ratio = (returns.mean() - risk_free_rate) / downside_returns.std() * np.sqrt(252)
        else:
            sortino_ratio = 0

        # Calmar ratio (return / max drawdown)
        total_days = (self.equity_df['timestamp'].iloc[-1] - self.equity_df['timestamp'].iloc[0]).days
        years = total_days / 365.25

        if years > 0:
            annualized_return = ((equity.iloc[-1] / self.initial_capital) ** (1 / years)) - 1
            calmar_ratio = annualized_return / abs(max_drawdown / 100) if max_drawdown != 0 else 0
        else:
            calmar_ratio = 0

        return {
            'max_drawdown_pct': max_drawdown,
            'sharpe_ratio': sharpe_ratio,
            'sortino_ratio': sortino_ratio,
            'calmar_ratio': calmar_ratio,
            'volatility_pct': returns.std() * np.sqrt(252) * 100 if len(returns) > 0 else 0
        }

    def _calculate_win_loss_metrics(self) -> Dict[str, Any]:
        """Calculate win/loss metrics."""
        if len(self.trades_df) == 0:
            return {}

        winning_trades = self.trades_df[self.trades_df['pnl'] > 0]
        losing_trades = self.trades_df[self.trades_df['pnl'] < 0]

        total_trades = len(self.trades_df)
        num_wins = len(winning_trades)
        num_losses = len(losing_trades)

        win_rate = (num_wins / total_trades * 100) if total_trades > 0 else 0

        avg_win = winning_trades['pnl'].mean() if num_wins > 0 else 0
        avg_loss = losing_trades['pnl'].mean() if num_losses > 0 else 0

        largest_win = winning_trades['pnl'].max() if num_wins > 0 else 0
        largest_loss = losing_trades['pnl'].min() if num_losses > 0 else 0

        profit_factor = abs(winning_trades['pnl'].sum() / losing_trades['pnl'].sum()) if num_losses > 0 and losing_trades['pnl'].sum() != 0 else 0

        return {
            'win_rate_pct': win_rate,
            'num_winning_trades': num_wins,
            'num_losing_trades': num_losses,
            'avg_win': avg_win,
            'avg_loss': avg_loss,
            'largest_win': largest_win,
            'largest_loss': largest_loss,
            'profit_factor': profit_factor
        }

    def _calculate_additional_metrics(self) -> Dict[str, Any]:
        """Calculate additional metrics."""
        if len(self.trades_df) == 0:
            return {}

        # Average trade duration
        self.trades_df['duration_hours'] = self.trades_df['duration'].dt.total_seconds() / 3600
        avg_duration = self.trades_df['duration_hours'].mean()

        # Calculate consecutive wins/losses
        pnl_signs = (self.trades_df['pnl'] > 0).astype(int)
        pnl_signs = pnl_signs * 2 - 1  # Convert to 1/-1

        max_consecutive_wins = 0
        max_consecutive_losses = 0
        current_streak = 0

        for sign in pnl_signs:
            if sign == 1:
                if current_streak > 0:
                    current_streak += 1
                else:
                    current_streak = 1
                max_consecutive_wins = max(max_consecutive_wins, current_streak)
            else:
                if current_streak < 0:
                    current_streak -= 1
                else:
                    current_streak = -1
                max_consecutive_losses = max(max_consecutive_losses, abs(current_streak))

        return {
            'avg_trade_duration_hours': avg_duration,
            'max_consecutive_wins': max_consecutive_wins,
            'max_consecutive_losses': max_consecutive_losses
        }

    def print_summary(self):
        """Print formatted summary of all metrics."""
        metrics = self.calculate_all_metrics()

        print("\n" + "="*60)
        print("BACKTEST PERFORMANCE SUMMARY")
        print("="*60)

        print("\n--- Basic Metrics ---")
        print(f"Total Trades: {metrics.get('total_trades', 0)}")
        print(f"Initial Capital: ${metrics.get('initial_capital', 0):,.2f}")
        print(f"Final Capital: ${metrics.get('final_capital', 0):,.2f}")
        print(f"Net Profit: ${metrics.get('net_profit', 0):,.2f}")
        print(f"Total Return: {metrics.get('total_return_pct', 0):.2f}%")

        print("\n--- Return Metrics ---")
        print(f"Annualized Return: {metrics.get('annualized_return_pct', 0):.2f}%")
        print(f"Avg Daily Return: {metrics.get('avg_daily_return_pct', 0):.3f}%")

        print("\n--- Risk Metrics ---")
        print(f"Max Drawdown: {metrics.get('max_drawdown_pct', 0):.2f}%")
        print(f"Sharpe Ratio: {metrics.get('sharpe_ratio', 0):.2f}")
        print(f"Sortino Ratio: {metrics.get('sortino_ratio', 0):.2f}")
        print(f"Calmar Ratio: {metrics.get('calmar_ratio', 0):.2f}")
        print(f"Volatility: {metrics.get('volatility_pct', 0):.2f}%")

        print("\n--- Win/Loss Metrics ---")
        print(f"Win Rate: {metrics.get('win_rate_pct', 0):.2f}%")
        print(f"Winning Trades: {metrics.get('num_winning_trades', 0)}")
        print(f"Losing Trades: {metrics.get('num_losing_trades', 0)}")
        print(f"Avg Win: ${metrics.get('avg_win', 0):,.2f}")
        print(f"Avg Loss: ${metrics.get('avg_loss', 0):,.2f}")
        print(f"Largest Win: ${metrics.get('largest_win', 0):,.2f}")
        print(f"Largest Loss: ${metrics.get('largest_loss', 0):,.2f}")
        print(f"Profit Factor: {metrics.get('profit_factor', 0):.2f}")

        print("\n--- Additional Metrics ---")
        print(f"Avg Trade Duration: {metrics.get('avg_trade_duration_hours', 0):.1f} hours")
        print(f"Max Consecutive Wins: {metrics.get('max_consecutive_wins', 0)}")
        print(f"Max Consecutive Losses: {metrics.get('max_consecutive_losses', 0)}")

        print("\n" + "="*60)
