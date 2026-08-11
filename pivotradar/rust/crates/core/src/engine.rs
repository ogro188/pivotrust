use crate::calibration::Calibration;
use crate::confluence;
use crate::gauges;
use crate::hypothesis;
use crate::indicators::{self, bar_shift_by_time};
use crate::structure::{self, EstructuraRef};
use crate::types::{Bar, Detector, PendingRecord, Signal};

/// Información de símbolo (espejo de _Symbol, _Point, _Digits).
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub symbol: String,
    pub point: f64,
    pub digits: i32,
}

impl SymbolInfo {
    pub fn new(symbol: &str, point: f64, digits: i32) -> Self {
        SymbolInfo { symbol: symbol.to_string(), point, digits }
    }
}

/// Snapshot de mercado que recibe el motor (índice 0 = vela actual en formación).
#[derive(Debug, Clone, Default)]
pub struct MarketData {
    pub m15: Vec<Bar>,
    pub h1: Vec<Bar>,
    pub h4: Vec<Bar>,
    pub d1: Vec<Bar>,
    /// Spread en puntos si la fuente lo provee directamente.
    pub spread_points: Option<f64>,
    pub ask: Option<f64>,
    pub bid: Option<f64>,
}

const MAX_PENDING: usize = 500;

#[derive(Debug, Clone, Default)]
struct DetectorLatch {
    last_bar: i64,
    pattern_key: String,
    fired: bool,
}

/// Salida de una llamada a `process`.
#[derive(Debug, Clone, Default)]
pub struct EngineOutput {
    /// Señales recién creadas en esta pasada.
    pub new_signals: Vec<Signal>,
    /// Señales que acaban de completar su medición (4 velas) con MFE/MAE finales.
    pub completed: Vec<Signal>,
}

/// El motor único: una instancia por activo, el mismo código para todos.
pub struct Engine {
    pub cal: Calibration,
    pub info: SymbolInfo,

    pub(crate) atr14_buffer: Vec<f64>,
    pub(crate) ema21_buffer: Vec<f64>,
    ema50_buffer: Vec<f64>,
    ema50_d1_buffer: Vec<f64>,
    ema200_d1_buffer: Vec<f64>,
    atr_history: [f64; 20],

    pub(crate) g1: f64,
    pub(crate) g2: f64,
    g3: f64,
    pub(crate) g4: f64,
    last_g_calc_bar: i64,

    pub(crate) estructura: EstructuraRef,
    estructura_timestamp: i64,
    last_struct_update: i64,

    mss_valid: bool,
    mss_time: i64,
    mss_bars_ago: i32,
    mss_dir: String,
    mss_level: f64,

    latches: [DetectorLatch; 6],
    pending: std::collections::VecDeque<Signal>,

    atr_history_head: usize,

    vol_cached_time: i64,
    vol_cached_shift: i32,
    vol_cached_n: i32,
    vol_cached_val: f64,
}

impl Engine {
    pub fn new(cal: Calibration, info: SymbolInfo, pending: Vec<PendingRecord>) -> Engine {
        let mut engine = Engine {
            cal,
            info,
            atr14_buffer: Vec::new(),
            ema21_buffer: Vec::new(),
            ema50_buffer: Vec::new(),
            ema50_d1_buffer: Vec::new(),
            ema200_d1_buffer: Vec::new(),
            atr_history: [0.0; 20],
            g1: 50.0,
            g2: 50.0,
            g3: 50.0,
            g4: 0.0,
            last_g_calc_bar: 0,
            estructura: EstructuraRef::new(),
            estructura_timestamp: 0,
            last_struct_update: 0,
            mss_valid: false,
            mss_time: 0,
            mss_bars_ago: 0,
            mss_dir: String::new(),
            mss_level: 0.0,
            latches: std::array::from_fn(|_| DetectorLatch::default()),
            pending: std::collections::VecDeque::new(),
            atr_history_head: 0,
            vol_cached_time: 0,
            vol_cached_shift: -1,
            vol_cached_n: -1,
            vol_cached_val: 1.0,
        };
        for rec in pending {
            engine.pending.push_back(signal_from_pending(&rec));
        }
        engine
    }

    pub fn pending_snapshot(&self) -> Vec<Signal> {
        self.pending.iter().cloned().collect()
    }

