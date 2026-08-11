"""Test harness de paridad Rust vs Python.

Genera un feed sintético determinista y ejecuta el MISMO MarketData por ambos
motores snapshot a snapshot, comparando señales nuevas, completadas, pendientes
y todos los campos numéricos.

Se salta automáticamente si `pivotradar_engine` (Rust) no está compilado.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from pivotradar_platform.calibration import Calibration  # noqa: E402
from pivotradar_platform.feeds.synthetic import SyntheticFeed  # noqa: E402
from pivotradar_platform.reference_engine import ReferenceEngine  # noqa: E402
from pivotradar_platform.rust_engine import RUST_AVAILABLE, RustEngine  # noqa: E402

TOL = 1e-9

FLOAT_FIELDS = [
    "cr", "bs", "bs_pips", "br", "range_break_pips", "nivel_estructural",
    "sweep_wick_ratio", "sweep_volume_ratio", "reclaim_body_ratio",
    "fvg_size_pips", "fvg_size_atr", "fvg_top", "fvg_bottom",
    "ob_high", "ob_low", "ob_impulse_atr",
    "mss_level", "atr14", "spread_pips", "volume_ratio",
    "calidad_sweep", "calidad_mss", "calidad_fvg", "calidad_ob", "salud_tendencial",
    "g1_compresion", "g2_persistencia", "g3_eficiencia", "g4_agotamiento",
    "conf_sweep_fvg", "conf_completa",
    "contexto_estructural", "distancia_al_sweep",
    "hipotesis_objetivo",
]

INT_FIELDS = [
    "direction", "sweep_bars_ago", "ob_bars_ago", "mss_bars_ago_h4",
    "hipotesis_expiry_velas", "hipotesis_expiry_minutos",
    "hipotesis_prob_min", "hipotesis_prob_max", "signal_age_bars",
]

STR_FIELDS = [
    "symbol", "detector", "tipo", "session", "kill_zone", "trend_d1",
    "estructura_direccion", "hipotesis_zona", "hipotesis_causa", "hipotesis_efecto",
    "hipotesis_razon", "hipotesis_invalidez", "hipotesis_texto", "mss_direction",
]

BOOL_FIELDS = [
    "equal_hl_detected", "fvg_mitigated", "ob_confluence", "mss_aligned",
    "vol_expanding", "vol_compressing", "en_zona_estructural", "gap_detected",
    "completada",
]


def _fields_equal(py_sig, rs_sig, idx: int, ctx: str):
    for f in FLOAT_FIELDS:
        pv = getattr(py_sig, f)
        rv = getattr(rs_sig, f)
        assert abs(pv - rv) < TOL, (
            f"{ctx} snapshot {idx} id={py_sig.id}: campo {f} diverge "
            f"(python={pv}, rust={rv})"
        )
    for f in INT_FIELDS:
        assert getattr(py_sig, f) == getattr(rs_sig, f), (
            f"{ctx} snapshot {idx} id={py_sig.id}: campo int {f} diverge"
        )
    for f in STR_FIELDS:
        assert getattr(py_sig, f) == getattr(rs_sig, f), (
            f"{ctx} snapshot {idx} id={py_sig.id}: campo str {f} diverge "
            f"(python={getattr(py_sig, f)!r}, rust={getattr(rs_sig, f)!r})"
        )
    for f in BOOL_FIELDS:
        assert getattr(py_sig, f) == getattr(rs_sig, f), (
            f"{ctx} snapshot {idx} id={py_sig.id}: campo bool {f} diverge"
        )
    for f in ("retorno", "mfe", "mae"):
        pv = getattr(py_sig, f)
        rv = getattr(rs_sig, f)
        for a, b in zip(pv, rv):
            assert abs(a - b) < TOL, f"{ctx} snapshot {idx} id={py_sig.id}: {f} diverge"
    assert py_sig.measured == rs_sig.measured


def _build_snapshots(n: int = 500) -> list:
    cal = Calibration(symbol="EURUSD", point=0.00001, digits=5)
    feed = SyntheticFeed(cal.symbol, cal.point, cal.digits, seed=42, bars_back=n + 100)
    snaps = [feed.snapshot()]
    for _ in range(n):
        feed.advance(900)
        snaps.append(feed.snapshot())
    return snaps


@pytest.mark.skipif(not RUST_AVAILABLE, reason="Rust no compilado")
def test_parity_500_snapshots():
    cal = Calibration(symbol="EURUSD", point=0.00001, digits=5)
    py_eng = ReferenceEngine(cal)
    rs_eng = RustEngine(cal)

    snaps = _build_snapshots(500)
    for i, mkt in enumerate(snaps):
        py_out = py_eng.process(mkt)
        rs_out = rs_eng.process(mkt)

        assert len(py_out.new_signals) == len(rs_out.new_signals), (
            f"Snapshot {i}: new count mismatch (py={len(py_out.new_signals)}, rust={len(rs_out.new_signals)})"
        )
        assert len(py_out.completed) == len(rs_out.completed), (
            f"Snapshot {i}: completed count mismatch"
        )

        py_ids = sorted(s.id for s in py_out.new_signals)
        rs_ids = sorted(s.id for s in rs_out.new_signals)
        assert py_ids == rs_ids, f"Snapshot {i}: ID mismatch en nuevas"

        py_by_id = {s.id: s for s in py_out.new_signals}
        rs_by_id = {s.id: s for s in rs_out.new_signals}
        for sid in py_ids:
            _fields_equal(py_by_id[sid], rs_by_id[sid], i, "new")

        py_c = sorted(s.id for s in py_out.completed)
        rs_c = sorted(s.id for s in rs_out.completed)
        assert py_c == rs_c, f"Snapshot {i}: IDs completadas divergen"

        py_pending = py_eng.pending_snapshot()
        rs_pending = rs_eng.pending_snapshot()
        assert len(py_pending) == len(rs_pending), f"Snapshot {i}: pending size diverge"
        assert sorted(s["id"] for s in py_pending) == sorted(s["id"] for s in rs_pending), (
            f"Snapshot {i}: pending IDs divergen"
        )


@pytest.mark.skipif(not RUST_AVAILABLE, reason="Rust no compilado")
def test_parity_detectors_each_fire():
    """Verifica que al menos cada detector dispare en la corrida (paridad funcional)."""
    cal = Calibration(symbol="EURUSD", point=0.00001, digits=5)
    py_eng = ReferenceEngine(cal)
    rs_eng = RustEngine(cal)

    py_det: set[str] = set()
    rs_det: set[str] = set()
    for mkt in _build_snapshots(2000):
        for s in py_eng.process(mkt).new_signals:
            py_det.add(s.detector)
        for s in rs_eng.process(mkt).new_signals:
            rs_det.add(s.detector)
    assert py_det == rs_det, f"Detectores divergen: py={py_det}, rs={rs_det}"
    assert py_det, "No disparó ningún detector"
