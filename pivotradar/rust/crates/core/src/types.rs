use serde::{Deserialize, Serialize};

/// Vela OHLCV en orden de serie: índice 0 = vela actual (en formación), como en MQL5.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bar {
    /// Segundos Unix de la apertura de la vela (hora del servidor).
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    /// Volumen de ticks (iVolume).
    pub volume: f64,
}

/// Detectores (D1..D5, D2_ANTICIPACION y D3_DEF).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Detector {
    D1,
    D2,
    D2Anticipacion,
    D3,
    D3Def,
    D4,
    D5,
}

impl Detector {
    pub fn as_str(&self) -> &'static str {
        match self {
            Detector::D1 => "D1",
            Detector::D2 => "D2",
            Detector::D2Anticipacion => "D2_ANTICIPACION",
            Detector::D3 => "D3",
            Detector::D3Def => "D3_DEF",
            Detector::D4 => "D4",
            Detector::D5 => "D5",
        }
    }

    pub fn from_str(s: &str) -> Option<Detector> {
        match s {
            "D1" => Some(Detector::D1),
            "D2" => Some(Detector::D2),
            "D2_ANTICIPACION" => Some(Detector::D2Anticipacion),
            "D3" => Some(Detector::D3),
            "D3_DEF" => Some(Detector::D3Def),
            "D4" => Some(Detector::D4),
            "D5" => Some(Detector::D5),
            _ => None,
        }
    }

    /// Índice del latch interno (0..6), espejo del EA.
    pub fn latch_index(&self) -> Option<usize> {
        match self {
            Detector::D1 => Some(0),
            Detector::D2 => Some(1),
            Detector::D3 => Some(2),
            Detector::D3Def => Some(3),
            Detector::D4 => Some(4),
            Detector::D5 => Some(5),
            Detector::D2Anticipacion => None,
        }
    }
}

/// Registro mínimo para persistir señales pendientes entre reinicios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRecord {
    pub id: u64,
    pub entry_time: i64,
    pub direction: i32,
    pub entry_price: f64,
    pub detector: String,
    pub tipo: String,
    pub signal_age_bars: i32,
    pub completada: bool,
    pub hipotesis_expiry_velas: i32,
    pub entry_bar_shift: i32,
    pub measured: [bool; 4],
    pub retorno: [f64; 4],
    pub mfe: [f64; 4],
    pub mae: [f64; 4],
}

impl PendingRecord {
    pub fn from_signal(s: &Signal) -> PendingRecord {
        PendingRecord {
            id: s.id,
            entry_time: s.entry_time,
            direction: s.direction,
            entry_price: s.entry_price,
            detector: s.detector.clone(),
            tipo: s.tipo.clone(),
            signal_age_bars: s.signal_age_bars,
            completada: s.completada,
            hipotesis_expiry_velas: s.hipotesis_expiry_velas,
            entry_bar_shift: s.entry_bar_shift,
            measured: s.measured,
            retorno: s.retorno,
            mfe: s.mfe,
            mae: s.mae,
        }
    }
}

