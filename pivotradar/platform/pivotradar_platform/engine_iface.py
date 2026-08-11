"""Contratos del motor: la plataforma habla con CUALQUIER motor que cumpla esta interfaz."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Protocol, runtime_checkable

from .types import Bar, Signal


@dataclass
class MarketData:
    """Snapshot que la plataforma entrega al motor. Índice 0 = vela en formación."""

    m15: list[Bar] = field(default_factory=list)
    h1: list[Bar] = field(default_factory=list)
    h4: list[Bar] = field(default_factory=list)
    d1: list[Bar] = field(default_factory=list)
    spread_points: float | None = None
    ask: float | None = None
    bid: float | None = None


@dataclass
class EngineOutput:
    """Salida de `process`: señales nuevas y señales que completaron medición."""

    new_signals: list[Signal] = field(default_factory=list)
    completed: list[Signal] = field(default_factory=list)


@runtime_checkable
class PivotEngine(Protocol):
    """Cualquier motor (Rust o Python) debe implementar esta interfaz."""

    def process(self, mkt: MarketData) -> EngineOutput: ...

    def pending_snapshot(self) -> list[dict]: ...
