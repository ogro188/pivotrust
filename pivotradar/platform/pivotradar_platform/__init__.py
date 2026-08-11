"""Plataforma Python agnóstica para estrategias de detección.

El motor de detección es intercambiable:
  - `pivotradar_engine` (Rust, PyO3): el motor de producción.
  - `ReferenceEngine` (Python puro): referencia / base de prueba, sin Rust.

Cada activo tiene su propia instancia de motor, su CSV, su SQLite y su topic ntfy.
"""

__version__ = "0.1.0"

from .engine_factory import build_engine, engine_available, ENGINE_AVAILABLE  # noqa: F401
from .types import Bar, Signal  # noqa: F401
