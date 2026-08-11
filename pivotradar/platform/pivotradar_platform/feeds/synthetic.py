"""Feed sintético: genera OHLCV M15/H1/H4/D1 con un paseo aleatorio dirigido.

Base de prueba de estrategias: no requiere MetaTrader ni conexión. Determinista
con `seed` para reproducible backtest de la plataforma.

Generación:
  - Se construye una serie de velas M15 hacia atrás desde `start_time`
    (paseo con tendencia + noise), y de ahí se agregan H1/H4/D1.
  - El índice 0 de cada serie es la vela en formación (consistente con MT5).
  - `advance(delta)` mueve el reloj: la vela en formación se actualiza y,
    al cruzar un límite de 15 min, rota a las series superiores.
"""

from __future__ import annotations

import math
import random
import time

from ..engine_iface import MarketData
from ..types import Bar

TF_MINUTES = {"M15": 15, "H1": 60, "H4": 240, "D1": 1440}


def _align_floor(t: int, minutes: int) -> int:
    return t - (t % (minutes * 60))


def _aggregate_from_m15(bars_m15: list[Bar], minutes: int) -> list[Bar]:
    """Agrupa velas M15 cerradas (reverse-chrono, index 0 = más reciente) en `minutes`.

    Devuelve velas cerradas de esa TF en reverse-chrono (index 0 = más reciente).
    """
    if not bars_m15:
        return []
    closed = [b for b in bars_m15]
    buckets: dict[int, dict] = {}
    order: list[int] = []
    for b in closed:
        key = _align_floor(b.time, minutes)
        if key not in buckets:
            buckets[key] = {"t": key, "o": b.open, "h": b.high, "l": b.low, "c": b.close, "v": b.volume}
            order.append(key)
        else:
            e = buckets[key]
            e["h"] = max(e["h"], b.high)
            e["l"] = min(e["l"], b.low)
            e["c"] = b.close
            e["v"] += b.volume
    out = [Bar(time=e["t"], open=e["o"], high=e["h"], low=e["l"], close=e["c"], volume=e["v"])
           for e in (buckets[k] for k in reversed(order))]
    return out


class SyntheticFeed:
    def __init__(self, symbol: str, point: float, digits: int, seed: int | None = None,
                 start_time: int | None = None, bars_back: int = 4000,
                 vol_atr_points: float = 8.0, trend: float = 0.002):
        self.symbol = symbol
        self.point = point
        self.digits = digits
        self.rng = random.Random(seed)
        self.trend = trend
        self.vol_points = max(vol_atr_points, 0.1)
        now = start_time or int(time.time())
        self.now = _align_floor(now, 15)
        self.bars_m15: list[Bar] = []  # reverse-chrono, index 0 = forming
        self._generate_history(bars_back)

    # ------------------------------------------------------------------ generación
    def _bar_ohlc(self, prev_close: float, minutes_frac: float) -> tuple:
        p = self.point
        scale = max(abs(prev_close), 0.0001)
        step = self.trend * scale * minutes_frac * self.rng.uniform(-1.0, 1.0)
        rng_v = scale * self.vol_points * p * math.sqrt(minutes_frac)
        wick = rng_v * self.rng.uniform(0.1, 0.8)
        open_p = prev_close
        close_p = prev_close + step + self.rng.gauss(0.0, rng_v)
        high_p = max(open_p, close_p) + wick
        low_p = min(open_p, close_p) - wick
        return open_p, high_p, low_p, close_p

    def _generate_history(self, bars_back: int):
        t = self.now - (bars_back - 1) * 900
        price = self.rng.uniform(0.9, 1.2) if self.symbol not in ("BTCUSD", "XAUUSD") else self.rng.uniform(100, 3000)
        price = round(price, self.digits)
        for i in range(bars_back):
            o, h, l, c = self._bar_ohlc(price, 1.0)
            vol = self.rng.uniform(0.5, 2.0)
            self.bars_m15.append(Bar(time=t + i * 900, open=round(o, self.digits), high=round(h, self.digits),
                                     low=round(l, self.digits), close=round(c, self.digits), volume=vol))
            price = c
        self.bars_m15 = list(reversed(self.bars_m15))  # index 0 = más reciente

    # ------------------------------------------------------------------ series
    def _m15_closed(self) -> list[Bar]:
        return self.bars_m15[1:]  # todo menos la forming

    def snapshot(self) -> MarketData:
        m15 = list(self.bars_m15)
        closed = self._m15_closed()
        h1 = _aggregate_from_m15(closed, 60)
        h4 = _aggregate_from_m15(closed, 240)
        d1 = _aggregate_from_m15(closed, 1440)
        forming = m15[0]
        spread = self.point * self.rng.uniform(1.0, 2.5)
        return MarketData(m15=m15, h1=h1, h4=h4, d1=d1,
                          spread_points=spread, ask=forming.close + spread / 2, bid=forming.close - spread / 2)

    def advance(self, delta_sec: int = 15):
        """Mueve el reloj `delta_sec` segundos; rota velas al cruzar límites de 15 min."""
        self.now += delta_sec
        cur = self.bars_m15[0]
        prev_close = cur.close
        if cur.time + 900 <= self.now:
            self.bars_m15.pop(0)
            self.bars_m15.insert(0, Bar(time=_align_floor(self.now, 15), open=prev_close, high=prev_close,
                                        low=prev_close, close=prev_close, volume=0.0))
            cur = self.bars_m15[0]
        o, h, l, c = self._bar_ohlc(cur.open, delta_sec / 900.0)
        self.bars_m15[0] = Bar(time=cur.time, open=cur.open, high=max(cur.high, h), low=min(cur.low, l),
                               close=c, volume=cur.volume + self.rng.uniform(0, 0.1))

    def history(self, step_bars: int = 4, window_bars: int = 1000) -> list[MarketData]:
        """Snapshots para backtest (cronológico ascendente), uno cada `step_bars`.

        Cada snapshot usa una ventana de a lo sumo `window_bars` velas M15 (las
        más recientes hasta ese instante), suficientes para los indicadores del
        motor; acotar la ventana evita agregaciones O(n²) en historiales largos.
        """
        closed = [b for b in self.bars_m15 if b.time < self.now]
        closed = list(reversed(closed))  # oldest -> newest
        out = []
        min_bars = min(300, len(closed))
        for i in range(min_bars, len(closed) + 1, step_bars):
            window = closed[:i][-window_bars:]
            rev = list(reversed(window))
            last = window[-1]
            out.append(MarketData(
                m15=rev,
                h1=_aggregate_from_m15(rev, 60),
                h4=_aggregate_from_m15(rev, 240),
                d1=_aggregate_from_m15(rev, 1440),
                spread_points=self.point,
                ask=last.close + self.point,
                bid=last.close - self.point,
            ))
        return out
