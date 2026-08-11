use serde::{Deserialize, Serialize};

/// Calibración por activo. Espejo de los `input` + `prob_base` del EA MQL5.
/// Todos los campos tienen default = valores EURUSD (referencia de calibración conservadora).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Calibration {
    pub symbol: String,
    /// Offset en horas del servidor respecto a UTC (para sesiones/kill zones).
    pub utc_offset_hours: i32,
    pub ntfy_topic: String,
    pub ntfy_server: String,

    // --- D1 (Ruptura de rango)
    pub n_ruptura: i32,
    pub d1_atr_threshold: f64,
    pub body_ratio_min: f64,
    pub d1_use_retest: bool,
    pub d1_use_volume: bool,
    pub d1_min_volume: f64,

    // --- D2 (Liquidity Sweep + Reclaim)
    pub sweep_n: i32,
    pub sweep_wick_min: f64,
    pub reclaim_body_min: f64,
    pub equal_hl_window: i32,
    pub equal_hl_tol: f64,
    pub d2_anticipar: bool,

    // --- D3 (Fair Value Gap)
    pub fvg_min_size_atr: f64,
    pub fvg_body_ratio: f64,
    pub fvg_mitig_umbral: f64,

    // --- D4 (Order Block)
    pub ob_lookback: i32,
    pub ob_body_min: f64,
    pub ob_impulse_min: f64,

    // --- D5 (MSS H4 + Sweep)
    pub mss_lookback_h4: i32,
    pub mss_max_age_h4_bars: i32,

    // --- D0 (Estructura)
    pub pivot_depth: i32,
    pub pivot_lookback: i32,
    pub sweep_distancia: f64,
    pub zona_margen: f64,
    pub peso_estructural: f64,

    // --- Probabilidad base por detector (difiere por activo)
    pub prob_d1: i32,
    pub prob_d2: i32,
    pub prob_d3: i32,
    pub prob_d4: i32,
    pub prob_d5: i32,
}

impl Default for Calibration {
    fn default() -> Self {
        Calibration {
            symbol: String::from("EURUSD"),
            utc_offset_hours: 0,
            ntfy_topic: String::new(),
            ntfy_server: String::from("https://ntfy.sh"),
            n_ruptura: 4,
            d1_atr_threshold: 0.5,
            body_ratio_min: 0.4,
            d1_use_retest: true,
            d1_use_volume: true,
            d1_min_volume: 1.2,
            sweep_n: 6,
            sweep_wick_min: 0.55,
            reclaim_body_min: 0.55,
            equal_hl_window: 10,
            equal_hl_tol: 0.15,
            d2_anticipar: true,
            fvg_min_size_atr: 0.2,
            fvg_body_ratio: 0.55,
            fvg_mitig_umbral: 0.5,
            ob_lookback: 12,
            ob_body_min: 0.4,
            ob_impulse_min: 0.7,
            mss_lookback_h4: 20,
            mss_max_age_h4_bars: 12,
            pivot_depth: 2,
            pivot_lookback: 24,
            sweep_distancia: 1.5,
            zona_margen: 0.5,
            peso_estructural: 0.25,
            prob_d1: 65,
            prob_d2: 70,
            prob_d3: 65,
            prob_d4: 65,
            prob_d5: 75,
        }
    }
}

impl Calibration {
    /// Probabilidad base según detector (calibración por activo).
    pub fn prob_base_for(&self, det: &str) -> i32 {
        match det {
            "D1" => self.prob_d1,
            "D2" | "D2_ANTICIPACION" => self.prob_d2,
            "D3" | "D3_DEF" => self.prob_d3,
            "D4" => self.prob_d4,
            "D5" => self.prob_d5,
            _ => 55,
        }
    }
}
