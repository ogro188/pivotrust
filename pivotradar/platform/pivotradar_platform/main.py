"""CLI de la plataforma PivotRadar.

Ejemplos:
  python -m pivotradar_platform          # sintético, sin notificar
  python -m pivotradar_platform --source mt5 --notify
  python -m pivotradar_platform --symbols EURUSD --duration 30 --poll 2
"""

from __future__ import annotations

import argparse
from pathlib import Path

from . import __version__
from .manager import run_cli

DEFAULT_CONFIG = Path(__file__).resolve().parent.parent / "config" / "calibrations.json"


def main(argv=None):
    ap = argparse.ArgumentParser(prog="pivotradar-platform", description="Plataforma agnóstica PivotRadar")
    ap.add_argument("--version", action="version", version=f"%(prog)s {__version__}")
    ap.add_argument("--config", default=str(DEFAULT_CONFIG), help="path a calibrations.json")
    ap.add_argument("--data-dir", default="data", help="directorio de salida (sqlite+csv)")
    ap.add_argument("--source", choices=["synthetic", "mt5"], default="synthetic")
    ap.add_argument("--poll", type=float, default=1.0, help="segundos entre snapshots")
    ap.add_argument("--duration", type=float, default=None, help="segundos de ejecución (None = infinito)")
    ap.add_argument("--seed", type=int, default=None, help="semilla del feed sintético")
    ap.add_argument("--notify", action="store_true", help="enviar notificaciones ntfy")
    ap.add_argument("--symbols", nargs="*", default=None, help="restringir a símbolos")
    args = ap.parse_args(argv)

    run_cli(config_path=args.config, data_dir=args.data_dir, source=args.source,
            poll_seconds=args.poll, duration=args.duration, seed=args.seed,
            notify=args.notify, symbols=args.symbols)


if __name__ == "__main__":
    main()
