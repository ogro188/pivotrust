"""EngineHost: orquesta feed -> motor -> storage -> notificador para UN activo."""

from __future__ import annotations

import logging
import threading
import time

from .calibration import Calibration
from .engine_factory import build_engine
from .feeds.base import Feed
from .notifier import Notifier
from .storage import Storage

log = logging.getLogger(__name__)


class EngineHost:
    def __init__(self, cal: Calibration, feed: Feed, data_dir: str,
                 poll_seconds: float = 1.0, notify: bool = True):
        self.cal = cal
        self.feed = feed
        self.storage = Storage(cal.symbol, data_dir)
        self.notifier = Notifier(cal.ntfy_server, cal.ntfy_topic, enabled=notify)
        self.poll_seconds = poll_seconds
        self.pending = self.storage.load_pending()
        self.engine = build_engine(cal, self.pending)
        self._stop = threading.Event()

    def step_once(self):
        """Procesa un snapshot y persiste/notifica el resultado."""
        mkt = self.feed.snapshot()
        out = self.engine.process(mkt)
        for sig in out.new_signals:
            self.storage.upsert(sig)
            self.notifier.signal_new(sig)
            log.info("[%s] NUEVA %s %s tipo=%s en %s",
                     self.cal.symbol, sig.detector, "COMPRA" if sig.direction == 1 else "VENTA",
                     sig.tipo, sig.entry_price)
        for sig in out.completed:
            self.storage.upsert(sig)
            self.notifier.signal_completed(sig)
            log.info("[%s] COMPLETADA %s ret=%s pips", self.cal.symbol, sig.detector,
                     round(sig.retorno[-1], 1) if sig.retorno else 0.0)

    def run(self):
        while not self._stop.is_set():
            try:
                self.step_once()
            except Exception:  # noqa: BLE001
                log.exception("[%s] error en step_once", self.cal.symbol)
            self._stop.wait(self.poll_seconds)

    def stop(self):
        self._stop.set()