/// Señal completa, espejo del `struct Signal` del EA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub id: u64,
    pub entry_time: i64,
    pub entry_bar_shift: i32,
    pub symbol: String,
    pub direction: i32,
    pub entry_price: f64,
    pub detector: String,
    pub tipo: String,

    pub cr: f64,
    pub bs: f64,
    pub bs_pips: f64,
    pub br: f64,
    pub range_break_pips: f64,
    pub nivel_estructural: f64,

    pub sweep_wick_ratio: f64,
    pub sweep_volume_ratio: f64,
    pub reclaim_body_ratio: f64,
    pub sweep_bars_ago: i32,
    pub equal_hl_detected: bool,
    pub level_swept: f64,

    pub fvg_size_pips: f64,
    pub fvg_size_atr: f64,
    pub fvg_mitigated: bool,
    pub fvg_top: f64,
    pub fvg_bottom: f64,

    pub ob_high: f64,
    pub ob_low: f64,
    pub ob_bars_ago: i32,
    pub ob_impulse_atr: f64,
    pub ob_confluence: bool,

    pub mss_aligned: bool,
    pub mss_bars_ago_h4: i32,
    pub mss_direction: String,
    pub mss_level: f64,

    pub atr14: f64,
    pub spread_pips: f64,
    pub volume_ratio: f64,
    pub session: String,
    pub kill_zone: String,
    pub trend_d1: String,
    pub vol_expanding: bool,
    pub vol_compressing: bool,

    pub calidad_sweep: f64,
    pub calidad_mss: f64,
    pub calidad_fvg: f64,
    pub calidad_ob: f64,
    pub salud_tendencial: f64,

    pub g1_compresion: f64,
    pub g2_persistencia: f64,
    pub g3_eficiencia: f64,
    pub g4_agotamiento: f64,

    pub conf_sweep_fvg: f64,
    pub conf_completa: f64,

    pub contexto_estructural: f64,
    pub estructura_direccion: String,
    pub distancia_al_sweep: f64,
    pub en_zona_estructural: bool,

    pub hipotesis_causa: String,
    pub hipotesis_efecto: String,
    pub hipotesis_razon: String,
    pub hipotesis_invalidez: String,
    pub hipotesis_expiry_velas: i32,
    pub hipotesis_expiry_minutos: i32,
    pub hipotesis_prob_min: i32,
    pub hipotesis_prob_max: i32,
    pub hipotesis_zona: String,
    pub hipotesis_objetivo: f64,
    pub hipotesis_texto: String,

    pub signal_age_bars: i32,
    pub measured: [bool; 4],
    pub retorno: [f64; 4],
    pub mfe: [f64; 4],
    pub mae: [f64; 4],
    pub gap_detected: bool,
    pub completada: bool,
}

impl Default for Signal {
    fn default() -> Self {
        Signal {
            id: 0,
            entry_time: 0,
            entry_bar_shift: -1,
            symbol: String::new(),
            direction: 0,
            entry_price: 0.0,
            detector: String::new(),
            tipo: String::new(),
            cr: 0.0,
            bs: 0.0,
            bs_pips: 0.0,
            br: 0.0,
            range_break_pips: 0.0,
            nivel_estructural: 0.0,
            sweep_wick_ratio: 0.0,
            sweep_volume_ratio: 0.0,
            reclaim_body_ratio: 0.0,
            sweep_bars_ago: 0,
            equal_hl_detected: false,
            level_swept: 0.0,
            fvg_size_pips: 0.0,
            fvg_size_atr: 0.0,
            fvg_mitigated: false,
            fvg_top: 0.0,
            fvg_bottom: 0.0,
            ob_high: 0.0,
            ob_low: 0.0,
            ob_bars_ago: 0,
            ob_impulse_atr: 0.0,
            ob_confluence: false,
            mss_aligned: false,
            mss_bars_ago_h4: 0,
            mss_direction: String::new(),
            mss_level: 0.0,
            atr14: 0.0,
            spread_pips: 0.0,
            volume_ratio: 0.0,
            session: String::new(),
            kill_zone: String::new(),
            trend_d1: String::new(),
            vol_expanding: false,
            vol_compressing: false,
            calidad_sweep: 0.0,
            calidad_mss: 0.0,
            calidad_fvg: 0.0,
            calidad_ob: 0.0,
            salud_tendencial: 0.0,
            g1_compresion: 0.0,
            g2_persistencia: 0.0,
            g3_eficiencia: 0.0,
            g4_agotamiento: 0.0,
            conf_sweep_fvg: 0.0,
            conf_completa: 0.0,
            contexto_estructural: 0.0,
            estructura_direccion: String::from("NEUTRO"),
            distancia_al_sweep: 0.0,
            en_zona_estructural: false,
            hipotesis_causa: String::new(),
            hipotesis_efecto: String::new(),
            hipotesis_razon: String::new(),
            hipotesis_invalidez: String::new(),
            hipotesis_expiry_velas: 0,
            hipotesis_expiry_minutos: 0,
            hipotesis_prob_min: 0,
            hipotesis_prob_max: 0,
            hipotesis_zona: String::from("NEUTRO"),
            hipotesis_objetivo: 0.0,
            hipotesis_texto: String::new(),
            signal_age_bars: 0,
            measured: [false; 4],
            retorno: [0.0; 4],
            mfe: [0.0; 4],
            mae: [0.0; 4],
            gap_detected: false,
            completada: false,
        }
    }
}
