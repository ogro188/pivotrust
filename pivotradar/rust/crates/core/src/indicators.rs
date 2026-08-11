use crate::types::Bar;

/// Clamp 0..100 (espejo de `Clamp01100`).
pub fn clamp_0_100(v: f64) -> f64 {
    if v < 0.0 { 0.0 } else if v > 100.0 { 100.0 } else { v }
}

/// min(a,b)
pub fn min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}

pub fn max(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

/// NormalizeDouble de MQL5 (redondeo half-away-from-zero a `digits` decimales).
pub fn round_to_digits(x: f64, digits: i32) -> f64 {
    let f = 10f64.powi(digits);
    (x * f).round() / f
}

/// Busca el índice de la vela cuyo time es exacto o el más cercano dentro de ±tolerancia_seg.
/// Si no hay coincidencia, retorna -1.
pub fn bar_shift_by_time(bars: &[Bar], time: i64, tolerancia_seg: i64) -> i32 {
    let mut best_idx: i32 = -1;
    let mut best_diff: i64 = i64::MAX;
    for (i, b) in bars.iter().enumerate() {
        let diff = (b.time - time).abs();
        if diff == 0 {
            return i as i32;
        }
        if diff < best_diff && diff <= tolerancia_seg {
            best_diff = diff;
            best_idx = i as i32;
        }
    }
    best_idx
}

/// EMA sobre closes en orden de serie (índice 0 = más reciente).
/// Seed = SMA de los primeros `period` valores cronológicos (como iMA MODE_EMA).
pub fn ema_close_series(bars: &[Bar], period: usize) -> Vec<f64> {
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    ema_series(&closes, period)
}

pub fn ema_series(values: &[f64], period: usize) -> Vec<f64> {
    let n = values.len();
    let mut out = vec![0.0; n];
    if n == 0 || period == 0 {
        return out;
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    if n < period {
        // Poca historia: promedio acumulado como aproximación.
        let mut acc = 0.0;
        let mut cnt = 0.0;
        for i in (0..n).rev() {
            acc += values[i];
            cnt += 1.0;
            out[i] = acc / cnt;
        }
        return out;
    }
    // Seed = SMA de los primeros `period` valores cronológicos: índices [n-period..n).
    let seed_sum: f64 = values[n - period..n].iter().sum();
    let mut ema = seed_sum / period as f64;
    out[n - period] = ema;
    for i in (0..=n - period - 1).rev() {
        ema = alpha * values[i] + (1.0 - alpha) * ema;
        out[i] = ema;
    }
    out
}

/// ATR (Wilder) en orden de serie. Espejo de iATR(symbol, M15, 14).
pub fn atr_series(bars: &[Bar], period: usize) -> Vec<f64> {
    let n = bars.len();
    let mut tr = vec![0.0; n];
    for i in (0..n).rev() {
        let h = bars[i].high;
        let l = bars[i].low;
        if i + 1 < n {
            let pc = bars[i + 1].close;
            tr[i] = (h - l).max((h - pc).abs()).max((l - pc).abs());
        } else {
            tr[i] = h - l;
        }
    }
    let mut atr = vec![0.0; n];
    if n >= period && period > 0 {
        let seed: f64 = tr[n - period..n].iter().sum::<f64>() / period as f64;
        atr[n - period] = seed;
        for i in (0..=n - period - 1).rev() {
            atr[i] = (atr[i + 1] * (period as f64 - 1.0) + tr[i]) / period as f64;
        }
    }
    atr
}

/// Hora/minuto "local del broker" a partir del tiempo Unix + offset fijo en horas.
pub fn local_hour_min(time: i64, offset_hours: i32) -> (i32, i32) {
    let shifted = time + offset_hours as i64 * 3600;
    let secs = shifted.rem_euclid(86400);
    ((secs / 3600) as i32, ((secs % 3600) / 60) as i32)
}

/// Espejo de GetSession (bar_time en hora del servidor).
pub fn session(hour: i32) -> &'static str {
    match hour {
        0..=6 => "ASIA",
        7..=12 => "LONDON",
        13..=14 => "NY_OPEN",
        15 => "LONDON_CLOSE",
        16..=20 => "NY",
        _ => "OUT",
    }
}

/// Espejo de GetKillZone.
pub fn kill_zone(hour: i32, minute: i32) -> &'static str {
    if hour == 7 || hour == 8 {
        return "LONDON_OPEN_KILL";
    }
    if hour == 13 || (hour == 14 && minute <= 30) {
        return "NY_OPEN_KILL";
    }
    if hour >= 13 && hour < 15 {
        return "LONDON_NY_OVERLAP";
    }
    "NONE"
}

/// Espejo de GetTrendD1. Usa índice [1] (vela D1 cerrada).
pub fn trend_d1(ema50_d1: &[f64], ema200_d1: &[f64]) -> &'static str {
    if ema50_d1.len() < 2 || ema200_d1.len() < 2 {
        return "NEUTRO";
    }
    let e50 = ema50_d1[1];
    let e200 = ema200_d1[1];
    if e50 == 0.0 || e200 == 0.0 {
        return "NEUTRO";
    }
    let eps = e200 * 0.0005;
    if e50 > e200 + eps {
        "ALCISTA"
    } else if e50 < e200 - eps {
        "BAJISTA"
    } else {
        "NEUTRO"
    }
}

