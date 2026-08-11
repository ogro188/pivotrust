"""Contrato de feed de datos y helpers compartidos.

Un feed debe ser capaz de producir un `MarketData` (snapshot actual) y, para
backtest/escenarios, un historial completo con el cual reproducir la operación.
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

from ..engine_iface import MarketData


@runtime_checkable
class Feed(Protocol):
    symbol: str

    def snapshot(self) -> MarketData: ...

    def history(self) -> list[MarketData]: ...
