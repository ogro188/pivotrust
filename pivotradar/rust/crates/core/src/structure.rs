use crate::indicators::clamp_0_100;
use crate::types::Bar;

/// Estructura D0 (espejo de `EstructuraRef`).
#[derive(Debug, Clone, Default)]
pub struct EstructuraRef {
    pub swing_high: f64,
    pub swing_low: f64,
    pub swing_high_ant: f64,
    pub swing_low_ant: f64,
    pub sweep_nivel: f64,
    pub sweep_dir: i32,
    pub zona_alta: f64,
    pub zona_baja: f64,
    pub en_zona: bool,
    pub dir_estructura: String,
    pub valida: bool,
}

impl EstructuraRef {
    pub fn new() -> Self {
        EstructuraRef { dir_estructura: String::from("NEUTRO"), ..Default::default() }
    }
}

/// DetectarPivotsH1: swings de profundidad `depth` en los últimos `lookback` H1.
pub fn detectar_pivots_h1(bars: &[Bar], depth: usize, lookback: usize) -> (f64, f64, f64, f64) {
    let mut swing_high = 0.0;
    let mut swing_low = 0.0;
    let mut swing_high_ant = 0.0;
    let mut swing_low_ant = 0.0;

    let max_pivots = 50usize;
    let mut highs: Vec<f64> = Vec::with_capacity(max_pivots);
    let mut lows: Vec<f64> = Vec::with_capacity(max_pivots);

    if lookback > depth + 1 {
        let start = depth + 1;
        let end = (lookback - depth - 1).min(bars.len());
        for i in start..end {
            if i >= max_pivots {
                break;
            }
            let hi = match bars.get(i) {
                Some(b) => b.high,
                None => break,
            };
            if hi == 0.0 {
                continue;
            }
            let mut is_swing = true;
            for j in 1..=depth {
                let left = bars.get(i.wrapping_sub(j)).map(|b| b.high).unwrap_or(f64::NEG_INFINITY);
                let right = bars.get(i + j).map(|b| b.high).unwrap_or(f64::NEG_INFINITY);
                if left >= hi || right >= hi {
                    is_swing = false;
                    break;
                }
            }
            if is_swing && highs.len() < max_pivots {
                highs.push(hi);
            }
        }

        for i in start..end {
            if i >= max_pivots {
                break;
            }
            let lo = match bars.get(i) {
                Some(b) => b.low,
                None => break,
            };
            if lo == 0.0 {
                continue;
            }
            let mut is_swing = true;
            for j in 1..=depth {
                let left = bars.get(i.wrapping_sub(j)).map(|b| b.low).unwrap_or(f64::INFINITY);
                let right = bars.get(i + j).map(|b| b.low).unwrap_or(f64::INFINITY);
                if left <= lo || right <= lo {
                    is_swing = false;
                    break;
                }
            }
            if is_swing && lows.len() < max_pivots {
                lows.push(lo);
            }
        }
    }

    if !highs.is_empty() {
        let max_high = highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        swing_high = max_high;
        let mut second = 0.0;
        for &h in &highs {
            if h < max_high && h > second {
                second = h;
            }
        }
        swing_high_ant = second;
    }
    if !lows.is_empty() {
        let min_low = lows.iter().cloned().fold(f64::INFINITY, f64::min);
        swing_low = min_low;
        let mut second = 999_999.0;
        for &l in &lows {
            if l > min_low && l < second {
                second = l;
            }
        }
        swing_low_ant = if second < 999_999.0 { second } else { 0.0 };
    }

    (swing_high, swing_low, swing_high_ant, swing_low_ant)
}

/// IdentificarSweepMaestro: si el precio está cerca de un swing → nivel de barrido.
pub fn identificar_sweep_maestro(
    price: f64,
    atr14: f64,
    swing_high: f64,
    swing_low: f64,
    sweep_distancia: f64,
) -> (f64, i32) {
    let umbral = atr14 * sweep_distancia;
    if swing_high > 0.0 && (price - swing_high).abs() < umbral {
        return (swing_high, -1);
    }
    if swing_low > 0.0 && (price - swing_low).abs() < umbral {
        return (swing_low, 1);
    }
    (0.0, 0)
}

