use crate::indicators::clamp_0_100;
use crate::types::Bar;

/// CalcularG1_Compresion: ATR actual vs media 20.
pub fn g1_compresion(atr_now: f64, history: &[f64; 20]) -> f64 {
    if atr_now <= 0.0 {
        return 50.0;
    }
    let mut sum = 0.0;
    let mut cnt = 0.0;
    for i in 0..20 {
        if history[i] > 0.0 {
            sum += history[i];
            cnt += 1.0;
        }
    }
    if cnt == 0.0 {
        return 50.0;
    }
    let avg = sum / cnt;
    if avg <= 0.0 {
        return 50.0;
    }
    clamp_0_100((1.5 - atr_now / avg) / 1.0 * 100.0)
}

/// CalcularG2_Persistencia: dominancia de velas direccionales en 10 y 20 velas.
pub fn g2_persistencia(bars: &[Bar]) -> f64 {
    let mut up10 = 0;
    let mut down10 = 0;
    let mut up20 = 0;
    let mut down20 = 0;
    for i in 1..=20 {
        if i >= bars.len() {
            break;
        }
        let up = bars[i].close > bars[i].open;
        if i <= 10 {
            if up {
                up10 += 1;
            } else {
                down10 += 1;
            }
        }
        if up {
            up20 += 1;
        } else {
            down20 += 1;
        }
    }
    let d10 = (up10.max(down10)) as f64 / 10.0;
    let d20 = (up20.max(down20)) as f64 / 20.0;
    let p10 = clamp_0_100((d10 - 0.5) / 0.5 * 100.0);
    let p20 = clamp_0_100((d20 - 0.5) / 0.5 * 100.0);
    clamp_0_100(p10 * 0.6 + p20 * 0.4)
}

/// CalcularG3_Eficiencia: movimiento neto / rango total (10 velas).
pub fn g3_eficiencia(bars: &[Bar]) -> f64 {
    let n = 10usize;
    if bars.len() <= n {
        return 50.0;
    }
    let ini = bars[n].close;
    let fin = bars[0].close;
    let neto = (fin - ini).abs();
    let mut total = 0.0;
    for i in 0..n.min(100) {
        if i >= bars.len() {
            break;
        }
        let h = bars[i].high;
        let l = bars[i].low;
        if h == 0.0 || l == 0.0 {
            break;
        }
        total += h - l;
    }
    if total <= 0.0 {
        return 50.0;
    }
    clamp_0_100(neto / total * 100.0)
}

/// CalcularG4_Agotamiento: mechas recientes vs cuerpos (6 velas, mitades 3/3).
pub fn g4_agotamiento(bars: &[Bar], atr_now: f64) -> f64 {
    let n = 6usize;
    let m = n / 2;
    let mut mp = 0.0;
    let mut mu = 0.0;
    let mut cp = 0.0;
    let mut cu = 0.0;
    for i in 0..n.min(100) {
        if i >= bars.len() {
            break;
        }
        let o = bars[i].open;
        let c = bars[i].close;
        let h = bars[i].high;
        let l = bars[i].low;
        if h == 0.0 || l == 0.0 {
            break;
        }
        let r = h - l;
        if r <= 0.0 {
            continue;
        }
        let mecha = r - (c - o).abs();
        let cuerpo = (c - o).abs();
        if i < m {
            mu += mecha;
            cu += cuerpo;
        } else {
            mp += mecha;
            cp += cuerpo;
        }
    }
    if atr_now <= 0.0 {
        return 0.0;
    }
    let score_mechas = ((mu - mp) / atr_now) * 50.0;
    let score_cuerpos = ((cp - cu) / atr_now) * 50.0;
    clamp_0_100(clamp_0_100(score_mechas) + clamp_0_100(score_cuerpos))
}
