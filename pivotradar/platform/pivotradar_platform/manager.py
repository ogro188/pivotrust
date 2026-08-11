"""Manager: levanta un EngineHost por activo (multi-thread), gestiona ciclo de vida."""

from __future__ import annotations

import logging
import threading
import time

from .calibration import Calibration, load_calibrations
from .feeds.mt5 import MT5Feed
from .feeds.synthetic import SyntheticFeed
from .host import EngineHost

log = logging.getLogger(__name__)


def make_feed(cal: Calibration, source: str, seed: int | None = None):
    if source == "mt5":
        return MT5Feed(cal.symbol, cal.point, cal.digits)
    return SyntheticFeed(cal.symbol, cal.point, cal.digits, seed=seed)


class Manager:
    def __init__(self, calibrations: dict[str, Calibration], data_dir: str,
                 source: str = "synthetic", poll_seconds: float = 1.0,
                 seed: int | None = None, notify: bool = True):
        self.data_dir = data_dir
        self.source = source
        self.poll_seconds = poll_seconds
        self.hosts: list[EngineHost] = []
        for symbol, cal in calibrations.items():
            try:
                feed = make_feed(cal, source, seed)
                host = EngineHost(cal, feed, data_dir, poll_seconds=poll_seconds, notify=notify)
                self.hosts.append(host)
            except Exception:  # noqa: BLE001
                log.exception("No se pudo inicializar host para %s", symbol)

    def run(self, duration_sec: float | None = None):
        threads = []
        for host in self.hosts:
            t = threading.Thread(target=host.run, daemon=True, name=f"host-{host.cal.symbol}")
            t.start()
            threads.append(t)
        log.info("Manager corriendo %d activos (source=%s)", len(self.hosts), self.source)
        if duration_sec is not None:
            time.sleep(duration_sec)
            self.stop()
        for t in threads:
            t.join()

    def stop(self):
        for host in self.hosts:
            host.stop()


def run_cli(config_path: str | None, data_dir: str, source: str, poll_seconds: float,
            duration: float | None, seed: int | None, notify: bool, symbols: list[str] | None):
    cal_map = load_calibrations(config_path)
    if symbols:
        cal_map = {s: c for s, c in cal_map.items() if s in symbols}
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(name)s: %(message)s")
    m = Manager(cal_map, data_dir, source=source, poll_seconds=poll_seconds, seed=seed, notify=notify)
    m.run(duration_sec=duration)
