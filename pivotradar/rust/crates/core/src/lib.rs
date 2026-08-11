//! PivotRadar Core — motor de detección único, multiactivo, agnóstico.
//!
//! Port 1:1 del EA MQL5 `PivotRadar_Hybrid_IntraVela_ema50_D5 v7.6` (3266 líneas).
//! Reglas de oro:
//!   - Cero I/O. Entra OHLCV en serie (índice 0 = vela actual en formación), sale `Signal`.
//!   - Determinista: misma entrada => misma salida.
//!   - El símbolo y su calibración viven en `Calibration`; la misma instancia sirve para
//!     N activos (una `Engine` por activo, el código es uno solo).

pub mod calibration;
pub mod classifiers;
pub mod confluence;
pub mod detectors;
pub mod engine;
pub mod gauges;
pub mod hypothesis;
pub mod indicators;
pub mod quality;
pub mod structure;
pub mod types;

pub use calibration::Calibration;
pub use engine::Engine;
pub use engine::{MarketData, SymbolInfo};
pub use types::{Bar, Detector, PendingRecord, Signal};