/// DefinirZonaDeInteres: franja alrededor de los swings ± margen.
pub fn definir_zona_de_interes(
    price: f64,
    atr14: f64,
    swing_high: f64,
    swing_low: f64,
    zona_margen: f64,
) -> (f64, f64, bool) {
    let margen = atr14 * zona_margen;
    let (zona_alta, zona_baja) = if swing_high > 0.0 && swing_low > 0.0 {
        (swing_high.max(swing_low) + margen, swing_high.min(swing_low) - margen)
    } else if swing_high > 0.0 {
        (swing_high + margen, swing_high - margen)
    } else if swing_low > 0.0 {
        (swing_low + margen, swing_low - margen)
    } else {
        (price + margen, price - margen)
    };
    let en_zona = price >= zona_baja && price <= zona_alta;
    (zona_alta, zona_baja, en_zona)
}

/// DeterminarDireccionEstructural: HH/HL => ALCISTA, LL/LH => BAJISTA.
pub fn direccion_estructural(est: &EstructuraRef) -> String {
    if est.swing_high > 0.0
        && est.swing_high_ant > 0.0
        && est.swing_low > 0.0
        && est.swing_low_ant > 0.0
    {
        let hh = est.swing_high > est.swing_high_ant;
        let hl = est.swing_low > est.swing_low_ant;
        if hh && hl {
            return String::from("ALCISTA");
        }
        if !hh && !hl {
            return String::from("BAJISTA");
        }
    }
    String::from("NEUTRO")
}

/// ActualizarEstructura completa (espejo de la función del EA).
pub fn actualizar_estructura(
    h1: &[Bar],
    price: f64,
    atr14: f64,
    depth: usize,
    lookback: usize,
    sweep_distancia: f64,
    zona_margen: f64,
) -> EstructuraRef {
    let mut est = EstructuraRef::new();
    let (sh, sl, sha, sla) = detectar_pivots_h1(h1, depth, lookback);
    est.swing_high = sh;
    est.swing_low = sl;
    est.swing_high_ant = sha;
    est.swing_low_ant = sla;
    let (nivel, dir) = identificar_sweep_maestro(price, atr14, sh, sl, sweep_distancia);
    est.sweep_nivel = nivel;
    est.sweep_dir = dir;
    let (za, zb, en_zona) = definir_zona_de_interes(price, atr14, sh, sl, zona_margen);
    est.zona_alta = za;
    est.zona_baja = zb;
    est.en_zona = en_zona;
    est.dir_estructura = direccion_estructural(&est);
    est.valida = sh > 0.0 || sl > 0.0 || nivel > 0.0;
    est
}

/// DetectMSS_H4: busca la vela H4 más reciente que cierra por encima del máximo previo
/// (MSS alcista) o por debajo del mínimo previo (MSS bajista).
/// Devuelve (bars_ago, "ALCISTA"/"BAJISTA", nivel).
pub fn detect_mss_h4(bars: &[Bar], lookback: usize) -> Option<(i32, String, f64)> {
    let lookback = lookback.min(50);
    for i in 1..=lookback.min(49) {
        let Some(b) = bars.get(i) else { continue; };
        let close_i = b.close;
        if close_i == 0.0 {
            continue;
        }
        // Búsqueda de máximo/mínimo previo excluyendo la vela i (MSS) e incluyendo desde i+1 en adelante.
        // Fiel al EA v7.6 (MotorD5_MSS_Sweep). No modificar sin validar contra MQL5.
        let Some(b_next) = bars.get(i + 1) else { continue; };
        let mut prior_high = b_next.high;
        let mut prior_low = b_next.low;
        let k_end = (i + lookback).min(50);
        for k in (i + 1)..=k_end {
            let Some(bk) = bars.get(k) else { break; };
            let hk = bk.high;
            let lk = bk.low;
            if hk == 0.0 || lk == 0.0 {
                break;
            }
            if hk > prior_high {
                prior_high = hk;
            }
            if lk < prior_low {
                prior_low = lk;
            }
        }
        if close_i > prior_high {
            return Some((i as i32, String::from("ALCISTA"), prior_high));
        }
        if close_i < prior_low {
            return Some((i as i32, String::from("BAJISTA"), prior_low));
        }
    }
    None
}

