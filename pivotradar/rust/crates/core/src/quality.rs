use crate::indicators::{clamp_0_100, max, min};

/// CalcularCalidadSweep.
/// Umbrales hardcodeados (fieles al EA v7.6):
///   - Peso wick: 40 pts; mínimo 0.55 (sweep_wick_min por defecto), escalado lineal hasta wick=1.0.
///   - Peso reclaim: 35 pts; mínimo 0.55 (reclaim_body_min por defecto).
///   - Frescura: 15 pts lineales hasta bars_ago=6.
///   - Volumen: 10 pts lineales hasta vol=2.0 (saturado).
///   - Bonus equal-HL: +10 pts.
/// Fuente: EA v7.6, inputs SweepWickMin/ReclaimBodyMin; calibrado en EURUSD 2024.
/// No modificar sin re-backtest.
pub fn calidad_sweep(wick: f64, reclaim: f64, vol: f64, bars_ago: i32, equal_hl: bool) -> f64 {
    let mut t = clamp_0_100((wick - 0.55) / 0.45 * 40.0)
        + clamp_0_100((reclaim - 0.55) / 0.45 * 35.0)
        + clamp_0_100((6 - bars_ago) as f64 / 5.0 * 15.0)
        + clamp_0_100(min(vol, 2.0) / 2.0 * 10.0);
    if equal_hl {
        t = min(100.0, t + 10.0);
    }
    clamp_0_100(t)
}

/// CalcularCalidadMSS.
/// Umbrales hardcodeados (fieles al EA v7.6):
///   - wick mínimo 0.55 (40 pts), frescura MSS (30 pts lineales hasta mss_max_age),
///     reclaim mínimo 0.55 (30 pts).
pub fn calidad_mss(wick: f64, reclaim: f64, mss_bars_ago: i32, mss_max_age: i32) -> f64 {
    let denom = max((mss_max_age - 1) as f64, 1.0);
    clamp_0_100(
        clamp_0_100((wick - 0.55) / 0.45 * 40.0)
            + clamp_0_100((mss_max_age - mss_bars_ago) as f64 / denom * 30.0)
            + clamp_0_100((reclaim - 0.55) / 0.45 * 30.0),
    )
}

/// CalcularCalidadFVG.
/// Umbrales hardcodeados (fieles al EA v7.6):
///   - tamaño FVG 45 pts escalado hasta 0.80 ATR (techo fijo; usar min_size_atr del input).
///   - body ratio del impulso 35 pts hasta 1.0.
///   - Bonus defendido: +20 pts.
pub fn calidad_fvg(fvg_size: f64, br_impulso: f64, defendido: bool, min_size_atr: f64, body_ratio: f64) -> f64 {
    let mut t = clamp_0_100((fvg_size - min_size_atr) / (0.80 - min_size_atr) * 45.0)
        + clamp_0_100((br_impulso - body_ratio) / (1.0 - body_ratio) * 35.0);
    if defendido {
        t = min(100.0, t + 20.0);
    }
    clamp_0_100(t)
}

/// CalcularCalidadOB.
/// Umbrales hardcodeados (fieles al EA v7.6):
///   - impulso 50 pts escalado hasta 2.5 ATR (techo fijo; usar ob_impulse_min del input).
///   - frescura OB 30 pts lineales hasta ob_lookback.
///   - volumen 20 pts lineales hasta vol=2.0 (saturado).
pub fn calidad_ob(impulso: f64, ob_bars: i32, vol: f64, ob_impulse_min: f64, ob_lookback: i32) -> f64 {
    let denom = max((ob_lookback - 1) as f64, 1.0);
    clamp_0_100(
        clamp_0_100((impulso - ob_impulse_min) / (2.5 - ob_impulse_min) * 50.0)
            + clamp_0_100((ob_lookback - ob_bars) as f64 / denom * 30.0)
            + clamp_0_100(min(vol, 2.0) / 2.0 * 20.0),
    )
}

/// CalcularSaludTendencial.
/// Umbrales hardcodeados (fieles al EA v7.6):
///   - alineación de EMAs 40 pts hasta 15 velas, pendiente 35 pts hasta ±1 ATR,
///     tendencia D1 a favor 25 pts.
pub fn salud_tendencial(trend: i32, slope: f64, trend_d1: &str, dir: i32) -> f64 {
    let mut p3 = 0.0;
    if (dir == 1 && trend_d1 == "ALCISTA") || (dir == -1 && trend_d1 == "BAJISTA") {
        p3 = 25.0;
    }
    clamp_0_100(
        clamp_0_100(min(trend as f64, 15.0) / 15.0 * 40.0)
            + clamp_0_100(min(slope.abs(), 1.0) * 35.0)
            + p3,
    )
}
