use crate::indicators::clamp_0_100;
use crate::types::Signal;

/// HuboSenalRecienteEnDireccion: hay señal del detector en la dirección con entry_bar_shift <= n.
pub fn hubo_senal_reciente(pending: &[Signal], det: &str, dir: i32, n_velas: i32) -> bool {
    pending
        .iter()
        .any(|s| s.detector == det && s.direction == dir && s.entry_bar_shift >= 0 && s.entry_bar_shift <= n_velas)
}

/// CalcularConfluenciaSweepFVG.
pub fn confluencia_sweep_fvg(pending: &[Signal], dir: i32, fvg_ahora: bool, fvg_size: f64, min_size_atr: f64) -> f64 {
    if !hubo_senal_reciente(pending, "D2", dir, 6) {
        return 0.0;
    }
    if !fvg_ahora {
        return 40.0;
    }
    clamp_0_100(60.0 + clamp_0_100((fvg_size - min_size_atr) / 0.60 * 40.0) * 0.4)
}

/// CalcularConfluenciaCompleta.
pub fn confluencia_completa(pending: &[Signal], dir: i32, fvg_ahora: bool, fvg_size: f64, min_size_atr: f64) -> f64 {
    let mut p = 0;
    if hubo_senal_reciente(pending, "D5", dir, 8) {
        p += 1;
    }
    if hubo_senal_reciente(pending, "D2", dir, 8) {
        p += 1;
    }
    if fvg_ahora {
        p += 1;
    }
    match p {
        0 => 0.0,
        1 => 25.0,
        2 => 60.0,
        _ => clamp_0_100(85.0 + clamp_0_100((fvg_size - min_size_atr) / 0.60 * 15.0) * 0.15),
    }
}