    /// Procesa un snapshot de mercado. Equivale a OnTick -> ProcessIntraBar + ruteo.
    pub fn process(&mut self, mkt: &MarketData) -> EngineOutput {
        let mut out = EngineOutput::default();
        if !self.update_indicators(mkt) {
            return out;
        }

        self.measure_returns(mkt, &mut out.completed);
        self.update_atr_history();

        let max_lookback = self
            .cal
            .n_ruptura
            .max(self.cal.sweep_n)
            .max(self.cal.ob_lookback)
            .max(10);
        let vol_ratio = self.volume_ratio_cached(mkt, 0, max_lookback);

        let Some(cur) = mkt.m15.first() else { return out };
        let current_bar = cur.time;
        let (hour, minute) = indicators::local_hour_min(current_bar, self.cal.utc_offset_hours);
        let session = indicators::session(hour);
        let kill_zone = indicators::kill_zone(hour, minute);
        let vol_exp = indicators::vol_expanding(&self.atr_history, self.atr_history_head);
        let vol_comp = indicators::vol_compressing(&self.atr_history, self.atr_history_head);
        let trend_d1 = indicators::trend_d1(&self.ema50_d1_buffer, &self.ema200_d1_buffer);
        let trend_velas = indicators::trend_velas(&self.ema21_buffer, &self.ema50_buffer, 55);

        if current_bar != self.last_g_calc_bar {
            self.g1 = gauges::g1_compresion(self.atr14_buffer[0], &self.atr_history);
            self.g2 = gauges::g2_persistencia(&mkt.m15);
            self.g3 = gauges::g3_eficiencia(&mkt.m15);
            self.g4 = gauges::g4_agotamiento(&mkt.m15, self.atr14_buffer[0]);
            self.last_g_calc_bar = current_bar;
        }

        let current_h1 = mkt.h1.first().map(|b| b.time).unwrap_or(current_bar);
        if current_h1 != self.last_struct_update || self.estructura_timestamp == 0 {
            let price = cur.close;
            self.estructura = structure::actualizar_estructura(
                &mkt.h1,
                price,
                self.atr14_buffer[0],
                self.cal.pivot_depth as usize,
                self.cal.pivot_lookback as usize,
                self.cal.sweep_distancia,
                self.cal.zona_margen,
            );
            self.last_struct_update = current_h1;
            self.estructura_timestamp = current_bar;
        }

        let current_h4 = mkt.h4.first().map(|b| b.time).unwrap_or(current_bar);
        if self.mss_time != current_h4 {
            self.mss_valid = false;
        }

        let ctx = BarCtx {
            vol_ratio,
            session,
            kill_zone,
            vol_exp,
            vol_comp,
            trend_d1,
            trend_velas,
        };

        let mut candidatas: Vec<Signal> = Vec::new();
        self.motor_d1(mkt, &ctx, &mut candidatas);
        self.motor_d2(mkt, &ctx, &mut candidatas);
        if self.cal.d2_anticipar {
            self.motor_d2_anticipacion(mkt, &ctx, &mut candidatas);
        }
        self.motor_d3(mkt, &ctx, &mut candidatas);
        self.motor_d4(mkt, &ctx, &mut candidatas);
        self.motor_d5(mkt, &ctx, &mut candidatas);

        self.resolve_and_route(mkt, candidatas, &mut out.new_signals);
        out
    }

    /// Espejo de ResolverConfluenciasYRutear.
    fn resolve_and_route(&mut self, mkt: &MarketData, candidatas: Vec<Signal>, out: &mut Vec<Signal>) {
        for mut sig in candidatas {
            sig.hipotesis_expiry_velas = hypothesis::calcular_vencimiento(&sig);
            sig.hipotesis_expiry_minutos = sig.hipotesis_expiry_velas * 15;

            let (ok_zona, zona) = structure::zona_premium_discount(&mkt.m15, sig.entry_price);
            if ok_zona {
                sig.hipotesis_zona = zona;
            }

            hypothesis::generar_hipotesis(
                &mut sig,
                &self.cal,
                &self.estructura,
                self.atr14_buffer[0],
                &mkt.m15,
                self.info.point,
                self.info.digits,
            );
            self.route_signal(mkt, sig, out);
        }
    }

