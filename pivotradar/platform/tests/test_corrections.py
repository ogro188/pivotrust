"""Tests de las correcciones del spec (sin Rust, sin red).

Cubre: bar_shift tolerante a gaps, measure_returns excluye vela de entrada,
spread_pips/gap_detected, notifier con separadores y engine_factory fallback.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from pivotradar_platform.calibration import Calibration  # noqa: E402
from pivotradar_platform.engine_factory import build_engine  # noqa: E402
from pivotradar_platform.engine_iface import EngineOutput, MarketData  # noqa: E402
from pivotradar_platform.feeds.synthetic import SyntheticFeed  # noqa: E402
from pivotradar_platform.notifier import format_notification  # noqa: E402
from pivotradar_platform.reference_engine import ReferenceEngine, _bar_shift_by_time  # noqa: E402
from pivotradar_platform.types import Bar, Signal  # noqa: E402


def _bar(t: int, o=1.0, h=1.0, l=1.0, c=1.0) -> Bar:
    return Bar(time=t, open=o, high=h, low=l, close=c, volume=1.0)


def test_bar_shift_exact():
    bars = [_bar(1800), _bar(900), _bar(0)]
    assert _bar_shift_by_time(bars, 900) == 1


def test_bar_shift_tolerates_gap():
    # gap: no existe time=1000; el más cercano es 1050 (dentro de ±900s).
    bars = [_bar(1800), _bar(1050), _bar(0)]
    assert _bar_shift_by_time(bars, 1000) == 1


def test_bar_shift_out_of_tolerance():
    bars = [_bar(1800), _bar(900), _bar(0)]
    assert _bar_shift_by_time(bars, 999_999) == -1


def test_measure_returns_excludes_entry_bar():
    cal = Calibration(symbol="EURUSD", point=1e-5, digits=5)
    eng = ReferenceEngine(cal)
    sig = Signal(id=1, entry_time=900, entry_bar_shift=1, direction=1, entry_price=1.0)
    eng.pending.append(sig)

    mkt = MarketData(m15=[_bar(1800, 1.0005, 1.0010, 0.9995, 1.0005),   # vela 0 (post-entry): high +10 pips
                          _bar(900, 1.0, 1.0020, 0.9990, 1.0005),      # vela 1 (entrada): high +20 pips (excluida)
                          _bar(0, 0.9995, 0.9995, 0.9985, 0.9990)])
    eng._measure_returns(mkt, EngineOutput())
    assert sig.measured[0]
    assert abs(sig.mfe[0] - 100.0) < 1e-9, f"MFE debe ser +100 pips, fue {sig.mfe[0]}"
    assert abs(sig.mae[0] + 50.0) < 1e-9, f"MAE debe ser -50 pips, fue {sig.mae[0]}"


def test_measure_returns_survives_gap():
    cal = Calibration(symbol="EURUSD", point=1e-5, digits=5)
    eng = ReferenceEngine(cal)
    sig = Signal(id=2, entry_time=900, entry_bar_shift=-1, direction=1, entry_price=1.0)
    eng.pending.append(sig)

    mkt = MarketData(m15=[_bar(1800, 1.0005, 1.0010, 0.9995, 1.0005),
                          _bar(950, 1.0, 1.0015, 0.9990, 1.0005),  # gap: 900 ausente, 950 cerca
                          _bar(0, 0.9995, 0.9995, 0.9985, 0.9990)])
    eng._measure_returns(mkt, EngineOutput())
    assert sig.entry_bar_shift == 1
    assert sig.measured[0]


def test_route_populates_spread_and_gap():
    cal = Calibration(symbol="EURUSD", point=1e-5, digits=5)
    eng = ReferenceEngine(cal)
    mkt = MarketData(m15=[_bar(0, 1.0, 1.0, 1.0, 1.0)], spread_points=40.0)
    sig = Signal(id=3, atr14=10.0)  # spread 40 > 3×10 => gap
    eng._route(mkt, sig, EngineOutput())
    assert sig.spread_pips == 40.0
    assert sig.gap_detected is True


def test_route_no_gap_when_normal_spread():
    cal = Calibration(symbol="EURUSD", point=1e-5, digits=5)
    eng = ReferenceEngine(cal)
    mkt = MarketData(m15=[_bar(0, 1.0, 1.0, 1.0, 1.0)], spread_points=5.0)
    sig = Signal(id=4, atr14=10.0)
    eng._route(mkt, sig, EngineOutput())
    assert sig.gap_detected is False


def test_format_notification_separators():
    sig = Signal(hipotesis_texto="Causa\nEfecto\nRazón")
    out = format_notification(sig)
    parts = out.split("\n")
    assert "━" in out
    assert parts[0] == "Causa"
    assert parts[-1] == "Razón"  # sin separador redundante final


def test_engine_factory_default_reference():
    cal = Calibration(engine="rust")  # rust no compilado -> fallback
    eng = build_engine(cal)
    assert isinstance(eng, ReferenceEngine)


def test_engine_factory_force_reference():
    cal = Calibration(engine="reference")
    eng = build_engine(cal)
    assert isinstance(eng, ReferenceEngine)
