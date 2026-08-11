"""Motor de referencia PivotRadar en Python puro.

Port 1:1 del motor Rust (`pivotradar-core`) y del EA MQL5 v7.6.
No requiere Rust: permite probar la plataforma y validar el port.
La interfaz es la misma que la del motor Rust (`PivotEngine`).
"""

from __future__ import annotations

import time as _time

from .calibration import Calibration
from .engine_iface import EngineOutput, MarketData
from .types import Signal

_ATR_BUFFER_SIZE = 55
_MAX_PENDING = 500


def _clamp(v: float) -> float:
    if v < 0:
        return 0.0
    if v > 100:
        return 100.0
    return v


def _round_digits(x: float, digits: int) -> float:
    f = 10.0 ** digits
    return float(round(x * f)) / f


def _ema_series(values, period: int):
    n = len(values)
    out = [0.0] * n
    if n == 0 or period == 0:
        return out
    alpha = 2.0 / (period + 1.0)
    if n < period:
        acc = 0.0
        cnt = 0.0
        for i in range(n - 1, -1, -1):
            acc += values[i]
            cnt += 1.0
            out[i] = acc / cnt
        return out
    seed = sum(values[n - period:n]) / period
    ema = seed
    out[n - period] = ema
    for i in range(n - period - 1, -1, -1):
        ema = alpha * values[i] + (1.0 - alpha) * ema
        out[i] = ema
    return out


def _atr_series(bars, period: int):
    n = len(bars)
    tr = [0.0] * n
    for i in range(n - 1, -1, -1):
        h, l, c = bars[i].high, bars[i].low, bars[i].close
        if i + 1 < n:
            pc = bars[i + 1].close
            tr[i] = max(h - l, abs(h - pc), abs(l - pc))
        else:
            tr[i] = h - l
    out = [0.0] * n
    if n >= period and period > 0:
        seed = sum(tr[n - period:n]) / period
        out[n - period] = seed
        for i in range(n - period - 1, -1, -1):
            out[i] = (out[i + 1] * (period - 1) + tr[i]) / period
    return out


def _local_hour_min(time_sec: int, offset_hours: int):
    shifted = time_sec + offset_hours * 3600
    secs = shifted % 86400
    return secs // 3600, (secs % 3600) // 60


def _session(hour: int) -> str:
    if 0 <= hour < 7:
        return "ASIA"
    if 7 <= hour < 13:
        return "LONDON"
    if 13 <= hour < 15:
        return "NY_OPEN"
    if 15 <= hour < 16:
        return "LONDON_CLOSE"
    if 16 <= hour < 21:
        return "NY"
    return "OUT"


def _kill_zone(hour: int, minute: int) -> str:
    if hour == 7 or hour == 8:
        return "LONDON_OPEN_KILL"
    if hour == 13 or (hour == 14 and minute <= 30):
        return "NY_OPEN_KILL"
    if 13 <= hour < 15:
        return "LONDON_NY_OVERLAP"
    return "NONE"


def _bar_shift_by_time(bars, time_sec: int, tolerancia_sec: int = 900) -> int:
    """Índice de la vela con time exacto o el más cercano dentro de ±tolerancia_sec."""
    best_idx = -1
    best_diff = float("inf")
    for i, b in enumerate(bars):
        diff = abs(b.time - time_sec)
        if diff == 0:
            return i
        if diff < best_diff and diff <= tolerancia_sec:
            best_diff = diff
            best_idx = i
    return best_idx


def _volume_ratio(bars, shift: int, n: int) -> float:
    if shift >= len(bars):
        return 1.0
    vol_signal = bars[shift].volume
    if vol_signal <= 0:
        return 1.0
    total = 0.0
    count = 0
    for i in range(shift + 1, shift + n + 1):
        if i < len(bars):
            v = bars[i].volume
            if v > 0:
                total += v
                count += 1
    if count == 0 or total <= 0:
        return 1.0
    return vol_signal / (total / count)


def _build_signal_id(time_sec: int, detector: str, direction: int, level: float, digits: int) -> int:
    h = time_sec & 0xFFFFFFFFFFFFFFFF
    for ch in detector.encode("utf-8"):
        h = (h * 31 + ch) & 0xFFFFFFFFFFFFFFFF
    h = (h * 31 + (direction + 2)) & 0xFFFFFFFFFFFFFFFF
    lvl = _round_digits(level, digits)
    h = (h * 31 + int(lvl * 100000.0)) & 0xFFFFFFFFFFFFFFFF
    return h