    /// Espejo de RouteSignal (sin I/O: la plataforma persiste y notifica).
    fn route_signal(&mut self, mkt: &MarketData, mut sig: Signal, out: &mut Vec<Signal>) {
        sig.calidad_sweep = indicators::clamp_0_100(sig.calidad_sweep);
        sig.calidad_mss = indicators::clamp_0_100(sig.calidad_mss);
        sig.calidad_fvg = indicators::clamp_0_100(sig.calidad_fvg);
        sig.calidad_ob = indicators::clamp_0_100(sig.calidad_ob);
        sig.salud_tendencial = indicators::clamp_0_100(sig.salud_tendencial);
        sig.contexto_estructural = indicators::clamp_0_100(sig.contexto_estructural);

        // NUEVO: poblar spread y gap si el feed lo provee
        if let Some(spread) = mkt.spread_points {
            sig.spread_pips = spread;
        }
        // gap_detected: true si el spread es anómalo (> 3× ATR14 en puntos)
        let atr_pips = sig.atr14; // ya está en puntos
        if sig.spread_pips > atr_pips * 3.0 && atr_pips > 0.0 {
            sig.gap_detected = true;
        }

        if self.pending.len() >= MAX_PENDING {
            self.pending.pop_front();
        }
        self.pending.push_back(sig.clone());
        out.push(sig);
    }

    fn update_indicators(&mut self, mkt: &MarketData) -> bool {
        if mkt.m15.len() < 3 || mkt.d1.len() < 2 {
            return false;
        }
        self.atr14_buffer = indicators::atr_series(&mkt.m15, 14);
        self.ema21_buffer = indicators::ema_close_series(&mkt.m15, 21);
        self.ema50_buffer = indicators::ema_close_series(&mkt.m15, 50);
        self.ema50_d1_buffer = indicators::ema_close_series(&mkt.d1, 50);
        self.ema200_d1_buffer = indicators::ema_close_series(&mkt.d1, 200);
        if self.atr14_buffer.len() < 3
            || self.ema21_buffer.len() < 3
            || self.ema50_buffer.len() < 3
            || self.ema50_d1_buffer.len() < 2
            || self.ema200_d1_buffer.len() < 2
        {
            return false;
        }
        true
    }

    fn update_atr_history(&mut self) {
        self.atr_history_head = (self.atr_history_head + 1) % 20;
        self.atr_history[self.atr_history_head] = self.atr14_buffer[0];
    }

    fn volume_ratio_cached(&mut self, mkt: &MarketData, shift: usize, n: i32) -> f64 {
        let bar_time = mkt.m15.get(shift).map(|b| b.time).unwrap_or(-1);
        let valid = self.vol_cached_time == bar_time
            && self.vol_cached_shift == shift as i32
            && self.vol_cached_n == n;
        if valid {
            return self.vol_cached_val;
        }
        let result = indicators::volume_ratio(&mkt.m15, shift, n as usize);
        self.vol_cached_val = result;
        self.vol_cached_shift = shift as i32;
        self.vol_cached_n = n;
        self.vol_cached_time = bar_time;
        result
    }

    pub(crate) fn is_duplicate(&self, id: u64) -> bool {
        self.pending.iter().any(|s| s.id == id)
    }

    pub(crate) fn latch_fired(&self, det: Detector, direction: i32, current_bar: i64) -> bool {
        let Some(idx) = det.latch_index() else { return false };
        let key = format!("{}|{}", det.as_str(), direction);
        let l = &self.latches[idx];
        if l.last_bar != current_bar {
            return false;
        }
        l.fired && l.pattern_key == key
    }

    pub(crate) fn mark_latch(&mut self, det: Detector, direction: i32, current_bar: i64) {
        let Some(idx) = det.latch_index() else { return };
        let key = format!("{}|{}", det.as_str(), direction);
        self.latches[idx].last_bar = current_bar;
        self.latches[idx].pattern_key = key;
        self.latches[idx].fired = true;
    }

    pub(crate) fn detect_mss_cached(&mut self, mkt: &MarketData) -> Option<(i32, String, f64)> {
        let current_h4 = mkt.h4.first().map(|b| b.time).unwrap_or(0);
        if self.mss_valid && self.mss_time == current_h4 {
            return Some((self.mss_bars_ago, self.mss_dir.clone(), self.mss_level));
        }
        let r = structure::detect_mss_h4(&mkt.h4, self.cal.mss_lookback_h4 as usize);
        match r {
            Some((bars, dir, level)) => {
                self.mss_valid = true;
                self.mss_time = current_h4;
                self.mss_bars_ago = bars;
                self.mss_dir = dir.clone();
                self.mss_level = level;
                Some((bars, dir, level))
            }
            None => {
                self.mss_valid = false;
                None
            }
        }
    }

