"""Tests de la plataforma: motor de referencia + feed sintético end-to-end.

Se ejecutan con `python -m pytest tests/ -x -q` (sin Rust, sin red, sin MT5).
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from pivotradar_platform.calibration import load_calibrations  # noqa: E402
from pivotradar_platform.feeds.synthetic import SyntheticFeed  # noqa: E402
from pivotradar_platform.host import EngineHost  # noqa: E402
from pivotradar_platform.reference_engine import ReferenceEngine  # noqa: E402
from pivotradar_platform.storage import Storage  # noqa: E402


@pytest.fixture()
def cal():
    return load_calibrations()["EURUSD"]


@pytest.fixture()
def feed(cal):
    return SyntheticFeed(cal.symbol, cal.point, cal.digits, seed=42)


def test_feed_snapshot_shape(feed):
    mkt = feed.snapshot()
    assert len(mkt.m15) > 100
    assert len(mkt.h1) > 0 and len(mkt.h4) > 0 and len(mkt.d1) > 0
    # índice 0 = vela en formación, serie más reciente primero
    assert mkt.m15[0].time > mkt.m15[1].time
    assert mkt.ask > mkt.bid


def test_reference_engine_process(cal, feed):
    eng = ReferenceEngine(cal)
    mkt = feed.snapshot()
    out = eng.process(mkt)
    assert out.new_signals is not None
    assert out.completed is not None
    # snapshot de pendientes es lista de dicts
    snap = eng.pending_snapshot()
    assert isinstance(snap, list)
    if snap:
        assert "detector" in snap[0]


def test_host_persists(cal, feed, tmp_path):
    host = EngineHost(cal, feed, data_dir=str(tmp_path), poll_seconds=0)
    for _ in range(50):
        host.step_once()
        feed.advance(900)
    db = tmp_path / "EURUSD.db"
    csv = tmp_path / "EURUSD.csv"
    assert db.exists() and csv.exists()
    st = Storage("EURUSD", str(tmp_path))
    assert st.load_pending() is not None


def test_multi_asset(tmp_path):
    from pivotradar_platform.manager import Manager

    cals = load_calibrations()
    m = Manager({s: c for s, c in cals.items() if s in ("EURUSD", "BTCUSD")},
                data_dir=str(tmp_path), source="synthetic", poll_seconds=0.01)
    assert len(m.hosts) == 2
    m.stop()
