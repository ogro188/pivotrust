# PivotRadar Platform

Plataforma Python **agnóstica** para ejecutar y probar estrategias de detección
de señales. El motor de detección es intercambiable:

| Motor              | Implementación | Uso                          |
|--------------------|----------------|------------------------------|
| `pivotradar_engine` | Rust (PyO3)   | Producción (compilar con maturin) |
| `ReferenceEngine`  | Python puro    | Base de prueba / validación del port |

Cada activo tiene su **propia** instancia de motor, su **SQLite + CSV**, su
**topic ntfy**, y su **calibración** en `config/calibrations.json`.

## Arquitectura

```
feed (synthetic | mt5)  ──►  EngineHost  ──►  Motor (Rust o Python)
                                    │
                                    ├─► Storage (SQLite + CSV por activo)
                                    └─► Notifier (ntfy por topic)
```

La interfaz del motor está en `engine_iface.py` (`PivotEngine` protocol):
cualquier motor que implemente `process(mkt) -> EngineOutput` y
`pending_snapshot() -> list[dict]` puede enchufarse sin tocar la plataforma.

## Instalación

```bash
cd pivotradar/platform
pip install -e .

# (opcional) feed MetaTrader 5
pip install -e ".[mt5]"
```

## Uso

```bash
# Feed sintético (base de prueba de estrategias), sin notificar
python -m pivotradar_platform

# Con MT5 y notificaciones ntfy
python -m pivotradar_platform --source mt5 --notify

# Solo EURUSD, 60 segundos, snapshot cada 2s
python -m pivotradar_platform --symbols EURUSD --duration 60 --poll 2
```

Datos de salida en `data/`:
- `data/<SYMBOL>.db`  — SQLite canónico (`signals`).
- `data/<SYMBOL>.csv` — espejo para auditoría / análisis.

## Compilar el motor Rust

```bash
cd pivotradar/rust
maturin develop --release -m crates/py/Cargo.toml
```

Si `pivotradar_engine` está disponible, `build_engine` usará el motor Rust;
si no, cae al `ReferenceEngine` automáticamente.

## Escenarios (base de prueba de estrategias)

El feed sintético es determinista con `--seed` y genera OHLCV M15/H1/H4/D1
consistentes (las series superiores se agregan desde M15). Sirve para:
- validar la plataforma sin conexión ni broker;
- hacer backtest de una calibración cambiando parámetros y seed;
- comparar el motor Rust vs `ReferenceEngine` con la misma entrada.

## Tests

```bash
cd pivotradar/platform
python -m pytest tests/ -x -q
```