    /// Espejo de MeasureReturns: mide MFE/MAE/retorno de señales pendientes.
    fn measure_returns(&mut self, mkt: &MarketData, completed: &mut Vec<Signal>) {
        for i in 0..self.pending.len() {
            if self.pending[i].completada {
                continue;
            }
            let entry_time = self.pending[i].entry_time;
            let mut shift = self.pending[i].entry_bar_shift;
            let new_shift = bar_shift_by_time(&mkt.m15, entry_time, 900); // ±15 min para M15
            if new_shift >= 0 {
                shift = new_shift;
                self.pending[i].entry_bar_shift = new_shift;
            } else if shift < 0 {
                continue;
            }

            if shift < 0 || shift == 0 {
                continue;
            }
            if shift > 4 {
                self.pending[i].completada = true;
                continue;
            }

            let idx = (shift - 1) as usize;
            if idx >= 4 || self.pending[i].measured[idx] {
                continue;
            }

            let close = mkt.m15.get(shift as usize).map(|b| b.close).unwrap_or(0.0);
            if close == 0.0 {
                continue;
            }

            let dir = self.pending[i].direction;
            let entry = self.pending[i].entry_price;
            let mut mfe = entry;
            let mut mae = entry;
            // EXCLUSIVO: no incluye la vela de entrada (0..shift). El high/low de la vela
            // de entrada ocurrió ANTES del close que sirvió como entry_price.
            for b in 0..shift {
                let Some(bar) = mkt.m15.get(b as usize) else { break };
                let h = bar.high;
                let l = bar.low;
                if h == 0.0 || l == 0.0 {
                    break;
                }
                if dir == 1 {
                    if h > mfe {
                        mfe = h;
                    }
                    if l < mae {
                        mae = l;
                    }
                } else {
                    if l < mfe {
                        mfe = l;
                    }
                    if h > mae {
                        mae = h;
                    }
                }
            }

            let ret = if dir == 1 {
                (close - entry) / self.info.point
            } else {
                (entry - close) / self.info.point
            };
            let mfe_p = if dir == 1 { (mfe - entry) / self.info.point } else { (entry - mfe) / self.info.point };
            let mae_p = if dir == 1 { (mae - entry) / self.info.point } else { (entry - mae) / self.info.point };

            self.pending[i].retorno[idx] = ret;
            self.pending[i].mfe[idx] = mfe_p;
            self.pending[i].mae[idx] = mae_p;
            self.pending[i].measured[idx] = true;
            self.pending[i].signal_age_bars = shift;

            if idx == 3 {
                self.pending[i].completada = true;
                completed.push(self.pending[i].clone());
            }
        }

        self.pending.retain(|s| !s.completada);
    }
}

fn signal_from_pending(rec: &PendingRecord) -> Signal {
    Signal {
        id: rec.id,
        entry_time: rec.entry_time,
        entry_bar_shift: rec.entry_bar_shift,
        direction: rec.direction,
        entry_price: rec.entry_price,
        detector: rec.detector.clone(),
        tipo: rec.tipo.clone(),
        signal_age_bars: rec.signal_age_bars,
        measured: rec.measured,
        retorno: rec.retorno,
        mfe: rec.mfe,
        mae: rec.mae,
        completada: rec.completada,
        hipotesis_expiry_velas: rec.hipotesis_expiry_velas,
        ..Default::default()
    }
}

/// Contexto por vela ya calculado y compartido entre motores.
pub(crate) struct BarCtx {
    pub vol_ratio: f64,
    pub session: &'static str,
    pub kill_zone: &'static str,
    pub vol_exp: bool,
    pub vol_comp: bool,
    pub trend_d1: &'static str,
    pub trend_velas: i32,
}

