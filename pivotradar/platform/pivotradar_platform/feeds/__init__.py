"""Feeds de datos para la plataforma.

Cada feed entrega un `MarketData` (índice 0 = vela en formación, M15/H1/H4/D1,
spread/ask/bid). La plataforma es agnóstica del origen: sintético (base de
prueba de estrategias) o MetaTrader 5 (producción).
"""

from .base import Feed
from .synthetic import SyntheticFeed
from .mt5 import MT5Feed

__all__ = ["Feed", "SyntheticFeed", "MT5Feed"]
