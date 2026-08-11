"""Wrapper del motor Rust compilado (pivotradar_engine, PyO3).

Implementa la misma interfaz `PivotEngine` que el motor de referencia Python.
Si el módulo compilado no está disponible, esta clase no se usa (la plataforma
cae al motor de referencia).
"""

from __future__ import annotations

import json

from .calibration import Calibration
from .engine_iface import EngineOutput, MarketData
from .types import Signal, signal_from_dict

try:
    from pivotradar_engine import Engine as _RustEngine  # type: ignore
    from pivotradar_engine import Signal as _RustSignal  # type: ignore

    RUST_AVAILABLE = True
except Exception:  # pragma: no cover
    _RustEngine = None  # type: ignore
    RUST_AVAILABLE = False


def _to_rust_bars(bars):
    return [b.as_tuple() for b in bars]


def _to_signals(rows) -> list[Signal]:
    out = []
    for r in rows:
        if hasattr(r, "to_dict"):
            out.append(signal_from_dict(r.to_dict()))
        else:
            out.append(r)
    return out


class RustEngine:
    """Envuelve el motor Rust. Idéntico contrato al ReferenceEngine."""

    def __init__(self, cal: Calibration, pending: list[dict] | None = None):
        if not RUST_AVAILABLE:
            raise RuntimeError("pivotradar_engine (Rust) no está compilado. Instala el toolchain y ejecuta maturin develop.")
        self.cal = cal
        self._engine = _RustEngine(
            cal.to_json(),
            cal.symbol,
            float(cal.point),
            int(cal.digits),
            json.dumps(pending or []),
        )

    def process(self, mkt: MarketData) -> EngineOutput:
        new, completed = self._engine.process(
            _to_rust_bars(mkt.m15),
            _to_rust_bars(mkt.h1),
            _to_rust_bars(mkt.h4),
            _to_rust_bars(mkt.d1),
            mkt.spread_points,
            mkt.ask,
            mkt.bid,
        )
        return EngineOutput(new_signals=_to_signals(new), completed=_to_signals(completed))

    def pending_snapshot(self) -> list[dict]:
        raw = self._engine.pending_snapshot()
        if isinstance(raw, str):
            return json.loads(raw) if raw else []
        return raw
