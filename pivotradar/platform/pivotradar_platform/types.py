"""Tipos compartidos de la plataforma (independientes del motor)."""

from __future__ import annotations

from dataclasses import dataclass, field, asdict


@dataclass
class Bar:
    """Vela OHLCV. `time` en segundos Unix; el índice 0 de una serie es la vela en formación."""

    time: int
    open: float
    high: float
    low: float
    close: float
    volume: float

    def as_tuple(self) -> tuple:
        return (self.time, self.open, self.high, self.low, self.close, self.volume)


def bar_from_tuple(t: tuple) -> Bar:
    return Bar(time=int(t[0]), open=t[1], high=t[2], low=t[3], close=t[4], volume=t[5])


def bars_from_tuples(rows) -> list[Bar]:
    return [bar_from_tuple(r) for r in rows]


@dataclass
class Signal:
    """Señal producida por el motor (misma forma para Rust y referencia Python)."""

    id: int = 0
    entry_time: int = 0
    entry_bar_shift: int = -1
    symbol: str = ""
    direction: int = 0
    entry_price: float = 0.0
    detector: str = ""
    tipo: str = ""

    cr: float = 0.0
    bs: float = 0.0
    bs_pips: float = 0.0
    br: float = 0.0
    range_break_pips: float = 0.0
    nivel_estructural: float = 0.0

    sweep_wick_ratio: float = 0.0
    sweep_volume_ratio: float = 0.0
    reclaim_body_ratio: float = 0.0
    sweep_bars_ago: int = 0
    equal_hl_detected: bool = False
    level_swept: float = 0.0

    fvg_size_pips: float = 0.0
    fvg_size_atr: float = 0.0
    fvg_mitigated: bool = False
    fvg_top: float = 0.0
    fvg_bottom: float = 0.0

    ob_high: float = 0.0
    ob_low: float = 0.0
    ob_bars_ago: int = 0
    ob_impulse_atr: float = 0.0
    ob_confluence: bool = False

    mss_aligned: bool = False
    mss_bars_ago_h4: int = 0
    mss_direction: str = ""
    mss_level: float = 0.0

    atr14: float = 0.0
    spread_pips: float = 0.0
    volume_ratio: float = 0.0
    session: str = ""
    kill_zone: str = ""
    trend_d1: str = "NEUTRO"
    vol_expanding: bool = False
    vol_compressing: bool = False

    calidad_sweep: float = 0.0
    calidad_mss: float = 0.0
    calidad_fvg: float = 0.0
    calidad_ob: float = 0.0
    salud_tendencial: float = 0.0

    g1_compresion: float = 0.0
    g2_persistencia: float = 0.0
    g3_eficiencia: float = 0.0
    g4_agotamiento: float = 0.0

    conf_sweep_fvg: float = 0.0
    conf_completa: float = 0.0

    contexto_estructural: float = 0.0
    estructura_direccion: str = "NEUTRO"
    distancia_al_sweep: float = 0.0
    en_zona_estructural: bool = False

    hipotesis_causa: str = ""
    hipotesis_efecto: str = ""
    hipotesis_razon: str = ""
    hipotesis_invalidez: str = ""
    hipotesis_expiry_velas: int = 0
    hipotesis_expiry_minutos: int = 0
    hipotesis_prob_min: int = 0
    hipotesis_prob_max: int = 0
    hipotesis_zona: str = "NEUTRO"
    hipotesis_objetivo: float = 0.0
    hipotesis_texto: str = ""

    signal_age_bars: int = 0
    measured: list = field(default_factory=lambda: [False] * 4)
    retorno: list = field(default_factory=lambda: [0.0] * 4)
    mfe: list = field(default_factory=lambda: [0.0] * 4)
    mae: list = field(default_factory=lambda: [0.0] * 4)
    gap_detected: bool = False
    completada: bool = False

    def as_dict(self) -> dict:
        return asdict(self)


def signal_from_dict(d: dict) -> Signal:
    known = {f.name for f in Signal.__dataclass_fields__.values()}
    return Signal(**{k: v for k, v in d.items() if k in known})
