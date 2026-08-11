"""Almacenamiento por activo: SQLite (canónico) + CSV espejo (para auditoría/análisis).

Cada activo tiene:
  - {data_dir}/{symbol}.db   -> tabla `signals` (estado actual de cada señal).
  - {data_dir}/{symbol}.csv  -> fila por señal, mismos campos (encabezado + snapshot).

`load_pending()` devuelve las señales no completadas (para restaurar el estado
del motor al arrancar). `upsert()` guarda señales nuevas y completadas.
"""

from __future__ import annotations

import csv
import json
import sqlite3
from pathlib import Path

from .types import Signal

FIELD_NAMES = [f.name for f in Signal.__dataclass_fields__.values()]


class Storage:
    def __init__(self, symbol: str, data_dir: str | Path):
        self.symbol = symbol
        self.data_dir = Path(data_dir)
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.db_path = self.data_dir / f"{symbol}.db"
        self.csv_path = self.data_dir / f"{symbol}.csv"
        self._init_db()
        self._init_csv()

    # ------------------------------------------------------------------ sqlite
    def _init_db(self):
        cols = ", ".join(f'"{c}"' for c in FIELD_NAMES if c != "id")
        with sqlite3.connect(self.db_path) as conn:
            conn.execute(f"CREATE TABLE IF NOT EXISTS signals (id TEXT PRIMARY KEY, {cols})")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_completada ON signals (completada)")

    def upsert(self, sig: Signal):
        d = sig.as_dict()
        fields = [c for c in FIELD_NAMES if c != "id"]
        cols = ", ".join(f'"{c}"' for c in fields)
        ph = ", ".join("?" for _ in fields)
        sql = f"INSERT OR REPLACE INTO signals (id, {cols}) VALUES (?, {ph})"
        row = [str(sig.id)] + [json.dumps(d[c]) if c in ("measured", "retorno", "mfe", "mae") else d[c]
                               for c in fields]
        with sqlite3.connect(self.db_path) as conn:
            conn.execute(sql, row)
        self._append_csv(sig)

    def load_pending(self) -> list[dict]:
        cols = ", ".join(f'"{c}"' for c in FIELD_NAMES if c != "id")
        with sqlite3.connect(self.db_path) as conn:
            conn.row_factory = sqlite3.Row
            rows = conn.execute(f"SELECT id, {cols} FROM signals WHERE completada = 0").fetchall()
        out = []
        for r in rows:
            d = {c: r[c] for c in FIELD_NAMES}
            d["id"] = int(r["id"])
            for c in ("measured", "retorno", "mfe", "mae"):
                v = d.get(c)
                if isinstance(v, str):
                    try:
                        d[c] = json.loads(v)
                    except Exception:
                        d[c] = []
            out.append(d)
        return out

    # ------------------------------------------------------------------ csv espejo
    def _init_csv(self):
        if not self.csv_path.exists():
            with open(self.csv_path, "w", newline="", encoding="utf-8") as f:
                w = csv.DictWriter(f, fieldnames=FIELD_NAMES)
                w.writeheader()

    def _append_csv(self, sig: Signal):
        d = {k: json.dumps(v) if isinstance(v, (list, bool)) else v for k, v in sig.as_dict().items()}
        with open(self.csv_path, "a", newline="", encoding="utf-8") as f:
            w = csv.DictWriter(f, fieldnames=FIELD_NAMES)
            w.writerow(d)
