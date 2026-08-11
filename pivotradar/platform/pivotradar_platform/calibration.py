"""Carga de calibración por activo desde JSON."""

from __future__ import annotations

import json
from dataclasses import dataclass, asdict
from pathlib import Path


@dataclass
class Calibration:
    symbol: str = "EURUSD"
    utc_offset_hours: int = 0
    point: float = 1e-5
    digits: int = 5
    ntfy_topic: str = ""
    ntfy_server: str = "https://ntfy.sh"
    engine: str = "rust"  # rust | reference

    n_ruptura: int = 4
    d1_atr_threshold: float = 0.5
    body_ratio_min: float = 0.4
    d1_use_retest: bool = True
    d1_use_volume: bool = True
    d1_min_volume: float = 1.2

    sweep_n: int = 6
    sweep_wick_min: float = 0.55
    reclaim_body_min: float = 0.55
    equal_hl_window: int = 10
    equal_hl_tol: float = 0.15
    d2_anticipar: bool = True

    fvg_min_size_atr: float = 0.2
    fvg_body_ratio: float = 0.55
    fvg_mitig_umbral: float = 0.5

    ob_lookback: int = 12
    ob_body_min: float = 0.4
    ob_impulse_min: float = 0.7

    mss_lookback_h4: int = 20
    mss_max_age_h4_bars: int = 12

    pivot_depth: int = 2
    pivot_lookback: int = 24
    sweep_distancia: float = 1.5
    zona_margen: float = 0.5
    peso_estructural: float = 0.25

    prob_d1: int = 65
    prob_d2: int = 70
    prob_d3: int = 65
    prob_d4: int = 65
    prob_d5: int = 75

    def prob_base_for(self, detector: str) -> int:
        if detector == "D1":
            return self.prob_d1
        if detector in ("D2", "D2_ANTICIPACION"):
            return self.prob_d2
        if detector in ("D3", "D3_DEF"):
            return self.prob_d3
        if detector == "D4":
            return self.prob_d4
        if detector == "D5":
            return self.prob_d5
        return 55

    def to_json(self) -> str:
        d = asdict(self)
        return json.dumps(d)

    @classmethod
    def from_dict(cls, d: dict) -> "Calibration":
        known = {f.name for f in cls.__dataclass_fields__.values()}
        return cls(**{k: v for k, v in d.items() if k in known})


def load_calibrations(path: str | Path | None = None) -> dict[str, Calibration]:
    """Carga `assets` de calibrations.json -> {symbol: Calibration}."""
    if path is None:
        path = Path(__file__).resolve().parent.parent / "config" / "calibrations.json"
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    return {sym: Calibration.from_dict(cfg) for sym, cfg in data["assets"].items()}
