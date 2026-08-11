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

    atr14_buffer: Vec<f64>,
    ema21_buffer: Vec<f64>,
    ema50_buffer: Vec<f64>,
    ema50_d1_buffer: Vec<f64>,
    ema200_d1_buffer: Vec<f64>,
    atr_history: [f64; 20],

    g1: f64,
    g2: f64,
    g3: f64,
    g4: f64,
    last_g_calc_bar: i64,

    estructura: EstructuraRef,
    estructura_timestamp: i64,
    last_struct_update: i64,

    mss_valid: bool,
    mss_time: i64,
    mss_bars_ago: i32,
    mss_dir: String,
    mss_level: f64,

    zona_valid: bool,
    zona_time: i64,
    zona_mid: f64,

    latches: [DetectorLatch; 6],
    pending: Vec<Signal>,

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
            zona_valid: false,
            zona_time: 0,
            zona_mid: 0.0,
            latches: std::array::from_fn(|_| DetectorLatch::default()),
            pending: Vec::new(),
            vol_cached_time: 0,
            vol_cached_shift: -1,
            vol_cached_n: -1,
            vol_cached_val: 1.0,
        };
        for rec in pending {
            engine.pending.push(signal_from_pending(&rec));
        }
        engine
    }

    pub fn pending_snapshot(&self) -> Vec<Signal> {
        self.pending.clone()
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
        let vol_exp = indicators::vol_expanding(&self.atr_history);
        let vol_comp = indicators::vol_compressing(&self.atr_history);
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
        self.zona_valid = false;

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
            self.route_signal(sig, out);
        }
    }

    /// Espejo de RouteSignal (sin I/O: la plataforma persiste y notifica).
    fn route_signal(&mut self, mut sig: Signal, out: &mut Vec<Signal>) {
        sig.calidad_sweep = indicators::clamp_0_100(sig.calidad_sweep);
        sig.calidad_mss = indicators::clamp_0_100(sig.calidad_mss);
        sig.calidad_fvg = indicators::clamp_0_100(sig.calidad_fvg);
        sig.calidad_ob = indicators::clamp_0_100(sig.calidad_ob);
        sig.salud_tendencial = indicators::clamp_0_100(sig.salud_tendencial);
        sig.contexto_estructural = indicators::clamp_0_100(sig.contexto_estructural);

        if self.pending.len() >= MAX_PENDING {
            self.pending.remove(0);
        }
        self.pending.push(sig.clone());
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
        for i in (1..20).rev() {
            self.atr_history[i] = self.atr_history[i - 1];
        }
        self.atr_history[0] = self.atr14_buffer[0];
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

    fn is_duplicate(&self, id: u64) -> bool {
        self.pending.iter().any(|s| s.id == id)
    }

    fn latch_fired(&self, det: Detector, direction: i32, current_bar: i64) -> bool {
        let Some(idx) = det.latch_index() else { return false };
        let key = format!("{}|{}", det.as_str(), direction);
        let l = &self.latches[idx];
        if l.last_bar != current_bar {
            return false;
        }
        l.fired && l.pattern_key == key
    }

    fn mark_latch(&mut self, det: Detector, direction: i32, current_bar: i64) {
        let Some(idx) = det.latch_index() else { return };
        let key = format!("{}|{}", det.as_str(), direction);
        self.latches[idx].last_bar = current_bar;
        self.latches[idx].pattern_key = key;
        self.latches[idx].fired = true;
    }

    fn detect_mss_cached(&mut self, mkt: &MarketData) -> Option<(i32, String, f64)> {
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

    fn zona_mid(&mut self, mkt: &MarketData) -> f64 {
        let cur = mkt.m15.first().map(|b| b.time).unwrap_or(0);
        if self.zona_valid && self.zona_time == cur {
            return self.zona_mid;
        }
        let mut max_high = 0.0;
        let mut min_low = 999_999.0;
        for i in 1..=50 {
            if i >= 100 {
                break;
            }
            let Some(b) = mkt.m15.get(i) else { break };
            if b.high == 0.0 || b.low == 0.0 {
                break;
            }
            if b.high > max_high {
                max_high = b.high;
            }
            if b.low < min_low {
                min_low = b.low;
            }
        }
        let mid = if max_high > 0.0 && min_low > 0.0 && max_high > min_low {
            (max_high + min_low) / 2.0
        } else {
            0.0
        };
        self.zona_valid = true;
        self.zona_time = cur;
        self.zona_mid = mid;
        mid
    }

    fn es_zona_premium(&mut self, mkt: &MarketData, nivel: f64) -> (bool, String) {
        let mid = self.zona_mid(mkt);
        if mid > 0.0 {
            let zona = if nivel > mid { "PREMIUM" } else { "DISCOUNT" };
            (true, String::from(zona))
        } else {
            (false, String::from("NEUTRO"))
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
            let new_shift = bar_shift_by_time(&mkt.m15, entry_time);
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
            for b in 0..=shift {
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
