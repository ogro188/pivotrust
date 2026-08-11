"""Fábrica de motores: devuelve el motor Rust si está compilado, si no el de referencia.

La plataforma es agnóstica: `build_engine(cal, pending)` es el único punto de
entrada para construir un motor, y el resto del código no conoce la
implementación (cumple el protocolo `PivotEngine` de `engine_iface.py`).
"""

from __future__ import annotations

from .calibration import Calibration
from .reference_engine import ReferenceEngine
from .rust_engine import RUST_AVAILABLE, RustEngine

ENGINE_AVAILABLE = True


def engine_available() -> str:
    """Qué motor está disponible para producción: 'rust' | 'reference'."""
    if RUST_AVAILABLE:
        return "rust"
    return "reference"


def build_engine(cal: Calibration, pending: list[dict] | None = None):
    """Construye la instancia de motor para un activo.

    - Si la calibración pide `rust` y el módulo compilado está presente: motor Rust.
    - Cualquier otro caso: `ReferenceEngine` (base de prueba).
    """
    if getattr(cal, "engine", "rust") == "rust" and RUST_AVAILABLE:
        return RustEngine(cal, pending)
    return ReferenceEngine(cal, pending)
