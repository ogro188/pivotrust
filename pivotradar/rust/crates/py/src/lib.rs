//! Bindings PyO3: `pivotradar_engine` — el motor Rust expuesto a la plataforma Python.
//!
//! API:
//!   from pivotradar_engine import Engine
//!   eng = Engine(calibration_json, symbol, point, digits, pending_json)
//!   new_signals, completed = eng.process(m15, h1, h4, d1, spread_points, ask, bid)
//!   eng.pending_snapshot() -> json string

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::IntoPyObjectExt;
use pivotradar_core::{Bar, Engine as CoreEngine, PendingRecord, SymbolInfo};
use serde_json::Value;

#[pyclass(name = "Signal", module = "pivotradar_engine")]
struct Signal {
    inner: pivotradar_core::Signal,
}

#[pymethods]
impl Signal {
    /// Representación completa como dict Python (todos los campos del motor).
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let v = serde_json::to_value(&self.inner).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        json_to_py(py, &v)
    }

    fn __repr__(&self) -> String {
        format!(
            "Signal(id={}, symbol={}, detector={}, dir={}, price={})",
            self.inner.id, self.inner.symbol, self.inner.detector, self.inner.direction, self.inner.entry_price
        )
    }
}

fn json_to_py(py: Python<'_>, v: &Value) -> PyResult<Py<PyAny>> {
    Ok(match v {
        Value::Null => py.None(),
        Value::Bool(b) => {
            let bval = *b;
            bval.into_py_any(py)?
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_pyobject(py)?.into_any().unbind()
            } else if let Some(u) = n.as_u64() {
                u.into_pyobject(py)?.into_any().unbind()
            } else {
                n.as_f64()
                    .unwrap_or(0.0)
                    .into_pyobject(py)?
                    .into_any()
                    .unbind()
            }
        }
        Value::String(s) => s.clone().into_pyobject(py)?.into_any().unbind(),
        Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_py(py, item)?)?;
            }
            list.into_any().unbind()
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, val) in map {
                dict.set_item(k, json_to_py(py, val)?)?;
            }
            dict.into_any().unbind()
        }
    })
}

fn bars_from_tuples(t: Vec<(i64, f64, f64, f64, f64, f64)>) -> Vec<Bar> {
    t.into_iter()
        .map(|(time, open, high, low, close, volume)| Bar {
            time,
            open,
            high,
            low,
            close,
            volume,
        })
        .collect()
}

#[pyclass(name = "Engine", module = "pivotradar_engine")]
struct PyEngine {
    inner: CoreEngine,
}

#[pymethods]
impl PyEngine {
    /// calibración y pendientes se pasan como JSON (dict / lista).
    #[new]
    fn new(
        calibration_json: String,
        symbol: String,
        point: f64,
        digits: i32,
        pending_json: String,
    ) -> PyResult<Self> {
        let cal: pivotradar_core::Calibration =
            serde_json::from_str(&calibration_json).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let pending: Vec<PendingRecord> = if pending_json.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&pending_json).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
        };
        let info = SymbolInfo::new(&symbol, point, digits);
        Ok(PyEngine {
            inner: CoreEngine::new(cal, info, pending),
        })
    }

    /// Procesa un snapshot. Velas como listas de tuplas (time, open, high, low, close, volume).
    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        m15: Vec<(i64, f64, f64, f64, f64, f64)>,
        h1: Vec<(i64, f64, f64, f64, f64, f64)>,
        h4: Vec<(i64, f64, f64, f64, f64, f64)>,
        d1: Vec<(i64, f64, f64, f64, f64, f64)>,
        spread_points: Option<f64>,
        ask: Option<f64>,
        bid: Option<f64>,
    ) -> (Vec<Signal>, Vec<Signal>) {
        let md = pivotradar_core::MarketData {
            m15: bars_from_tuples(m15),
            h1: bars_from_tuples(h1),
            h4: bars_from_tuples(h4),
            d1: bars_from_tuples(d1),
            spread_points,
            ask,
            bid,
        };
        let out = self.inner.process(&md);
        let to_sig = |s: pivotradar_core::Signal| Signal { inner: s };
        (out.new_signals.into_iter().map(to_sig).collect(), out.completed.into_iter().map(to_sig).collect())
    }

    /// Snapshot JSON de señales pendientes (para persistir y restaurar en reinicios).
    fn pending_snapshot(&self) -> String {
        serde_json::to_string(&self.inner.pending_snapshot()).unwrap_or_default()
    }
}

#[pymodule]
fn pivotradar_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEngine>()?;
    m.add_class::<Signal>()?;
    m.add("__version__", "0.1.0")?;
    Ok(())
}