/// Espejo de GetTrendVelas: velas consecutivas con EMA21/EMA50 alineadas.
pub fn trend_velas(ema21: &[f64], ema50: &[f64], max_i: usize) -> i32 {
    if ema21.len() < 2 || ema50.len() < 2 {
        return 0;
    }
    let up = ema21[1] > ema50[1];
    let down = ema21[1] < ema50[1];
    if !up && !down {
        return 0;
    }
    let mut count = 0;
    let lim = max_i.min(ema21.len()).min(ema50.len());
    for i in 1..lim {
        let u = ema21[i] > ema50[i];
        let d = ema21[i] < ema50[i];
        if !u && !d {
            continue;
        }
        if up && !u {
            break;
        }
        if down && !d {
            break;
        }
        count += 1;
    }
    count
}

/// Lee el valor en el ring buffer a `ago` posiciones atrás (0 = más reciente).
fn atr_history_at(history: &[f64; 20], head: usize, ago: usize) -> f64 {
    history[(head + 20 - ago) % 20]
}

/// Espejo de IsVolatilityExpanding (ring buffer de ATR, head = más reciente).
pub fn vol_expanding(history: &[f64; 20], head: usize) -> bool {
    if atr_history_at(history, head, 10) == 0.0 {
        return false;
    }
    let mut sum = 0.0;
    let mut cnt = 0.0;
    for i in 1..=10 {
        let v = atr_history_at(history, head, i);
        if v > 0.0 {
            sum += v;
            cnt += 1.0;
        }
    }
    if cnt == 0.0 {
        return false;
    }
    let avg = sum / cnt;
    atr_history_at(history, head, 0) > avg * 1.30
}

/// Espejo de IsVolatilityCompressing (ring buffer de ATR, head = más reciente).
pub fn vol_compressing(history: &[f64; 20], head: usize) -> bool {
    if atr_history_at(history, head, 10) == 0.0 {
        return false;
    }
    let mut sum = 0.0;
    let mut cnt = 0.0;
    for i in 1..=10 {
        let v = atr_history_at(history, head, i);
        if v > 0.0 {
            sum += v;
            cnt += 1.0;
        }
    }
    if cnt == 0.0 {
        return false;
    }
    let avg = sum / cnt;
    atr_history_at(history, head, 0) < avg * 0.80
}

/// Espejo de GetVolumeRatio: vol[shift] / media(vol[shift+1..shift+n]).
pub fn volume_ratio(bars: &[Bar], shift: usize, n: usize) -> f64 {
    let vol_signal = bars.get(shift).map(|b| b.volume).unwrap_or(0.0);
    if vol_signal <= 0.0 {
        return 1.0;
    }
    let mut sum = 0.0;
    let mut count = 0.0;
    for i in (shift + 1)..=(shift + n) {
        if let Some(b) = bars.get(i) {
            if b.volume > 0.0 {
                sum += b.volume;
                count += 1.0;
            }
        }
    }
    if count == 0.0 || sum <= 0.0 {
        return 1.0;
    }
    vol_signal / (sum / count)
}

/// Espejo de BuildSignalId (hash ulong, wrapping como MQL5).
pub fn build_signal_id(time: i64, detector: &str, direction: i32, level: f64, digits: i32) -> u64 {
    let mut hash = time as u64;
    for &b in detector.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(b as u64);
    }
    hash = hash.wrapping_mul(31).wrapping_add((direction + 2) as u64);
    let lvl = round_to_digits(level, digits);
    hash = hash.wrapping_mul(31).wrapping_add((lvl * 100_000.0).trunc() as u64);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_bars(n: usize) -> Vec<Bar> {
        let mut v = Vec::with_capacity(n);
        for i in (0..n).rev() {
            let t = (n as i64 - i as i64) * 900;
            let base = 100.0 + (i as f64) * 0.1;
            v.push(Bar { time: t, open: base, high: base + 0.3, low: base - 0.3, close: base + 0.1, volume: 100.0 });
        }
        v
    }

    #[test]
    fn ema_length_and_shape() {
        let bars = mk_bars(60);
        let e = ema_close_series(&bars, 21);
        assert_eq!(e.len(), 60);
        assert!(e[0] > 0.0);
    }

    #[test]
    fn atr_positive() {
        let bars = mk_bars(60);
        let a = atr_series(&bars, 14);
        assert!(a[0] > 0.0);
        assert!(a[0] < 1.0);
    }

    #[test]
    fn session_bounds() {
        assert_eq!(session(3), "ASIA");
        assert_eq!(session(9), "LONDON");
        assert_eq!(session(13), "NY_OPEN");
        assert_eq!(session(15), "LONDON_CLOSE");
        assert_eq!(session(18), "NY");
        assert_eq!(session(22), "OUT");
    }

    #[test]
    fn signal_id_deterministic() {
        let a = build_signal_id(1000, "D2", 1, 1.23456, 5);
        let b = build_signal_id(1000, "D2", 1, 1.23456, 5);
        assert_eq!(a, b);
    }

    /// Corrección 4: bar_shift_by_time tolerante a gaps (±900s).
    #[test]
    fn bar_shift_exact_match() {
        let bars = mk_bars(10);
        let idx = bar_shift_by_time(&bars, bars[3].time, 900);
        assert_eq!(idx, 3);
    }

    #[test]
    fn bar_shift_tolerant_to_gap() {
        // Serie sin time=1000 (gap), pero con time=1050 dentro de ±900s.
        let bars = vec![
            Bar { time: 10_800, open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: 1.0 },
            Bar { time: 10_750, open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: 1.0 },
            Bar { time: 1050, open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: 1.0 },
            Bar { time: 0, open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: 1.0 },
        ];
        let idx = bar_shift_by_time(&bars, 1000, 900);
        assert_eq!(idx, 2);
    }

    #[test]
    fn bar_shift_out_of_tolerance_returns_minus_one() {
        let bars = mk_bars(10);
        assert_eq!(bar_shift_by_time(&bars, 999_999, 900), -1);
    }
}
