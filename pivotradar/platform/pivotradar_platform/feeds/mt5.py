"""Feed de MetaTrader 5 (producción).

Lee tasas de los 4 timeframes y datos de tick (ask/bid) con la librería
`MetaTrader5`. El índice 0 de cada serie es la vela en formación (MT5 ya las
devuelve en ese orden: la última fila es la vela actual).
"""

from __future__ import annotations

import logging

from ..engine_iface import MarketData
from ..types import Bar

log = logging.getLogger(__name__)


class MT5Feed:
    def __init__(self, symbol: str, point: float, digits: int,
                 bars: int = 500, utc_offset_hours: int = 0):
        import MetaTrader5 as mt5  # import tardío: dependencia opcional

        self.mt5 = mt5
        self.symbol = symbol
        self.point = point
        self.digits = digits
        self.bars = bars

    def _copy(self, tf, n):
        import MetaTrader5 as mt5

        rates = mt5.copy_rates_from_pos(self.symbol, tf, 0, n)
        if rates is None:
            log.warning("copy_rates_from_pos %s TF%s -> None", self.symbol, tf)
            return []
        bars = []
        for r in rates:
            bars.append(Bar(time=int(r["time"]), open=float(r["open"]), high=float(r["high"]),
                            low=float(r["low"]), close=float(r["close"]), volume=float(r["tick_volume"])))
        # MT5 devuelve cronológico ascendente; el índice 0 debe ser la vela actual
        bars.reverse()
        return bars

    def snapshot(self) -> MarketData:
        import MetaTrader5 as mt5

        TIMEFRAMES = {15: mt5.TIMEFRAME_M15, 60: mt5.TIMEFRAME_H1,
                      240: mt5.TIMEFRAME_H4, 1440: mt5.TIMEFRAME_D1}
        m15 = self._copy(TIMEFRAMES[15], self.bars)
        h1 = self._copy(TIMEFRAMES[60], self.bars)
        h4 = self._copy(TIMEFRAMES[240], self.bars)
        d1 = self._copy(TIMEFRAMES[1440], self.bars)

        ask = bid = spread = None
        tick = mt5.symbol_info_tick(self.symbol)
        if tick is not None:
            ask, bid = float(tick.ask), float(tick.bid)
            if self.point and ask and bid:
                spread = abs(ask - bid) / self.point
        return MarketData(m15=m15, h1=h1, h4=h4, d1=d1, spread_points=spread, ask=ask, bid=bid)

    def history(self) -> list[MarketData]:
        return [self.snapshot()]