class ReferenceEngine:
    """Misma interfaz que el motor Rust (`PivotEngine`)."""

    def __init__(self, cal: Calibration, pending: list[dict] | None = None):
        self.cal = cal
        self.atr14_buffer: list[float] = []
        self.ema21_buffer: list[float] = []
        self.ema50_buffer: list[float] = []
        self.ema50_d1_buffer: list[float] = []
        self.ema200_d1_buffer: list[float] = []
        self.atr_history = [0.0] * 20
        self.g1 = 50.0
        self.g2 = 50.0
        self.g3 = 50.0
        self.g4 = 0.0
        self.last_g_calc_bar = 0
        self.estructura = {
            "swing_high": 0.0, "swing_low": 0.0, "swing_high_ant": 0.0, "swing_low_ant": 0.0,
            "sweep_nivel": 0.0, "sweep_dir": 0, "zona_alta": 0.0, "zona_baja": 0.0,
            "en_zona": False, "dir_estructura": "NEUTRO", "valida": False,
        }
        self.estructura_timestamp = 0
        self.last_struct_update = 0
        self.mss_valid = False
        self.mss_time = 0
        self.mss_bars_ago = 0
        self.mss_dir = ""
        self.mss_level = 0.0
        self.zona_valid = False
        self.zona_time = 0
        self.zona_mid = 0.0
        self.latches = [{"last_bar": 0, "key": "", "fired": False} for _ in range(6)]
        self.pending: list[Signal] = []
        self.vol_cache = {"time": 0, "shift": -1, "n": -1, "val": 1.0}
        for rec in (pending or []):
            s = Signal(**{k: v for k, v in rec.items() if hasattr(Signal, k)})
            self.pending.append(s)

    # ------------------------------------------------------------------ indicadores
    def _update_indicators(self, mkt: MarketData) -> bool:
        if len(mkt.m15) < 3 or len(mkt.d1) < 2:
            return False
        closes15 = [b.close for b in mkt.m15]
        self.atr14_buffer = _atr_series(mkt.m15, 14)
        self.ema21_buffer = _ema_series(closes15, 21)
        self.ema50_buffer = _ema_series(closes15, 50)
        d1c = [b.close for b in mkt.d1]
        self.ema50_d1_buffer = _ema_series(d1c, 50)
        self.ema200_d1_buffer = _ema_series(d1c, 200)
        return len(self.atr14_buffer) >= 3 and len(self.ema50_d1_buffer) >= 2

    def _trend_d1(self) -> str:
        if len(self.ema50_d1_buffer) < 2 or len(self.ema200_d1_buffer) < 2:
            return "NEUTRO"
        e50, e200 = self.ema50_d1_buffer[1], self.ema200_d1_buffer[1]
        if e50 == 0 or e200 == 0:
            return "NEUTRO"
        eps = e200 * 0.0005
        if e50 > e200 + eps:
            return "ALCISTA"
        if e50 < e200 - eps:
            return "BAJISTA"
        return "NEUTRO"

    def _trend_velas(self) -> int:
        if len(self.ema21_buffer) < 2 or len(self.ema50_buffer) < 2:
            return 0
        up = self.ema21_buffer[1] > self.ema50_buffer[1]
        down = self.ema21_buffer[1] < self.ema50_buffer[1]
        if not up and not down:
            return 0
        count = 0
        lim = min(_ATR_BUFFER_SIZE, len(self.ema21_buffer), len(self.ema50_buffer))
        for i in range(1, lim):
            u = self.ema21_buffer[i] > self.ema50_buffer[i]
            d = self.ema21_buffer[i] < self.ema50_buffer[i]
            if not u and not d:
                continue
            if up and not u:
                break
            if down and not d:
                break
            count += 1
        return count

    def _slope(self, atr: float) -> float:
        e0 = self.ema21_buffer[0] if self.ema21_buffer else 0.0
        e3 = self.ema21_buffer[3] if len(self.ema21_buffer) > 3 else e0
        return (e0 - e3) / atr if atr > 0 else 0.0

    def _vol_expanding(self) -> bool:
        if self.atr_history[10] == 0:
            return False
        vals = [v for v in self.atr_history[1:11] if v > 0]
        if not vals:
            return False
        avg = sum(vals) / len(vals)
        return self.atr_history[0] > avg * 1.30

    def _vol_compressing(self) -> bool:
        if self.atr_history[10] == 0:
            return False
        vals = [v for v in self.atr_history[1:11] if v > 0]
        if not vals:
            return False
        avg = sum(vals) / len(vals)
        return self.atr_history[0] < avg * 0.80

    def _volume_ratio_cached(self, mkt: MarketData, shift: int, n: int) -> float:
        bt = mkt.m15[shift].time if shift < len(mkt.m15) else -1
        if self.vol_cache["time"] == bt and self.vol_cache["shift"] == shift and self.vol_cache["n"] == n:
            return self.vol_cache["val"]
        r = _volume_ratio(mkt.m15, shift, n)
        self.vol_cache.update(time=bt, shift=shift, n=n, val=r)
        return r

    # ------------------------------------------------------------------ estructura
    def _detectar_pivots_h1(self, h1):
        depth, lookback = self.cal.pivot_depth, self.cal.pivot_lookback
        max_pivots = 50
        highs, lows = [], []
        start, end = depth + 1, lookback - depth - 1
        for i in range(start, min(end, len(h1))):
            if i >= max_pivots:
                break
            hi = h1[i].high
            if hi == 0:
                continue
            if all(h1[i - j].high < hi and h1[i + j].high < hi for j in range(1, depth + 1) if 0 <= i - j and i + j < len(h1)):
                highs.append(hi)
        for i in range(start, min(end, len(h1))):
            if i >= max_pivots:
                break
            lo = h1[i].low
            if lo == 0:
                continue
            if all(h1[i - j].low > lo and h1[i + j].low > lo for j in range(1, depth + 1) if 0 <= i - j and i + j < len(h1)):
                lows.append(lo)
        sh = max(highs) if highs else 0.0
        sl = min(lows) if lows else 0.0
        sha = max((h for h in highs if h < sh), default=0.0)
        cand = [l for l in lows if l > sl]
        sla = min(cand) if cand else 0.0
        return sh, sl, sha, sla

    def _actualizar_estructura(self, mkt: MarketData):
        h1 = mkt.h1
        price = mkt.m15[0].close
        atr = self.atr14_buffer[0]
        sh, sl, sha, sla = self._detectar_pivots_h1(h1)
        e = self.estructura
        e.update(swing_high=sh, swing_low=sl, swing_high_ant=sha, swing_low_ant=sla)
        e["sweep_nivel"], e["sweep_dir"] = 0.0, 0
        if sh > 0 and abs(price - sh) < atr * self.cal.sweep_distancia:
            e["sweep_nivel"], e["sweep_dir"] = sh, -1
        elif sl > 0 and abs(price - sl) < atr * self.cal.sweep_distancia:
            e["sweep_nivel"], e["sweep_dir"] = sl, 1
        margen = atr * self.cal.zona_margen
        if sh > 0 and sl > 0:
            e["zona_alta"], e["zona_baja"] = max(sh, sl) + margen, min(sh, sl) - margen
        elif sh > 0:
            e["zona_alta"], e["zona_baja"] = sh + margen, sh - margen
        elif sl > 0:
            e["zona_alta"], e["zona_baja"] = sl + margen, sl - margen
        else:
            e["zona_alta"], e["zona_baja"] = price + margen, price - margen
        e["en_zona"] = e["zona_baja"] <= price <= e["zona_alta"]
        if sh > 0 and sha > 0 and sl > 0 and sla > 0:
            hh, hl = sh > sha, sl > sla
            e["dir_estructura"] = "ALCISTA" if (hh and hl) else "BAJISTA" if (not hh and not hl) else "NEUTRO"
        else:
            e["dir_estructura"] = "NEUTRO"
        e["valida"] = sh > 0 or sl > 0 or e["sweep_nivel"] > 0

    # ------------------------------------------------------------------ caches
    def _detect_mss_cached(self, mkt: MarketData):
        current_h4 = mkt.h4[0].time if mkt.h4 else 0
        if self.mss_valid and self.mss_time == current_h4:
            return self.mss_bars_ago, self.mss_dir, self.mss_level
        r = self._detect_mss_h4(mkt.h4)
        if r:
            self.mss_valid, self.mss_time = True, current_h4
            self.mss_bars_ago, self.mss_dir, self.mss_level = r
        else:
            self.mss_valid = False
        return r

    def _detect_mss_h4(self, h4):
        lookback = min(self.cal.mss_lookback_h4, 50)
        for i in range(1, min(lookback, 50) + 1):
            if i >= len(h4):
                break
            close_i = h4[i].close
            if close_i == 0:
                continue
            if i + 1 >= len(h4):
                break
            ph = h4[i + 1].high
            pl = h4[i + 1].low
            for k in range(i + 1, min(i + lookback, 50) + 1):
                if k >= len(h4):
                    break
                hk, lk = h4[k].high, h4[k].low
                if hk == 0 or lk == 0:
                    break
                ph = max(ph, hk)
                pl = min(pl, lk)
            if close_i > ph:
                return i, "ALCISTA", ph
            if close_i < pl:
                return i, "BAJISTA", pl
        return None

    def _zona_premium(self, mkt: MarketData, nivel: float):
        cur = mkt.m15[0].time if mkt.m15 else 0
        if not (self.zona_valid and self.zona_time == cur):
            mx, mn = 0.0, 999999.0
            for i in range(1, 51):
                if i >= len(mkt.m15):
                    break
                b = mkt.m15[i]
                if b.high == 0 or b.low == 0:
                    break
                mx, mn = max(mx, b.high), min(mn, b.low)
            self.zona_mid = (mx + mn) / 2.0 if (mx > 0 and mn > 0 and mx > mn) else 0.0
            self.zona_valid, self.zona_time = True, cur
        if self.zona_mid > 0:
            return True, "PREMIUM" if nivel > self.zona_mid else "DISCOUNT"
        return False, "NEUTRO"

    def _ctx_estructural(self, direction: int, nivel: float) -> tuple[float, float]:
        e = self.estructura
        if not e["valida"] or e["sweep_nivel"] == 0:
            return 50.0, 0.0
        atr = self.atr14_buffer[0]
        if atr <= 0:
            return 50.0, 0.0
        tol = atr * 0.5
        p = self.cal.point
        dist = abs(nivel - e["sweep_nivel"]) / p
        score = 50.0 if dist <= tol / p else 30.0 if dist <= tol * 2 / p else 10.0
        if e["en_zona"]:
            score += 25.0
        if e["dir_estructura"] != "NEUTRO":
            score += 25.0 if ((direction == 1 and e["dir_estructura"] == "ALCISTA") or
                              (direction == -1 and e["dir_estructura"] == "BAJISTA")) else 5.0
        else:
            score += 10.0
        return _clamp(score), dist

    # ------------------------------------------------------------------ gauges
    def _g1(self) -> float:
        atr_now = self.atr14_buffer[0]
        if atr_now <= 0:
            return 50.0
        vals = [v for v in self.atr_history if v > 0]
        if not vals:
            return 50.0
        avg = sum(vals) / len(vals)
        if avg <= 0:
            return 50.0
        return _clamp((1.5 - atr_now / avg) / 1.0 * 100.0)

    def _g2(self, m15) -> float:
        up10 = down10 = up20 = down20 = 0
        for i in range(1, 21):
            if i >= len(m15):
                break
            up = m15[i].close > m15[i].open
            if i <= 10:
                if up:
                    up10 += 1
                else:
                    down10 += 1
            if up:
                up20 += 1
            else:
                down20 += 1
        d10 = max(up10, down10) / 10.0
        d20 = max(up20, down20) / 20.0
        return _clamp(_clamp((d10 - 0.5) / 0.5 * 100.0) * 0.6 + _clamp((d20 - 0.5) / 0.5 * 100.0) * 0.4)

    def _g3(self, m15) -> float:
        if len(m15) <= 10:
            return 50.0
        neto = abs(m15[0].close - m15[10].close)
        total = sum(b.high - b.low for b in m15[:10] if b.high and b.low)
        if total <= 0:
            return 50.0
        return _clamp(neto / total * 100.0)

    def _g4(self, m15) -> float:
        n, m = 6, 3
        mp = mu = cp = cu = 0.0
        for i in range(min(n, len(m15))):
            b = m15[i]
            if b.high == 0 or b.low == 0:
                break
            r = b.high - b.low
            if r <= 0:
                continue
            mecha = r - abs(b.close - b.open)
            cuerpo = abs(b.close - b.open)
            if i < m:
                mu += mecha
                cu += cuerpo
            else:
                mp += mecha
                cp += cuerpo
        atr = self.atr14_buffer[0]
        if atr <= 0:
            return 0.0
        return _clamp(_clamp(((mu - mp) / atr) * 50.0) + _clamp(((cp - cu) / atr) * 50.0))

    # ------------------------------------------------------------------ calidad/confluencia
    def _calidad_sweep(self, wick, reclaim, vol, bars_ago, equal_hl):
        t = (_clamp((wick - 0.55) / 0.45 * 40) + _clamp((reclaim - 0.55) / 0.45 * 35)
             + _clamp((6 - bars_ago) / 5.0 * 15) + _clamp(min(vol, 2) / 2 * 10))
        if equal_hl:
            t = min(100, t + 10)
        return _clamp(t)

    def _calidad_mss(self, wick, reclaim, mss_bars):
        denom = max(self.cal.mss_max_age_h4_bars - 1, 1)
        return _clamp(_clamp((wick - 0.55) / 0.45 * 40)
                      + _clamp((self.cal.mss_max_age_h4_bars - mss_bars) / denom * 30)
                      + _clamp((reclaim - 0.55) / 0.45 * 30))

    def _calidad_fvg(self, fvg_size, br, defendido):
        t = (_clamp((fvg_size - self.cal.fvg_min_size_atr) / (0.80 - self.cal.fvg_min_size_atr) * 45)
             + _clamp((br - self.cal.fvg_body_ratio) / (1.0 - self.cal.fvg_body_ratio) * 35))
        if defendido:
            t = min(100, t + 20)
        return _clamp(t)

    def _calidad_ob(self, impulso, ob_bars, vol):
        denom = max(self.cal.ob_lookback - 1, 1)
        return _clamp(_clamp((impulso - self.cal.ob_impulse_min) / (2.5 - self.cal.ob_impulse_min) * 50)
                      + _clamp((self.cal.ob_lookback - ob_bars) / denom * 30)
                      + _clamp(min(vol, 2) / 2 * 20))

    def _salud(self, trend, slope, trend_d1, direction):
        p3 = 25.0 if ((direction == 1 and trend_d1 == "ALCISTA") or (direction == -1 and trend_d1 == "BAJISTA")) else 0.0
        return _clamp(_clamp(min(trend, 15) / 15.0 * 40) + _clamp(min(abs(slope), 1) * 35) + p3)

    def _hubo_senal(self, det, dir_, n_velas):
        return any(s.detector == det and s.direction == dir_ and 0 <= s.entry_bar_shift <= n_velas for s in self.pending)

    def _conf_sweep_fvg(self, direction, fvg_ahora, fvg_size):
        if not self._hubo_senal("D2", direction, 6):
            return 0.0
        if not fvg_ahora:
            return 40.0
        return _clamp(60 + _clamp((fvg_size - self.cal.fvg_min_size_atr) / 0.60 * 40) * 0.4)

    def _conf_completa(self, direction, fvg_ahora, fvg_size):
        p = sum([self._hubo_senal("D5", direction, 8), self._hubo_senal("D2", direction, 8), bool(fvg_ahora)])
        if p == 0:
            return 0.0
        if p == 1:
            return 25.0
        if p == 2:
            return 60.0
        return _clamp(85 + _clamp((fvg_size - self.cal.fvg_min_size_atr) / 0.60 * 15) * 0.15)

    # ------------------------------------------------------------------ latches
    def _latch_fired(self, det: str, direction: int, cur_bar: int) -> bool:
        idx = {"D1": 0, "D2": 1, "D3": 2, "D3_DEF": 3, "D4": 4, "D5": 5}.get(det)
        if idx is None:
            return False
        key = f"{det}|{direction}"
        l = self.latches[idx]
        return l["fired"] and l["last_bar"] == cur_bar and l["key"] == key

    def _mark_latch(self, det: str, direction: int, cur_bar: int):
        idx = {"D1": 0, "D2": 1, "D3": 2, "D3_DEF": 3, "D4": 4, "D5": 5}.get(det)
        if idx is None:
            return
        self.latches[idx] = {"last_bar": cur_bar, "key": f"{det}|{direction}", "fired": True}

    # ------------------------------------------------------------------ hipótesis
    def _vencimiento(self, sig: Signal) -> int:
        atr = sig.atr14
        if sig.detector == "D1":
            return 2 if atr > 20 else 1
        if sig.detector in ("D2", "D2_ANTICIPACION"):
            if sig.kill_zone != "NONE":
                return 1
            return 1 if atr > 15 else 2
        if sig.detector in ("D3", "D3_DEF"):
            if sig.detector == "D3_DEF":
                return 2
            return 2 if atr > 20 else 1
        if sig.detector == "D4":
            return 1 if sig.ob_confluence else 2
        if sig.detector == "D5":
            return 2 if sig.kill_zone != "NONE" else 4
        return 2

    def _fmt(self, v: float, digits: int | None = None) -> str:
        d = digits if digits is not None else self.cal.digits
        return f"{v:.{d}f}"

    def _generar_hipotesis(self, sig: Signal, mkt: MarketData):
        sig.hipotesis_expiry_velas = self._vencimiento(sig)
        sig.hipotesis_expiry_minutos = sig.hipotesis_expiry_velas * 15
        ok, zona = self._zona_premium(mkt, sig.entry_price)
        if ok:
            sig.hipotesis_zona = zona

        if self.estructura["sweep_nivel"] > 0:
            sig.hipotesis_objetivo = self.estructura["sweep_nivel"]
        else:
            atr = sig.atr14 * self.cal.point
            if atr <= 0:
                atr = self.atr14_buffer[0]
            sig.hipotesis_objetivo = sig.entry_price + atr * 1.5 if sig.direction == 1 else sig.entry_price - atr * 1.5

        prob_base = self.cal.prob_base_for(sig.detector)
        atr = sig.atr14 * self.cal.point
        if atr <= 0:
            atr = self.atr14_buffer[0]
        if atr <= 0:
            return

        dig = self.cal.digits
        causa = efecto = razon = invalidez = ""
        if sig.detector == "D1":
            d = "alcista" if sig.direction == 1 else "bajista"
            causa = f"Ruptura {d} de {self._fmt(sig.nivel_estructural)}"
            efecto = f"va a provocar continuación {d} hacia {self._fmt(sig.hipotesis_objetivo)}"
            razon = f"porque la vela actual rompe con fuerza (BR={sig.br:.2f}) y la tendencia {sig.estructura_direccion} confirma"
            inv = sig.nivel_estructural - atr * 0.3 if sig.direction == 1 else sig.nivel_estructural + atr * 0.3
            invalidez = f"Si rompe {self._fmt(inv)} en contra, se invalida"
            if sig.br > 0.70:
                prob_base += 5
            if sig.bs > 1.0:
                prob_base += 5
            if sig.g1_compresion >= 60:
                prob_base += 5
            if sig.g2_persistencia >= 60:
                prob_base += 5
            if sig.kill_zone != "NONE":
                prob_base += 5
        elif sig.detector in ("D2", "D2_ANTICIPACION"):
            accion = "rebote alcista" if sig.direction == 1 else "rechazo bajista"
            causa = f"Sweep en {self._fmt(sig.level_swept)} en zona {sig.hipotesis_zona}"
            efecto = f"va a provocar {accion} hacia {self._fmt(sig.hipotesis_objetivo)}"
            razon = f"porque el sweep liquida stops y la tendencia {sig.estructura_direccion} confirma"
            inv = sig.level_swept - atr * 0.3 if sig.direction == 1 else sig.level_swept + atr * 0.3
            invalidez = f"Si rompe {self._fmt(inv)}, se invalida"
            if sig.equal_hl_detected:
                prob_base += 5
            if sig.hipotesis_zona == "PREMIUM" and sig.direction == -1:
                prob_base += 5
            if sig.hipotesis_zona == "DISCOUNT" and sig.direction == 1:
                prob_base += 5
            if sig.kill_zone != "NONE":
                prob_base += 5
            if sig.sweep_volume_ratio > 1.8:
                prob_base += 5
            if sig.g4_agotamiento >= 65:
                prob_base -= 10
        elif sig.detector in ("D3", "D3_DEF"):
            df = "BAJISTA" if sig.direction == -1 else "ALCISTA"
            accion = "rechazo bajista" if sig.direction == -1 else "rebote alcista"
            defensa = ("Los vendedores" if sig.direction == -1 else "Los compradores") + " defienden la zona" if sig.detector == "D3_DEF" else "La zona está activa"
            causa = f"FVG {df} en zona {sig.hipotesis_zona}"
            efecto = f"va a provocar {accion} hacia {self._fmt(sig.hipotesis_objetivo)}"
            razon = f"porque {defensa} y la tendencia {sig.estructura_direccion} confirma"
            inv = sig.fvg_top if sig.direction == -1 else sig.fvg_bottom
            di = "al alza" if sig.direction == -1 else "a la baja"
            invalidez = f"Si rompe {self._fmt(inv)} {di}, se invalida"
            if sig.detector == "D3_DEF":
                prob_base += 5
            if sig.hipotesis_zona == "PREMIUM" and sig.direction == -1:
                prob_base += 5
            if sig.hipotesis_zona == "DISCOUNT" and sig.direction == 1:
                prob_base += 5
            if sig.kill_zone != "NONE":
                prob_base += 5
            if sig.g1_compresion >= 60:
                prob_base += 5
            if sig.mss_aligned:
                prob_base += 5
            if sig.g4_agotamiento >= 65:
                prob_base -= 10
        elif sig.detector == "D4":
            accion = "rebote alcista" if sig.direction == 1 else "rechazo bajista"
            nivel = (sig.ob_high + sig.ob_low) / 2.0
            causa = f"Order Block en {self._fmt(nivel)}"
            efecto = f"va a provocar {accion} hacia {self._fmt(sig.hipotesis_objetivo)}"
            razon = f"porque el OB representa acumulación/distribución y la tendencia {sig.estructura_direccion} confirma"
            inv = sig.ob_low if sig.direction == 1 else sig.ob_high
            invalidez = f"Si rompe {self._fmt(inv)}, se invalida"
            if sig.ob_impulse_atr > 1.5:
                prob_base += 5
            if sig.ob_bars_ago <= 3:
                prob_base += 5
            if sig.kill_zone != "NONE":
                prob_base += 5
            if sig.g1_compresion >= 60:
                prob_base += 5
        elif sig.detector == "D5":
            accion = "continuación alcista" if sig.direction == 1 else "continuación bajista"
            causa = f"MSS H4 {sig.mss_direction} con sweep en {self._fmt(sig.level_swept)}"
            efecto = f"va a provocar {accion} hacia {self._fmt(sig.hipotesis_objetivo)}"
            razon = "porque el cambio de estructura en H4 confirma la dirección y el sweep valida la entrada"
            inv = sig.level_swept - atr * 0.5 if sig.direction == 1 else sig.level_swept + atr * 0.5
            invalidez = f"Si rompe {self._fmt(inv)}, se invalida"
            if sig.mss_bars_ago_h4 <= 4:
                prob_base += 5
            if sig.kill_zone != "NONE":
                prob_base += 5
            if sig.g1_compresion >= 60:
                prob_base += 5
            if sig.g2_persistencia >= 60:
                prob_base += 5

        prob_base = max(30, min(95, prob_base))
        sig.hipotesis_prob_min = max(30, prob_base - 5)
        sig.hipotesis_prob_max = min(95, prob_base + 5)
        sig.hipotesis_causa = causa
        sig.hipotesis_efecto = efecto
        sig.hipotesis_razon = razon
        sig.hipotesis_invalidez = invalidez
        sig.hipotesis_texto = "\n".join([causa, efecto, razon, invalidez])

    # ------------------------------------------------------------------ detectores
    def _motor_d1(self, mkt: MarketData, ctx: dict, out: list):
        b0 = mkt.m15[0]
        high0, low0, close0, open0 = b0.high, b0.low, b0.close, b0.open
        if high0 == 0 or low0 == 0 or close0 == 0 or len(mkt.m15) < 2:
            return
        atr = self.atr14_buffer[0]
        if atr <= 0:
            return
        highest_high = mkt.m15[1].high
        lowest_low = mkt.m15[1].low
        for k in range(2, min(self.cal.n_ruptura + 1, 100) + 1):
            if k >= len(mkt.m15):
                break
            b = mkt.m15[k]
            if b.high == 0 or b.low == 0:
                break
            highest_high = max(highest_high, b.high)
            lowest_low = min(lowest_low, b.low)
        direction = nivel = penet = 0.0
        if high0 > highest_high:
            direction, nivel, penet = 1, highest_high, (high0 - highest_high) / atr
        elif low0 < lowest_low:
            direction, nivel, penet = -1, lowest_low, (lowest_low - low0) / atr
        if direction == 0 or penet < self.cal.d1_atr_threshold:
            return
        rango0 = high0 - low0
        if rango0 <= 0:
            return
        br0 = abs(close0 - open0) / rango0
        if br0 < self.cal.body_ratio_min:
            return
        if self.cal.d1_use_volume and _volume_ratio(mkt.m15, 0, 20) < self.cal.d1_min_volume:
            return
        if self.cal.d1_use_retest:
            retested = (low0 <= nivel and close0 > nivel) if direction == 1 else (high0 >= nivel and close0 < nivel)
            if not retested:
                return
        cur = mkt.m15[0].time
        sig_id = _build_signal_id(cur, "D1", int(direction), nivel, self.cal.digits)
        if self._is_duplicate(sig_id) or self._latch_fired("D1", int(direction), cur):
            return
        self._mark_latch("D1", int(direction), cur)
        sig = Signal(id=sig_id, entry_time=cur, entry_bar_shift=0, symbol=self.cal.symbol,
                     direction=int(direction), entry_price=close0, detector="D1",
                     br=br0, bs=penet, nivel_estructural=nivel, atr14=atr / self.cal.point,
                     session=ctx["session"], kill_zone=ctx["kill_zone"], trend_d1=ctx["trend_d1"],
                     estructura_direccion=self.estructura["dir_estructura"],
                     g1_compresion=self.g1, g2_persistencia=self.g2, g4_agotamiento=self.g4,
                     volume_ratio=ctx["vol_ratio"], vol_expanding=ctx["vol_exp"], vol_compressing=ctx["vol_comp"])
        sig.tipo = self._clasificar_d1(br0, penet, ctx["session"])
        sig.salud_tendencial = self._salud(ctx["trend_velas"], self._slope(atr), ctx["trend_d1"], int(direction))
        cs, dist = self._ctx_estructural(int(direction), nivel)
        sig.contexto_estructural, sig.distancia_al_sweep = cs, dist
        sig.en_zona_estructural = self.estructura["en_zona"]
        out.append(sig)

    def _motor_d2(self, mkt: MarketData, ctx: dict, out: list):
        b0 = mkt.m15[0]
        close0, open0, high0, low0 = b0.close, b0.open, b0.high, b0.low
        if close0 == 0 or high0 == 0 or low0 == 0:
            return
        atr = self.atr14_buffer[0]
        if atr <= 0:
            return
        sweep_bar = sweep_dir = -1
        wick_found = vol_found = level = 0.0
        equal_hl = False
        for i in range(1, 3):
            if i >= len(mkt.m15):
                break
            bi = mkt.m15[i]
            hi, li = bi.high, bi.low
            if hi == 0 or li == 0:
                continue
            oi, ci, ri = bi.open, bi.close, hi - li
            if ri <= 0:
                continue
            ph = mkt.m15[i + 1].high if i + 1 < len(mkt.m15) else 0.0
            pl = mkt.m15[i + 1].low if i + 1 < len(mkt.m15) else 0.0
            for k in range(i + 1, min(i + self.cal.sweep_n, 100) + 1):
                if k >= len(mkt.m15):
                    break
                bk = mkt.m15[k]
                if bk.high == 0 or bk.low == 0:
                    break
                ph, pl = max(ph, bk.high), min(pl, bk.low)
            if ph == 0 or pl == 0:
                continue
            per_high, per_low = hi > ph and ci < ph, li < pl and ci > pl
            if not per_high and not per_low:
                continue
            if per_high:
                wr, dc, lc = (hi - max(oi, ci)) / ri, -1, ph
            else:
                wr, dc, lc = (min(oi, ci) - li) / ri, 1, pl
            if wr < self.cal.sweep_wick_min:
                continue
            eq = False
            for j in range(i + 1, min(i + self.cal.equal_hl_window, 100) + 1):
                if j >= len(mkt.m15):
                    break
                bj = mkt.m15[j]
                if bj.high == 0 or bj.low == 0:
                    break
                if per_high and abs(bj.high - lc) <= self.cal.equal_hl_tol * atr:
                    eq = True
                    break
                if not per_high and abs(bj.low - lc) <= self.cal.equal_hl_tol * atr:
                    eq = True
                    break
            equal_hl = eq or equal_hl
            sweep_bar, sweep_dir, wick_found, vol_found, level = i, dc, wr, _volume_ratio(mkt.m15, i, self.cal.sweep_n), lc
            break
        if sweep_bar == -1 or sweep_bar > 2 or abs(close0 - level) > atr * 2.0:
            return
        br_reclaim = abs(close0 - open0) / (high0 - low0) if high0 - low0 > 0 else 0.0
        reclaim_ok = (sweep_dir == 1 and close0 > open0 and close0 > level) or (sweep_dir == -1 and close0 < open0 and close0 < level)
        if not reclaim_ok or br_reclaim < self.cal.reclaim_body_min:
            return
        cur = mkt.m15[0].time
        sig_id = _build_signal_id(cur, "D2", sweep_dir, level, self.cal.digits)
        if self._is_duplicate(sig_id) or self._latch_fired("D2", sweep_dir, cur):
            return
        self._mark_latch("D2", sweep_dir, cur)
        sig = Signal(id=sig_id, entry_time=cur, entry_bar_shift=0, symbol=self.cal.symbol,
                     direction=sweep_dir, entry_price=close0, detector="D2",
                     level_swept=level, sweep_wick_ratio=wick_found, sweep_volume_ratio=vol_found,
                     reclaim_body_ratio=br_reclaim, equal_hl_detected=equal_hl,
                     atr14=atr / self.cal.point, session=ctx["session"], kill_zone=ctx["kill_zone"],
                     trend_d1=ctx["trend_d1"], estructura_direccion=self.estructura["dir_estructura"],
                     g1_compresion=self.g1, g2_persistencia=self.g2, g4_agotamiento=self.g4,
                     volume_ratio=ctx["vol_ratio"], vol_expanding=ctx["vol_exp"], vol_compressing=ctx["vol_comp"])
        sig.tipo = self._clasificar_d2(wick_found, vol_found, br_reclaim, equal_hl)
        sig.calidad_sweep = self._calidad_sweep(wick_found, br_reclaim, vol_found, sweep_bar, equal_hl)
        sig.salud_tendencial = self._salud(ctx["trend_velas"], self._slope(atr), ctx["trend_d1"], sweep_dir)
        cs, dist = self._ctx_estructural(sweep_dir, level)
        sig.contexto_estructural, sig.distancia_al_sweep = cs, dist
        sig.en_zona_estructural = self.estructura["en_zona"]
        out.append(sig)

    def _motor_d2_anticipacion(self, mkt: MarketData, ctx: dict, out: list):
        b0 = mkt.m15[0]
        high0, low0, close0, open0 = b0.high, b0.low, b0.close, b0.open
        if high0 == 0 or low0 == 0 or close0 == 0 or open0 == 0 or len(mkt.m15) < 2:
            return
        atr = self.atr14_buffer[0]
        if atr <= 0:
            return
        prior_high, prior_low = mkt.m15[1].high, mkt.m15[1].low
        for k in range(2, min(self.cal.sweep_n, 100) + 1):
            if k >= len(mkt.m15):
                break
            b = mkt.m15[k]
            if b.high == 0 or b.low == 0:
                break
            prior_high, prior_low = max(prior_high, b.high), min(prior_low, b.low)
        sweep_high, sweep_low = high0 > prior_high, low0 < prior_low
        if not sweep_high and not sweep_low:
            return
        sweep_dir = -1 if sweep_high else 1
        nivel = prior_high if sweep_high else prior_low
        rango = high0 - low0
        if rango <= 0:
            return
        wick = (high0 - max(open0, close0)) / rango if sweep_high else (min(open0, close0) - low0) / rango
        if wick < self.cal.sweep_wick_min * 0.6:
            return
        confluencias = 0
        hay_fvg = False
        for i in range(2, 6):
            if i >= len(mkt.m15):
                break
            ba, bb, bc = mkt.m15[i], mkt.m15[i - 1], mkt.m15[i - 2]
            if 0 in (ba.high, ba.low, bb.high, bb.low, bc.high, bc.low):
                continue
            if ba.high < bc.low:
                ce = bc.low - (bc.low - ba.high) * 0.5
                if abs(nivel - ce) < atr * 0.5:
                    hay_fvg = True
                    break
            elif ba.low > bc.high:
                ce = ba.low - (ba.low - bc.high) * 0.5
                if abs(nivel - ce) < atr * 0.5:
                    hay_fvg = True
                    break
        if hay_fvg:
            confluencias += 1
        hay_ob = False
        for i in range(2, 5):
            if i >= len(mkt.m15):
                break
            bi = mkt.m15[i]
            oi, ci, hi, li, ri = bi.open, bi.close, bi.high, bi.low, bi.high - bi.low
            if ri <= 0 or hi == 0 or li == 0:
                continue
            if abs(ci - oi) / ri < self.cal.ob_body_min:
                continue
            nc = mkt.m15[i - 1].close
            if abs(nc - ci) / atr < self.cal.ob_impulse_min:
                continue
            if abs(nivel - (hi + li) / 2.0) < atr * 0.5:
                hay_ob = True
                break
        if hay_ob:
            confluencias += 1
        r = self._detect_mss_cached(mkt)
        if r:
            md = 1 if r[1] == "ALCISTA" else -1
            if md == sweep_dir:
                confluencias += 1
        if confluencias < 2:
            return
        cur = mkt.m15[0].time
        sig_id = _build_signal_id(cur, "D2_ANTICIPACION", sweep_dir, nivel, self.cal.digits)
        if self._is_duplicate(sig_id):
            return
        sig = Signal(id=sig_id, entry_time=cur, entry_bar_shift=0, symbol=self.cal.symbol,
                     direction=sweep_dir, entry_price=close0, detector="D2_ANTICIPACION",
                     level_swept=nivel, sweep_wick_ratio=wick, sweep_volume_ratio=ctx["vol_ratio"],
                     atr14=atr / self.cal.point, session=ctx["session"], kill_zone=ctx["kill_zone"],
                     trend_d1=ctx["trend_d1"], estructura_direccion=self.estructura["dir_estructura"],
                     g1_compresion=self.g1, g2_persistencia=self.g2, g4_agotamiento=self.g4,
                     volume_ratio=ctx["vol_ratio"], vol_expanding=ctx["vol_exp"], vol_compressing=ctx["vol_comp"])
        sig.tipo = self._clasificar_d2_ant(wick, ctx["vol_ratio"], confluencias)
        sig.calidad_sweep = self._calidad_sweep(wick, 0.0, ctx["vol_ratio"], 0, False)
        sig.salud_tendencial = self._salud(ctx["trend_velas"], self._slope(atr), ctx["trend_d1"], sweep_dir)
        cs, dist = self._ctx_estructural(sweep_dir, nivel)
        sig.contexto_estructural, sig.distancia_al_sweep = cs, dist
        sig.en_zona_estructural = self.estructura["en_zona"]
        sig.conf_sweep_fvg = self._conf_sweep_fvg(sweep_dir, True, 0.0) if hay_fvg else 0.0
        sig.conf_completa = self._conf_completa(sweep_dir, hay_fvg, 0.0)
        out.append(sig)

    def _motor_d3(self, mkt: MarketData, ctx: dict, out: list):
        if len(mkt.m15) < 3:
            return
        b2, b1, b0 = mkt.m15[2], mkt.m15[1], mkt.m15[0]
        ha, la, hb, lb, cb, ob, hc, lc = b2.high, b2.low, b1.high, b1.low, b1.close, b1.open, b0.high, b0.low
        if 0 in (ha, la, hb, lb, hc, lc):
            return
        atr = self.atr14_buffer[0]
        if atr <= 0:
            return
        fvg_alcista, fvg_bajista = ha < lc, la > hc
        if not fvg_alcista and not fvg_bajista:
            return
        if fvg_alcista:
            fvg_size, fvg_top, fvg_bottom, direction = lc - ha, lc, ha, 1
        else:
            fvg_size, fvg_top, fvg_bottom, direction = la - hc, la, hc, -1
        if fvg_size <= 0:
            return
        fvg_size_atr = fvg_size / atr
        br_b = abs(cb - ob) / (hb - lb) if hb - lb > 0 else 0.0
        dir_ok = (fvg_alcista and cb > ob) or (fvg_bajista and cb < ob)
        if fvg_size_atr < self.cal.fvg_min_size_atr or br_b < self.cal.fvg_body_ratio or not dir_ok:
            return
        mit = fvg_bottom + (fvg_top - fvg_bottom) * self.cal.fvg_mitig_umbral
        price0 = b0.close
        mitigado = (direction == 1 and price0 <= mit) or (direction == -1 and price0 >= mit)
        defendido = (direction == 1 and price0 > fvg_top) or (direction == -1 and price0 < fvg_bottom)
        dentro = (direction == 1 and fvg_bottom <= price0 <= fvg_top) or (direction == -1 and fvg_top >= price0 >= fvg_bottom)
        det = "D3_DEF" if (defendido or dentro) else "D3"
        cur = mkt.m15[0].time
        sig_id = _build_signal_id(b1.time, det, direction, fvg_top, self.cal.digits)
        if self._is_duplicate(sig_id) or self._latch_fired(det, direction, cur):
            return
        self._mark_latch(det, direction, cur)
        sig = Signal(id=sig_id, entry_time=b1.time, entry_bar_shift=1, symbol=self.cal.symbol,
                     direction=direction, entry_price=price0, detector=det,
                     fvg_top=fvg_top, fvg_bottom=fvg_bottom, fvg_size_atr=fvg_size_atr, fvg_mitigated=mitigado,
                     atr14=atr / self.cal.point, session=ctx["session"], kill_zone=ctx["kill_zone"],
                     trend_d1=ctx["trend_d1"], estructura_direccion=self.estructura["dir_estructura"],
                     g1_compresion=self.g1, g2_persistencia=self.g2, g4_agotamiento=self.g4,
                     volume_ratio=ctx["vol_ratio"], vol_expanding=ctx["vol_exp"], vol_compressing=ctx["vol_comp"])
        r = self._detect_mss_cached(mkt)
        if r:
            sig.mss_aligned, sig.mss_bars_ago_h4, sig.mss_direction, sig.mss_level = True, r[0], r[1], r[2]
        slope = self._slope(atr)
        sig.tipo = self._clasificar_d3(fvg_size_atr, br_b, ctx["trend_velas"], slope)
        sig.calidad_fvg = self._calidad_fvg(fvg_size_atr, br_b, defendido)
        sig.salud_tendencial = self._salud(ctx["trend_velas"], slope, ctx["trend_d1"], direction)
        cs, dist = self._ctx_estructural(direction, fvg_top)
        sig.contexto_estructural, sig.distancia_al_sweep = cs, dist
        sig.en_zona_estructural = self.estructura["en_zona"]
        sig.conf_sweep_fvg = self._conf_sweep_fvg(direction, True, fvg_size_atr)
        sig.conf_completa = self._conf_completa(direction, True, fvg_size_atr)
        out.append(sig)

    def _motor_d4(self, mkt: MarketData, ctx: dict, out: list):
        close0 = mkt.m15[0].close
        if close0 == 0:
            return
        atr = self.atr14_buffer[0]
        if atr <= 0:
            return
        ob_bar = ob_dir = -1
        ob_high = ob_low = ob_impulse = ob_vol = 0.0
        for i in range(2, 5):
            if i >= len(mkt.m15):
                break
            bi = mkt.m15[i]
            oi, ci, hi, li, ri = bi.open, bi.close, bi.high, bi.low, bi.high - bi.low
            if ri <= 0 or hi == 0 or li == 0:
                continue
            if abs(ci - oi) / ri < self.cal.ob_body_min:
                continue
            di = 1 if ci > oi else -1
            nc = mkt.m15[i - 1].close
            imp = abs(nc - ci) / atr
            if imp < self.cal.ob_impulse_min:
                continue
            tested = False
            for j in range(i - 1, 0, -1):
                bj = mkt.m15[j]
                if bj.high == 0 or bj.low == 0:
                    break
                if (di == 1 and bj.low <= hi) or (di == -1 and bj.high >= li):
                    tested = True
                    break
            if tested:
                continue
            ob_bar, ob_dir, ob_high, ob_low, ob_impulse = i, di, hi, li, imp
            ob_vol = _volume_ratio(mkt.m15, i, self.cal.ob_lookback)
            break
        if ob_bar == -1 or ob_bar > 4:
            return
        entering = (ob_dir == 1 and ob_low <= close0 <= ob_high) or (ob_dir == -1 and ob_low <= close0 <= ob_high)
        if not entering:
            return
        centro = (ob_high + ob_low) / 2.0
        if abs(close0 - centro) > atr * 2.0:
            return
        cur = mkt.m15[0].time
        sig_id = _build_signal_id(cur, "D4", ob_dir, ob_high, self.cal.digits)
        if self._is_duplicate(sig_id) or self._latch_fired("D4", ob_dir, cur):
            return
        self._mark_latch("D4", ob_dir, cur)
        sig = Signal(id=sig_id, entry_time=cur, entry_bar_shift=0, symbol=self.cal.symbol,
                     direction=ob_dir, entry_price=close0, detector="D4",
                     ob_high=ob_high, ob_low=ob_low, ob_bars_ago=ob_bar, ob_impulse_atr=ob_impulse, ob_confluence=True,
                     atr14=atr / self.cal.point, session=ctx["session"], kill_zone=ctx["kill_zone"],
                     trend_d1=ctx["trend_d1"], estructura_direccion=self.estructura["dir_estructura"],
                     g1_compresion=self.g1, g2_persistencia=self.g2, volume_ratio=ctx["vol_ratio"],
                     vol_expanding=ctx["vol_exp"], vol_compressing=ctx["vol_comp"])
        sig.tipo = self._clasificar_d4(ob_impulse, ob_vol, ob_bar)
        sig.calidad_ob = self._calidad_ob(ob_impulse, ob_bar, ob_vol)
        sig.salud_tendencial = self._salud(ctx["trend_velas"], self._slope(atr), ctx["trend_d1"], ob_dir)
        cs, dist = self._ctx_estructural(ob_dir, centro)
        sig.contexto_estructural, sig.distancia_al_sweep = cs, dist
        sig.en_zona_estructural = self.estructura["en_zona"]
        out.append(sig)

    def _motor_d5(self, mkt: MarketData, ctx: dict, out: list):
        r = self._detect_mss_cached(mkt)
        if not r:
            return
        mss_bars, mss_dir, mss_level = r
        if mss_bars > self.cal.mss_max_age_h4_bars:
            return
        mss_dir_int = 1 if mss_dir == "ALCISTA" else -1
        b0 = mkt.m15[0]
        close0, open0, high0, low0 = b0.close, b0.open, b0.high, b0.low
        if close0 == 0 or high0 == 0 or low0 == 0:
            return
        atr = self.atr14_buffer[0]
        if atr <= 0:
            return
        sweep_bar = -1
        wick_found = level = 0.0
        for i in range(1, 3):
            if i >= len(mkt.m15):
                break
            bi = mkt.m15[i]
            hi, li, oi, ci, ri = bi.high, bi.low, bi.open, bi.close, bi.high - bi.low
            if hi == 0 or li == 0 or ri <= 0:
                continue
            ph = mkt.m15[i + 1].high if i + 1 < len(mkt.m15) else 0.0
            pl = mkt.m15[i + 1].low if i + 1 < len(mkt.m15) else 0.0
            for k in range(i + 1, min(i + self.cal.sweep_n, 100) + 1):
                if k >= len(mkt.m15):
                    break
                bk = mkt.m15[k]
                if bk.high == 0 or bk.low == 0:
                    break
                ph, pl = max(ph, bk.high), min(pl, bk.low)
            if ph == 0 or pl == 0:
                continue
            if mss_dir_int == 1:
                if not (li < pl and ci > pl):
                    continue
                w = (min(oi, ci) - li) / ri
                if w < self.cal.sweep_wick_min:
                    continue
                sweep_bar, wick_found, level = i, w, pl
                break
            else:
                if not (hi > ph and ci < ph):
                    continue
                w = (hi - max(oi, ci)) / ri
                if w < self.cal.sweep_wick_min:
                    continue
                sweep_bar, wick_found, level = i, w, ph
                break
        if sweep_bar == -1 or sweep_bar > 2 or abs(close0 - level) > atr * 2.0:
            return
        br_reclaim = abs(close0 - open0) / (high0 - low0) if high0 - low0 > 0 else 0.0
        reclaim_ok = (mss_dir_int == 1 and close0 > level) or (mss_dir_int == -1 and close0 < level)
        if not reclaim_ok or br_reclaim < self.cal.reclaim_body_min:
            return
        cur = mkt.m15[0].time
        sig_id = _build_signal_id(cur, "D5", mss_dir_int, level, self.cal.digits)
        if self._is_duplicate(sig_id) or self._latch_fired("D5", mss_dir_int, cur):
            return
        self._mark_latch("D5", mss_dir_int, cur)
        sig = Signal(id=sig_id, entry_time=cur, entry_bar_shift=0, symbol=self.cal.symbol,
                     direction=mss_dir_int, entry_price=close0, detector="D5",
                     mss_aligned=True, mss_direction=mss_dir, mss_bars_ago_h4=mss_bars, mss_level=mss_level,
                     level_swept=level, sweep_wick_ratio=wick_found, reclaim_body_ratio=br_reclaim,
                     atr14=atr / self.cal.point, session=ctx["session"], kill_zone=ctx["kill_zone"],
                     trend_d1=ctx["trend_d1"], estructura_direccion=self.estructura["dir_estructura"],
                     g1_compresion=self.g1, g2_persistencia=self.g2, g4_agotamiento=self.g4,
                     volume_ratio=ctx["vol_ratio"], vol_expanding=ctx["vol_exp"], vol_compressing=ctx["vol_comp"])
        sig.tipo = self._clasificar_d5(mss_bars, wick_found, br_reclaim, ctx["kill_zone"])
        sig.calidad_sweep = self._calidad_sweep(wick_found, br_reclaim, ctx["vol_ratio"], sweep_bar, False)
        sig.calidad_mss = self._calidad_mss(wick_found, br_reclaim, mss_bars)
        sig.salud_tendencial = self._salud(ctx["trend_velas"], self._slope(atr), ctx["trend_d1"], mss_dir_int)
        cs, dist = self._ctx_estructural(mss_dir_int, level)
        sig.contexto_estructural, sig.distancia_al_sweep = cs, dist
        sig.en_zona_estructural = self.estructura["en_zona"]
        sig.conf_completa = self._conf_completa(mss_dir_int, False, 0.0)
        out.append(sig)

    # ------------------------------------------------------------------ clasificadores
    def _clasificar_d1(self, br, bs, session):
        if session in ("ASIA", "OUT"):
            return "B" if br > 0.70 and bs > 0.80 else "D"
        if br > 0.70 and bs > 0.80:
            return "A"
        if br > 0.60 and bs > 0.50:
            return "B"
        if br > self.cal.body_ratio_min and bs > 0.30:
            return "C"
        return "D"

    def _clasificar_d2(self, wick, vol, reclaim, equal_hl):
        if equal_hl and wick > 0.70 and vol > 1.80 and reclaim > 0.70:
            return "A"
        if wick > 0.65 and vol > 1.50 and reclaim > 0.60:
            return "B"
        if wick > self.cal.sweep_wick_min and reclaim > self.cal.reclaim_body_min:
            return "C"
        return "D"

    def _clasificar_d2_ant(self, wick, vol, confluencias):
        if confluencias >= 3 and wick > 0.65 and vol > 1.50:
            return "A"
        if confluencias >= 2 and wick > 0.55 and vol > 1.20:
            return "B"
        if confluencias >= 2:
            return "C"
        return "D"

    def _clasificar_d3(self, fvg_size, br, trend, slope):
        if fvg_size > 0.50 and br > 0.70 and trend >= 3:
            return "A"
        if fvg_size > 0.35 and br > 0.60:
            return "B"
        if fvg_size > self.cal.fvg_min_size_atr and br > self.cal.fvg_body_ratio:
            return "C"
        return "D"

    def _clasificar_d4(self, impulso, vol, ob_bars):
        if impulso > 1.80 and vol > 1.50 and ob_bars <= 6:
            return "A"
        if impulso > 1.40 and vol > 1.20:
            return "B"
        if impulso >= self.cal.ob_impulse_min:
            return "C"
        return "D"

    def _clasificar_d5(self, mss_bars, wick, reclaim, kill_zone):
        in_kill = kill_zone in ("LONDON_OPEN_KILL", "NY_OPEN_KILL")
        if mss_bars <= 4 and wick > 0.70 and reclaim > 0.70 and in_kill:
            return "A"
        if mss_bars <= 8 and wick > 0.60 and reclaim > 0.60:
            return "B"
        if wick > self.cal.sweep_wick_min and reclaim > self.cal.reclaim_body_min:
            return "C"
        return "D"

    # ------------------------------------------------------------------ flujo principal
    def _is_duplicate(self, sig_id: int) -> bool:
        return any(s.id == sig_id for s in self.pending)

    def process(self, mkt: MarketData) -> EngineOutput:
        out = EngineOutput()
        if not self._update_indicators(mkt):
            return out
        self._measure_returns(mkt, out)
        for i in range(19, 0, -1):
            self.atr_history[i] = self.atr_history[i - 1]
        self.atr_history[0] = self.atr14_buffer[0]

        max_lookback = max(self.cal.n_ruptura, self.cal.sweep_n, self.cal.ob_lookback, 10)
        vol_ratio = self._volume_ratio_cached(mkt, 0, max_lookback)

        cur = mkt.m15[0].time
        hour, minute = _local_hour_min(cur, self.cal.utc_offset_hours)
        session = _session(hour)
        kill_zone = _kill_zone(hour, minute)
        vol_exp, vol_comp = self._vol_expanding(), self._vol_compressing()
        trend_d1 = self._trend_d1()
        trend_velas = self._trend_velas()

        if cur != self.last_g_calc_bar:
            self.g1 = self._g1()
            self.g2 = self._g2(mkt.m15)
            self.g3 = self._g3(mkt.m15)
            self.g4 = self._g4(mkt.m15)
            self.last_g_calc_bar = cur

        current_h1 = mkt.h1[0].time if mkt.h1 else cur
        if current_h1 != self.last_struct_update or self.estructura_timestamp == 0:
            self._actualizar_estructura(mkt)
            self.last_struct_update = current_h1
            self.estructura_timestamp = cur

        current_h4 = mkt.h4[0].time if mkt.h4 else cur
        if self.mss_time != current_h4:
            self.mss_valid = False
        self.zona_valid = False

        ctx = {"vol_ratio": vol_ratio, "session": session, "kill_zone": kill_zone,
               "vol_exp": vol_exp, "vol_comp": vol_comp, "trend_d1": trend_d1, "trend_velas": trend_velas}

        candidatas: list[Signal] = []
        self._motor_d1(mkt, ctx, candidatas)
        self._motor_d2(mkt, ctx, candidatas)
        if self.cal.d2_anticipar:
            self._motor_d2_anticipacion(mkt, ctx, candidatas)
        self._motor_d3(mkt, ctx, candidatas)
        self._motor_d4(mkt, ctx, candidatas)
        self._motor_d5(mkt, ctx, candidatas)

        for sig in candidatas:
            sig.hipotesis_expiry_velas = self._vencimiento(sig)
            sig.hipotesis_expiry_minutos = sig.hipotesis_expiry_velas * 15
            ok, zona = self._zona_premium(mkt, sig.entry_price)
            if ok:
                sig.hipotesis_zona = zona
            self._generar_hipotesis(sig, mkt)
            self._route(mkt, sig, out)
        return out

    def _route(self, mkt: MarketData, sig: Signal, out: EngineOutput):
        for f in ("calidad_sweep", "calidad_mss", "calidad_fvg", "calidad_ob", "salud_tendencial", "contexto_estructural"):
            setattr(sig, f, _clamp(getattr(sig, f)))
        if mkt.spread_points is not None:
            sig.spread_pips = mkt.spread_points
        if sig.atr14 > 0 and sig.spread_pips > sig.atr14 * 3:
            sig.gap_detected = True
        if len(self.pending) >= _MAX_PENDING:
            self.pending.pop(0)
        self.pending.append(sig)
        out.new_signals.append(sig)

    def _measure_returns(self, mkt: MarketData, out: EngineOutput):
        for sig in list(self.pending):
            if sig.completada:
                continue
            shift = sig.entry_bar_shift
            ns = _bar_shift_by_time(mkt.m15, sig.entry_time)
            if ns >= 0:
                shift = sig.entry_bar_shift = ns
            elif shift < 0:
                continue
            if shift <= 0:
                continue
            if shift > 4:
                sig.completada = True
                continue
            idx = shift - 1
            if sig.measured[idx]:
                continue
            close = mkt.m15[shift].close
            if close == 0:
                continue
            mfe = mae = sig.entry_price
            for b in range(shift - 1, -1, -1):   # EXCLUSIVO: omite la vela de entrada
                bar = mkt.m15[b]
                if bar.high == 0 or bar.low == 0:
                    break
                if sig.direction == 1:
                    mfe, mae = max(mfe, bar.high), min(mae, bar.low)
                else:
                    mfe, mae = min(mfe, bar.low), max(mae, bar.high)
            p = self.cal.point
            if sig.direction == 1:
                sig.retorno[idx] = (close - sig.entry_price) / p
                sig.mfe[idx] = (mfe - sig.entry_price) / p
                sig.mae[idx] = (mae - sig.entry_price) / p
            else:
                sig.retorno[idx] = (sig.entry_price - close) / p
                sig.mfe[idx] = (sig.entry_price - mfe) / p
                sig.mae[idx] = (sig.entry_price - mae) / p
            sig.measured[idx] = True
            sig.signal_age_bars = shift
            if idx == 3:
                sig.completada = True
                out.completed.append(sig)
        self.pending = [s for s in self.pending if not s.completada]

    def pending_snapshot(self) -> list[dict]:
        return [s.as_dict() for s in self.pending]