/// EsZonaPremiumDiscount: nivel por encima/dentro de la mitad del rango M15 (50 velas).
pub fn zona_premium_discount(m15: &[Bar], nivel: f64) -> (bool, String) {
    let mut max_high = 0.0;
    let mut min_low = 999_999.0;
    for i in 1..=50 {
        let Some(b) = m15.get(i) else { break };
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
    if max_high > 0.0 && min_low > 0.0 && max_high > min_low {
        let mid = (max_high + min_low) / 2.0;
        let zona = if nivel > mid { "PREMIUM" } else { "DISCOUNT" };
        return (true, String::from(zona));
    }
    (false, String::from("NEUTRO"))
}

/// EvaluarContextoEstructural: puntaje 0-100 y distancia al sweep maestro.
pub fn evaluar_contexto_estructural(
    est: &EstructuraRef,
    direction: i32,
    nivel: f64,
    atr14: f64,
    point: f64,
) -> (f64, f64) {
    let mut score = 0.0;
    if !est.valida || est.sweep_nivel == 0.0 {
        return (50.0, 0.0);
    }
    if atr14 <= 0.0 {
        return (50.0, 0.0);
    }
    let tolerancia = atr14 * 0.5;
    let distancia = (nivel - est.sweep_nivel).abs() / point;

    if distancia <= tolerancia / point {
        score += 50.0;
    } else if distancia <= tolerancia * 2.0 / point {
        score += 30.0;
    } else {
        score += 10.0;
    }
    if est.en_zona {
        score += 25.0;
    }
    if est.dir_estructura != "NEUTRO" {
        if (direction == 1 && est.dir_estructura == "ALCISTA")
            || (direction == -1 && est.dir_estructura == "BAJISTA")
        {
            score += 25.0;
        } else {
            score += 5.0;
        }
    } else {
        score += 10.0;
    }
    (clamp_0_100(score), distancia)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(t: i64, h: f64, l: f64, c: f64) -> Bar {
        Bar { time: t, open: c, high: h, low: l, close: c, volume: 1.0 }
    }

    /// Corrección 1: con H4 más corto que lookback no debe abortar por `?`.
    #[test]
    fn detect_mss_short_series_no_abort() {
        // 5 velas, lookback 20. Antes: bars.get(i)? abortaba -> None inmediato.
        let bars = vec![
            bar(5, 100.0, 90.0, 95.0),
            bar(4, 105.0, 92.0, 104.0), // i=1: close 104 > prior_high(99) -> MSS ALCISTA
            bar(3, 99.0, 90.0, 95.0),
            bar(2, 98.0, 88.0, 90.0),
            bar(1, 96.0, 86.0, 88.0),
        ];
        let r = detect_mss_h4(&bars, 20);
        assert!(r.is_some(), "no debe abortar por serie corta");
        let (bars_ago, dir, _) = r.unwrap();
        assert_eq!(bars_ago, 1);
        assert_eq!(dir, "ALCISTA");
    }

    #[test]
    fn detect_mss_returns_none_when_no_mss() {
        let bars = vec![
            bar(3, 100.0, 95.0, 99.0),
            bar(2, 99.0, 94.0, 96.0),
            bar(1, 98.0, 93.0, 95.0),
            bar(0, 97.0, 92.0, 94.0),
        ];
        assert!(detect_mss_h4(&bars, 10).is_none());
    }
}