impl Engine {
    pub(crate) fn confluence_sweep_fvg(&self, dir: i32, fvg_ahora: bool, fvg_size: f64) -> f64 {
        confluence::confluencia_sweep_fvg(&self.pending, dir, fvg_ahora, fvg_size, self.cal.fvg_min_size_atr)
    }
    pub(crate) fn confluence_completa(&self, dir: i32, fvg_ahora: bool, fvg_size: f64) -> f64 {
        confluence::confluencia_completa(&self.pending, dir, fvg_ahora, fvg_size, self.cal.fvg_min_size_atr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(t: i64, o: f64, h: f64, l: f64, c: f64) -> Bar {
        Bar { time: t, open: o, high: h, low: l, close: c, volume: 1.0 }
    }

    fn base_mkt(bars: Vec<Bar>) -> MarketData {
        MarketData { m15: bars, h1: vec![], h4: vec![], d1: vec![], spread_points: None, ask: None, bid: None }
    }

    /// Corrección 2: MFE/MAE no incluyen la vela de entrada (0..shift exclusivo).
    #[test]
    fn measure_returns_excludes_entry_bar() {
        let cal = Calibration::default();
        let info = SymbolInfo::new("EURUSD", 0.00001, 5);
        let mut engine = Engine::new(cal.clone(), info, vec![]);

        // Señal pendiente: entrada en close de la vela en shift 1 (time=T).
        let mut sig = Signal::default();
        sig.id = 1;
        sig.entry_time = 900;
        sig.entry_bar_shift = 1;
        sig.direction = 1;
        sig.entry_price = 1.0000;
        engine.pending.push_back(sig);

        // Serie: index 0 = actual (T+900), index 1 = entrada (T), index 2 = anterior.
        // La vela de entrada tiene high = entry + 20 pips -> NO debe contarse.
        // La vela 0 tiene high = entry + 10 pips -> MFE esperado = +100 pips.
        let bars = vec![
            bar(1800, 1.0005, 1.0010, 0.9995, 1.0005),
            bar(900, 1.0000, 1.0020, 0.9990, 1.0005),
            bar(0, 0.9995, 0.9995, 0.9985, 0.9990),
        ];
        let mkt = base_mkt(bars);
        let mut completed = Vec::new();
        engine.measure_returns(&mkt, &mut completed);

        let p = &engine.pending[0];
        assert!(p.measured[0], "debe medir shift 0");
        let mfe_pips = p.mfe[0];
        assert!((mfe_pips - 100.0).abs() < 1e-9, "MFE debe ser +100 pips, fue {mfe_pips}");
        let mae_pips = p.mae[0];
        assert!((mae_pips + 50.0).abs() < 1e-9, "MAE debe ser -50 pips, fue {mae_pips}");
    }

    /// Corrección 4: señal huérfana por gap se reencuentra con bar_shift tolerante.
    #[test]
    fn measure_returns_survives_gap() {
        let cal = Calibration::default();
        let info = SymbolInfo::new("EURUSD", 0.00001, 5);
        let mut engine = Engine::new(cal.clone(), info, vec![]);

        let mut sig = Signal::default();
        sig.id = 2;
        sig.entry_time = 900; // la serie NO tiene time=900; la más cercana es 950
        sig.entry_bar_shift = -1;
        sig.direction = 1;
        sig.entry_price = 1.0000;
        engine.pending.push_back(sig);

        let bars = vec![
            bar(1850, 1.0005, 1.0010, 0.9995, 1.0005),
            bar(950, 1.0000, 1.0015, 0.9990, 1.0005),
            bar(50, 0.9995, 0.9995, 0.9985, 0.9990),
        ];
        let mkt = base_mkt(bars);
        let mut completed = Vec::new();
        engine.measure_returns(&mkt, &mut completed);

        assert_eq!(engine.pending[0].entry_bar_shift, 1, "debe resolver el gap a shift 1");
        assert!(engine.pending[0].measured[0]);
    }

    /// Corrección 5: spread_pips y gap_detected se pueblan en route.
    #[test]
    fn route_populates_spread_and_gap() {
        let cal = Calibration::default();
        let info = SymbolInfo::new("EURUSD", 0.00001, 5);
        let mut engine = Engine::new(cal.clone(), info, vec![]);

        let mut sig = Signal::default();
        sig.id = 3;
        sig.atr14 = 10.0; // 10 puntos
        let mut mkt = base_mkt(vec![]);
        mkt.spread_points = Some(40.0); // > 3×10 => gap

        let mut out = Vec::new();
        engine.route_signal(&mkt, sig.clone(), &mut out);
        assert_eq!(engine.pending[0].spread_pips, 40.0);
        assert!(engine.pending[0].gap_detected);
        assert_eq!(out.len(), 1);
    }
}
