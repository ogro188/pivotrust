use crate::classifiers;
use crate::engine::{BarCtx, Engine};
use crate::indicators::{self, volume_ratio};
use crate::quality;
use crate::structure;
use crate::types::{Detector, Signal};

impl Engine {
    fn atr_now(&self) -> f64 {
        self.atr14_buffer[0]
    }

    fn slope(&self, atr_now: f64) -> f64 {
        let e0 = self.ema21_buffer.first().copied().unwrap_or(0.0);
        let e3 = self.ema21_buffer.get(3).copied().unwrap_or(e0);
        if atr_now > 0.0 {
            (e0 - e3) / atr_now
        } else {
            0.0
        }
    }

    fn current_bar(&self, mkt: &crate::engine::MarketData) -> i64 {
        mkt.m15.first().map(|b| b.time).unwrap_or(0)
    }

    fn blank(&self) -> Signal {
        Signal::default()
    }

    /// Espejo de MotorD1_IntraBar (Ruptura de rango).
    pub(crate) fn motor_d1(
        &mut self,
        mkt: &crate::engine::MarketData,
        ctx: &BarCtx,
        out: &mut Vec<Signal>,
    ) {
        let cur = self.current_bar(mkt);
        let Some(b0) = mkt.m15.first() else { return };
        if mkt.m15.len() < 2 {
            return;
        }
        let (high0, low0, close0, open0) = (b0.high, b0.low, b0.close, b0.open);
        if high0 == 0.0 || low0 == 0.0 || close0 == 0.0 {
            return;
        }
        let atr = self.atr_now();
        if atr <= 0.0 {
            return;
        }

        let mut highest_high = mkt.m15[1].high;
        let mut lowest_low = mkt.m15[1].low;
        let lim = (self.cal.n_ruptura + 1).max(2).min(99) as usize;
        for k in 2..=lim {
            let Some(b) = mkt.m15.get(k) else { break };
            if b.high == 0.0 || b.low == 0.0 {
                break;
            }
            if b.high > highest_high {
                highest_high = b.high;
            }
            if b.low < lowest_low {
                lowest_low = b.low;
            }
        }

        let (direction, nivel_ruptura, penetracion) = if high0 > highest_high {
            (1, highest_high, (high0 - highest_high) / atr)
        } else if low0 < lowest_low {
            (-1, lowest_low, (lowest_low - low0) / atr)
        } else {
            (0, 0.0, 0.0)
        };
        if direction == 0 {
            return;
        }
        if penetracion < self.cal.d1_atr_threshold {
            return;
        }

        let rango0 = high0 - low0;
        if rango0 <= 0.0 {
            return;
        }
        let br0 = (close0 - open0).abs() / rango0;
        if br0 < self.cal.body_ratio_min {
            return;
        }

        if self.cal.d1_use_volume {
            let vr = volume_ratio(&mkt.m15, 0, 20);
            if vr < self.cal.d1_min_volume {
                return;
            }
        }

        if self.cal.d1_use_retest {
            let retested = if direction == 1 {
                low0 <= nivel_ruptura && close0 > nivel_ruptura
            } else {
                high0 >= nivel_ruptura && close0 < nivel_ruptura
            };
            if !retested {
                return;
            }
        }

        let id = indicators::build_signal_id(cur, "D1", direction, nivel_ruptura, self.info.digits);
        if self.is_duplicate(id) || self.latch_fired(Detector::D1, direction, cur) {
            return;
        }
        self.mark_latch(Detector::D1, direction, cur);

        let mut sig = self.blank();
        sig.id = id;
        sig.entry_time = cur;
        sig.entry_bar_shift = 0;
        sig.symbol = self.info.symbol.clone();
        sig.direction = direction;
        sig.entry_price = close0;
        sig.detector = String::from("D1");
        sig.br = br0;
        sig.bs = penetracion;
        sig.nivel_estructural = nivel_ruptura;
        sig.atr14 = atr / self.info.point;
        sig.session = ctx.session.to_string();
        sig.kill_zone = ctx.kill_zone.to_string();
        sig.trend_d1 = ctx.trend_d1.to_string();
        sig.estructura_direccion = self.estructura.dir_estructura.clone();
        sig.g1_compresion = self.g1;
        sig.g2_persistencia = self.g2;
        sig.g4_agotamiento = self.g4;
        sig.volume_ratio = ctx.vol_ratio;
        sig.vol_expanding = ctx.vol_exp;
        sig.vol_compressing = ctx.vol_comp;
        sig.tipo = classifiers::clasificar_d1(br0, penetracion, ctx.session, self.cal.body_ratio_min).to_string();

        sig.salud_tendencial = quality::salud_tendencial(ctx.trend_velas, self.slope(atr), ctx.trend_d1, direction);
        let (cs, dist) = structure::evaluar_contexto_estructural(&self.estructura, direction, nivel_ruptura, atr, self.info.point);
        sig.contexto_estructural = cs;
        sig.distancia_al_sweep = dist;
        sig.en_zona_estructural = self.estructura.en_zona;
        out.push(sig);
    }

    /// Espejo de MotorD2_LiquiditySweep.
    pub(crate) fn motor_d2(
        &mut self,
        mkt: &crate::engine::MarketData,
        ctx: &BarCtx,
        out: &mut Vec<Signal>,
    ) {
        let cur = self.current_bar(mkt);
        let Some(b0) = mkt.m15.first() else { return };
        let (close0, open0, high0, low0) = (b0.close, b0.open, b0.high, b0.low);
        if close0 == 0.0 || high0 == 0.0 || low0 == 0.0 {
            return;
        }
        let atr = self.atr_now();
        if atr <= 0.0 {
            return;
        }

        let mut sweep_bar = -1i32;
        let mut sweep_dir = 0i32;
        let mut wick_found = 0.0;
        let mut vol_found = 0.0;
        let mut level = 0.0;
        let mut equal_hl = false;

        for i in 1..=2 {
            let Some(bi) = mkt.m15.get(i) else { continue };
            let (hi, li) = (bi.high, bi.low);
            if hi == 0.0 || li == 0.0 {
                continue;
            }
            let (oi, ci) = (bi.open, bi.close);
            let ri = hi - li;
            if ri <= 0.0 {
                continue;
            }

            let mut ph = mkt.m15.get(i + 1).map(|b| b.high).unwrap_or(0.0);
            let mut pl = mkt.m15.get(i + 1).map(|b| b.low).unwrap_or(0.0);
            let k_lim = (i + self.cal.sweep_n.max(0) as usize).min(99);
            for k in (i + 1)..=k_lim {
                let Some(bk) = mkt.m15.get(k) else { break };
                if bk.high == 0.0 || bk.low == 0.0 {
                    break;
                }
                if bk.high > ph {
                    ph = bk.high;
                }
                if bk.low < pl {
                    pl = bk.low;
                }
            }
            if ph == 0.0 || pl == 0.0 {
                continue;
            }

            let per_high = hi > ph && ci < ph;
            let per_low = li < pl && ci > pl;
            if !per_high && !per_low {
                continue;
            }

            let (wr, dc, lc) = if per_high {
                ((hi - oi.max(ci)) / ri, -1, ph)
            } else {
                ((oi.min(ci) - li) / ri, 1, pl)
            };
            if wr < self.cal.sweep_wick_min {
                continue;
            }

            let mut eq = false;
            let j_lim = (i + self.cal.equal_hl_window.max(0) as usize).min(99);
            for j in (i + 1)..=j_lim {
                let Some(bj) = mkt.m15.get(j) else { break };
                let (hj, lj) = (bj.high, bj.low);
                if hj == 0.0 || lj == 0.0 {
                    break;
                }
                if per_high {
                    if (hj - lc).abs() <= self.cal.equal_hl_tol * atr {
                        eq = true;
                        break;
                    }
                } else if (lj - lc).abs() <= self.cal.equal_hl_tol * atr {
                    eq = true;
                    break;
                }
            }
            if eq {
                equal_hl = true;
            }
            sweep_bar = i as i32;
            sweep_dir = dc;
            wick_found = wr;
            vol_found = volume_ratio(&mkt.m15, i, self.cal.sweep_n as usize);
            level = lc;
            break;
        }

        if sweep_bar == -1 || sweep_bar > 2 || (close0 - level).abs() > atr * 2.0 {
            return;
        }

        let br_reclaim = if high0 - low0 > 0.0 {
            (close0 - open0).abs() / (high0 - low0)
        } else {
            0.0
        };
        let reclaim_ok = (sweep_dir == 1 && close0 > open0 && close0 > level)
            || (sweep_dir == -1 && close0 < open0 && close0 < level);
        if !reclaim_ok || br_reclaim < self.cal.reclaim_body_min {
            return;
        }

        let id = indicators::build_signal_id(cur, "D2", sweep_dir, level, self.info.digits);
        if self.is_duplicate(id) || self.latch_fired(Detector::D2, sweep_dir, cur) {
            return;
        }
        self.mark_latch(Detector::D2, sweep_dir, cur);

        let mut sig = self.blank();
        sig.id = id;
        sig.entry_time = cur;
        sig.entry_bar_shift = 0;
        sig.symbol = self.info.symbol.clone();
        sig.direction = sweep_dir;
        sig.entry_price = close0;
        sig.detector = String::from("D2");
        sig.level_swept = level;
        sig.sweep_wick_ratio = wick_found;
        sig.sweep_volume_ratio = vol_found;
        sig.reclaim_body_ratio = br_reclaim;
        sig.equal_hl_detected = equal_hl;
        sig.atr14 = atr / self.info.point;
        sig.session = ctx.session.to_string();
        sig.kill_zone = ctx.kill_zone.to_string();
        sig.trend_d1 = ctx.trend_d1.to_string();
        sig.estructura_direccion = self.estructura.dir_estructura.clone();
        sig.g1_compresion = self.g1;
        sig.g2_persistencia = self.g2;
        sig.g4_agotamiento = self.g4;
        sig.volume_ratio = ctx.vol_ratio;
        sig.vol_expanding = ctx.vol_exp;
        sig.vol_compressing = ctx.vol_comp;
        sig.tipo = classifiers::clasificar_d2(
            wick_found, vol_found, br_reclaim, equal_hl, self.cal.sweep_wick_min, self.cal.reclaim_body_min,
        )
        .to_string();

        sig.calidad_sweep = quality::calidad_sweep(wick_found, br_reclaim, vol_found, sweep_bar, equal_hl);
        sig.salud_tendencial = quality::salud_tendencial(ctx.trend_velas, self.slope(atr), ctx.trend_d1, sweep_dir);
        let (cs, dist) = structure::evaluar_contexto_estructural(&self.estructura, sweep_dir, level, atr, self.info.point);
        sig.contexto_estructural = cs;
        sig.distancia_al_sweep = dist;
        sig.en_zona_estructural = self.estructura.en_zona;
        out.push(sig);
    }

    /// Espejo de MotorD2_Anticipacion.
    pub(crate) fn motor_d2_anticipacion(
        &mut self,
        mkt: &crate::engine::MarketData,
        ctx: &BarCtx,
        out: &mut Vec<Signal>,
    ) {
        let cur = self.current_bar(mkt);
        let Some(b0) = mkt.m15.first() else { return };
        if mkt.m15.len() < 2 {
            return;
        }
        let (high0, low0, close0, open0) = (b0.high, b0.low, b0.close, b0.open);
        if high0 == 0.0 || low0 == 0.0 || close0 == 0.0 || open0 == 0.0 {
            return;
        }
        let atr = self.atr_now();
        if atr <= 0.0 {
            return;
        }

        let mut prior_high = mkt.m15[1].high;
        let mut prior_low = mkt.m15[1].low;
        let lim = self.cal.sweep_n.max(2).min(99) as usize;
        for k in 2..=lim {
            let Some(b) = mkt.m15.get(k) else { break };
            if b.high == 0.0 || b.low == 0.0 {
                break;
            }
            if b.high > prior_high {
                prior_high = b.high;
            }
            if b.low < prior_low {
                prior_low = b.low;
            }
        }

        let sweep_high = high0 > prior_high;
        let sweep_low = low0 < prior_low;
        if !sweep_high && !sweep_low {
            return;
        }
        let sweep_dir = if sweep_high { -1 } else { 1 };
        let nivel_barrido = if sweep_high { prior_high } else { prior_low };

        let range = high0 - low0;
        if range <= 0.0 {
            return;
        }
        let wick_ratio = if sweep_high {
            (high0 - open0.max(close0)) / range
        } else {
            (open0.min(close0) - low0) / range
        };
        if wick_ratio < self.cal.sweep_wick_min * 0.6 {
            return;
        }

        let mut confluencias = 0i32;
        let mut hay_fvg = false;
        for i in 2..=5 {
            let (Some(ba), Some(bb), Some(bc)) = (mkt.m15.get(i), mkt.m15.get(i - 1), mkt.m15.get(i - 2)) else { continue };
            let (ha, la) = (ba.high, ba.low);
            let (hb, lb) = (bb.high, bb.low);
            let (hc, lc) = (bc.high, bc.low);
            if ha == 0.0 || la == 0.0 || hb == 0.0 || lb == 0.0 || hc == 0.0 || lc == 0.0 {
                continue;
            }
            if ha < lc {
                let ce = lc - (lc - ha) * 0.5;
                if (nivel_barrido - ce).abs() < atr * 0.5 {
                    hay_fvg = true;
                    break;
                }
            } else if la > hc {
                let ce = la - (la - hc) * 0.5;
                if (nivel_barrido - ce).abs() < atr * 0.5 {
                    hay_fvg = true;
                    break;
                }
            }
        }
        if hay_fvg {
            confluencias += 1;
        }

        let mut hay_ob = false;
        for i in 2..=4 {
            let Some(bi) = mkt.m15.get(i) else { continue };
            if i == 0 {
                continue;
            }
            let (oi, ci) = (bi.open, bi.close);
            let (hi, li) = (bi.high, bi.low);
            let ri = hi - li;
            if ri <= 0.0 || hi == 0.0 || li == 0.0 {
                continue;
            }
            if (ci - oi).abs() / ri < self.cal.ob_body_min {
                continue;
            }
            let nc = mkt.m15.get(i - 1).map(|b| b.close).unwrap_or(0.0);
            let imp = (nc - ci).abs() / atr;
            if imp < self.cal.ob_impulse_min {
                continue;
            }
            if (nivel_barrido - (hi + li) / 2.0).abs() < atr * 0.5 {
                hay_ob = true;
                break;
            }
        }
        if hay_ob {
            confluencias += 1;
        }

        if let Some((_, mss_dir, _)) = self.detect_mss_cached(mkt) {
            let md = if mss_dir == "ALCISTA" { 1 } else { -1 };
            if md == sweep_dir {
                confluencias += 1;
            }
        }

        if confluencias < 2 {
            return;
        }

        let id = indicators::build_signal_id(cur, "D2_ANTICIPACION", sweep_dir, nivel_barrido, self.info.digits);
        if self.is_duplicate(id) {
            return;
        }

        let mut sig = self.blank();
        sig.id = id;
        sig.entry_time = cur;
        sig.entry_bar_shift = 0;
        sig.symbol = self.info.symbol.clone();
        sig.direction = sweep_dir;
        sig.entry_price = close0;
        sig.detector = String::from("D2_ANTICIPACION");
        sig.level_swept = nivel_barrido;
        sig.sweep_wick_ratio = wick_ratio;
        sig.sweep_volume_ratio = ctx.vol_ratio;
        sig.atr14 = atr / self.info.point;
        sig.session = ctx.session.to_string();
        sig.kill_zone = ctx.kill_zone.to_string();
        sig.trend_d1 = ctx.trend_d1.to_string();
        sig.estructura_direccion = self.estructura.dir_estructura.clone();
        sig.g1_compresion = self.g1;
        sig.g2_persistencia = self.g2;
        sig.g4_agotamiento = self.g4;
        sig.volume_ratio = ctx.vol_ratio;
        sig.vol_expanding = ctx.vol_exp;
        sig.vol_compressing = ctx.vol_comp;
        sig.tipo = classifiers::clasificar_d2_anticipacion(wick_ratio, ctx.vol_ratio, confluencias).to_string();

        sig.calidad_sweep = quality::calidad_sweep(wick_ratio, 0.0, ctx.vol_ratio, 0, false);
        sig.salud_tendencial = quality::salud_tendencial(ctx.trend_velas, self.slope(atr), ctx.trend_d1, sweep_dir);
        let (cs, dist) = structure::evaluar_contexto_estructural(&self.estructura, sweep_dir, nivel_barrido, atr, self.info.point);
        sig.contexto_estructural = cs;
        sig.distancia_al_sweep = dist;
        sig.en_zona_estructural = self.estructura.en_zona;

        sig.conf_sweep_fvg = if hay_fvg { self.confluence_sweep_fvg(sweep_dir, true, 0.0) } else { 0.0 };
        sig.conf_completa = self.confluence_completa(sweep_dir, hay_fvg, 0.0);
        out.push(sig);
    }

    /// Espejo de MotorD3_IntraBar (Fair Value Gap).
    pub(crate) fn motor_d3(
        &mut self,
        mkt: &crate::engine::MarketData,
        ctx: &BarCtx,
        out: &mut Vec<Signal>,
    ) {
        if mkt.m15.len() < 3 {
            return;
        }
        let cur = self.current_bar(mkt);
        let b2 = &mkt.m15[2];
        let b1 = &mkt.m15[1];
        let b0 = &mkt.m15[0];
        let (ha, la) = (b2.high, b2.low);
        let (hb, lb) = (b1.high, b1.low);
        let (cb, ob) = (b1.close, b1.open);
        let (hc, lc) = (b0.high, b0.low);
        if ha == 0.0 || la == 0.0 || hb == 0.0 || lb == 0.0 || hc == 0.0 || lc == 0.0 {
            return;
        }
        let atr = self.atr_now();
        if atr <= 0.0 {
            return;
        }

        let fvg_alcista = ha < lc;
        let fvg_bajista = la > hc;
        if !fvg_alcista && !fvg_bajista {
            return;
        }

        let (fvg_size, fvg_top, fvg_bottom, direction) = if fvg_alcista {
            (lc - ha, lc, ha, 1)
        } else {
            (la - hc, la, hc, -1)
        };
        if fvg_size <= 0.0 {
            return;
        }

        let fvg_size_atr = fvg_size / atr;
        let br_b = if hb - lb > 0.0 { (cb - ob).abs() / (hb - lb) } else { 0.0 };
        let dir_ok = (fvg_alcista && cb > ob) || (fvg_bajista && cb < ob);
        if fvg_size_atr < self.cal.fvg_min_size_atr || br_b < self.cal.fvg_body_ratio || !dir_ok {
            return;
        }

        let mit_level = fvg_bottom + (fvg_top - fvg_bottom) * self.cal.fvg_mitig_umbral;
        let price0 = b0.close;
        let mitigado = (direction == 1 && price0 <= mit_level) || (direction == -1 && price0 >= mit_level);

        let mut defendido = false;
        let mut dentro_fvg = false;
        if direction == 1 && price0 > fvg_top {
            defendido = true;
        }
        if direction == -1 && price0 < fvg_bottom {
            defendido = true;
        }
        if direction == 1 && price0 >= fvg_bottom && price0 <= fvg_top {
            dentro_fvg = true;
        }
        if direction == -1 && price0 <= fvg_top && price0 >= fvg_bottom {
            dentro_fvg = true;
        }

        let det = if defendido || dentro_fvg { Detector::D3Def } else { Detector::D3 };
        let det_str = det.as_str();

        let id = indicators::build_signal_id(b1.time, det_str, direction, fvg_top, self.info.digits);
        if self.is_duplicate(id) || self.latch_fired(det, direction, cur) {
            return;
        }
        self.mark_latch(det, direction, cur);

        let mut sig = self.blank();
        sig.id = id;
        sig.entry_time = b1.time;
        sig.entry_bar_shift = 1;
        sig.symbol = self.info.symbol.clone();
        sig.direction = direction;
        sig.entry_price = price0;
        sig.detector = det_str.to_string();
        sig.fvg_top = fvg_top;
        sig.fvg_bottom = fvg_bottom;
        sig.fvg_size_atr = fvg_size_atr;
        sig.fvg_mitigated = mitigado;
        sig.atr14 = atr / self.info.point;
        sig.session = ctx.session.to_string();
        sig.kill_zone = ctx.kill_zone.to_string();
        sig.trend_d1 = ctx.trend_d1.to_string();
        sig.estructura_direccion = self.estructura.dir_estructura.clone();
        sig.g1_compresion = self.g1;
        sig.g2_persistencia = self.g2;
        sig.g4_agotamiento = self.g4;
        sig.volume_ratio = ctx.vol_ratio;
        sig.vol_expanding = ctx.vol_exp;
        sig.vol_compressing = ctx.vol_comp;

        if let Some((bars, mdir, mlvl)) = self.detect_mss_cached(mkt) {
            sig.mss_aligned = true;
            sig.mss_bars_ago_h4 = bars;
            sig.mss_direction = mdir;
            sig.mss_level = mlvl;
        }

        let slope = self.slope(atr);
        sig.tipo = classifiers::clasificar_d3(
            fvg_size_atr, br_b, ctx.trend_velas, slope, self.cal.fvg_min_size_atr, self.cal.fvg_body_ratio,
        )
        .to_string();

        sig.calidad_fvg = quality::calidad_fvg(fvg_size_atr, br_b, defendido, self.cal.fvg_min_size_atr, self.cal.fvg_body_ratio);
        sig.salud_tendencial = quality::salud_tendencial(ctx.trend_velas, slope, ctx.trend_d1, direction);
        let (cs, dist) = structure::evaluar_contexto_estructural(&self.estructura, direction, fvg_top, atr, self.info.point);
        sig.contexto_estructural = cs;
        sig.distancia_al_sweep = dist;
        sig.en_zona_estructural = self.estructura.en_zona;

        sig.conf_sweep_fvg = self.confluence_sweep_fvg(direction, true, fvg_size_atr);
        sig.conf_completa = self.confluence_completa(direction, true, fvg_size_atr);
        out.push(sig);
    }

    /// Espejo de MotorD4_OrderBlockConfluence.
    pub(crate) fn motor_d4(
        &mut self,
        mkt: &crate::engine::MarketData,
        ctx: &BarCtx,
        out: &mut Vec<Signal>,
    ) {
        let cur = self.current_bar(mkt);
        let Some(b0) = mkt.m15.first() else { return };
        let close0 = b0.close;
        if close0 == 0.0 {
            return;
        }
        let atr = self.atr_now();
        if atr <= 0.0 {
            return;
        }

        let mut ob_bar = -1i32;
        let mut ob_dir = 0i32;
        let mut ob_high = 0.0;
        let mut ob_low = 0.0;
        let mut ob_impulse = 0.0;
        let mut ob_vol = 0.0;

        for i in 2..=4 {
            let Some(bi) = mkt.m15.get(i) else { continue };
            let (oi, ci) = (bi.open, bi.close);
            let (hi, li) = (bi.high, bi.low);
            let ri = hi - li;
            if ri <= 0.0 || hi == 0.0 || li == 0.0 {
                continue;
            }
            if (ci - oi).abs() / ri < self.cal.ob_body_min {
                continue;
            }
            let di = if ci > oi { 1 } else { -1 };
            let nc = mkt.m15.get(i - 1).map(|b| b.close).unwrap_or(0.0);
            let imp = (nc - ci).abs() / atr;
            if imp < self.cal.ob_impulse_min {
                continue;
            }

            let mut tested = false;
            for j in (1..i).rev() {
                let Some(bj) = mkt.m15.get(j) else { break };
                let (hj, lj) = (bj.high, bj.low);
                if hj == 0.0 || lj == 0.0 {
                    break;
                }
                if di == 1 && lj <= hi {
                    tested = true;
                    break;
                }
                if di == -1 && hj >= li {
                    tested = true;
                    break;
                }
            }
            if tested {
                continue;
            }

            ob_bar = i as i32;
            ob_dir = di;
            ob_high = hi;
            ob_low = li;
            ob_impulse = imp;
            ob_vol = volume_ratio(&mkt.m15, i, self.cal.ob_lookback as usize);
            break;
        }

        if ob_bar == -1 || ob_bar > 4 {
            return;
        }

        let entering = (ob_dir == 1 && close0 <= ob_high && close0 >= ob_low)
            || (ob_dir == -1 && close0 >= ob_low && close0 <= ob_high);
        if !entering {
            return;
        }

        let centro = (ob_high + ob_low) / 2.0;
        if (close0 - centro).abs() > atr * 2.0 {
            return;
        }

        let id = indicators::build_signal_id(cur, "D4", ob_dir, ob_high, self.info.digits);
        if self.is_duplicate(id) || self.latch_fired(Detector::D4, ob_dir, cur) {
            return;
        }
        self.mark_latch(Detector::D4, ob_dir, cur);

        let mut sig = self.blank();
        sig.id = id;
        sig.entry_time = cur;
        sig.entry_bar_shift = 0;
        sig.symbol = self.info.symbol.clone();
        sig.direction = ob_dir;
        sig.entry_price = close0;
        sig.detector = String::from("D4");
        sig.ob_high = ob_high;
        sig.ob_low = ob_low;
        sig.ob_bars_ago = ob_bar;
        sig.ob_impulse_atr = ob_impulse;
        sig.ob_confluence = true;
        sig.atr14 = atr / self.info.point;
        sig.session = ctx.session.to_string();
        sig.kill_zone = ctx.kill_zone.to_string();
        sig.trend_d1 = ctx.trend_d1.to_string();
        sig.estructura_direccion = self.estructura.dir_estructura.clone();
        sig.g1_compresion = self.g1;
        sig.g2_persistencia = self.g2;
        sig.volume_ratio = ctx.vol_ratio;
        sig.vol_expanding = ctx.vol_exp;
        sig.vol_compressing = ctx.vol_comp;
        sig.tipo = classifiers::clasificar_d4(ob_impulse, ob_vol, ob_bar, self.cal.ob_impulse_min).to_string();

        sig.calidad_ob = quality::calidad_ob(ob_impulse, ob_bar, ob_vol, self.cal.ob_impulse_min, self.cal.ob_lookback);
        sig.salud_tendencial = quality::salud_tendencial(ctx.trend_velas, self.slope(atr), ctx.trend_d1, ob_dir);
        let (cs, dist) = structure::evaluar_contexto_estructural(&self.estructura, ob_dir, centro, atr, self.info.point);
        sig.contexto_estructural = cs;
        sig.distancia_al_sweep = dist;
        sig.en_zona_estructural = self.estructura.en_zona;
        out.push(sig);
    }

    /// Espejo de MotorD5_MSS_Sweep.
    pub(crate) fn motor_d5(
        &mut self,
        mkt: &crate::engine::MarketData,
        ctx: &BarCtx,
        out: &mut Vec<Signal>,
    ) {
        let Some((mss_bars, mss_dir, mss_level)) = self.detect_mss_cached(mkt) else { return };
        if mss_bars > self.cal.mss_max_age_h4_bars {
            return;
        }
        let mss_dir_int = if mss_dir == "ALCISTA" { 1 } else { -1 };

        let cur = self.current_bar(mkt);
        let Some(b0) = mkt.m15.first() else { return };
        let (close0, open0, high0, low0) = (b0.close, b0.open, b0.high, b0.low);
        if close0 == 0.0 || high0 == 0.0 || low0 == 0.0 {
            return;
        }
        let atr = self.atr_now();
        if atr <= 0.0 {
            return;
        }

        let mut sweep_bar = -1i32;
        let mut wick_found = 0.0;
        let mut level = 0.0;

        for i in 1..=2 {
            let Some(bi) = mkt.m15.get(i) else { continue };
            let (hi, li, oi, ci) = (bi.high, bi.low, bi.open, bi.close);
            if hi == 0.0 || li == 0.0 {
                continue;
            }
            let ri = hi - li;
            if ri <= 0.0 {
                continue;
            }

            let mut ph = mkt.m15.get(i + 1).map(|b| b.high).unwrap_or(0.0);
            let mut pl = mkt.m15.get(i + 1).map(|b| b.low).unwrap_or(0.0);
            let k_lim = (i + self.cal.sweep_n.max(0) as usize).min(99);
            for k in (i + 1)..=k_lim {
                let Some(bk) = mkt.m15.get(k) else { break };
                if bk.high == 0.0 || bk.low == 0.0 {
                    break;
                }
                if bk.high > ph {
                    ph = bk.high;
                }
                if bk.low < pl {
                    pl = bk.low;
                }
            }
            if ph == 0.0 || pl == 0.0 {
                continue;
            }

            if mss_dir_int == 1 {
                if !(li < pl && ci > pl) {
                    continue;
                }
                let w = (oi.min(ci) - li) / ri;
                if w < self.cal.sweep_wick_min {
                    continue;
                }
                sweep_bar = i as i32;
                wick_found = w;
                level = pl;
                break;
            } else {
                if !(hi > ph && ci < ph) {
                    continue;
                }
                let w = (hi - oi.max(ci)) / ri;
                if w < self.cal.sweep_wick_min {
                    continue;
                }
                sweep_bar = i as i32;
                wick_found = w;
                level = ph;
                break;
            }
        }

        if sweep_bar == -1 || sweep_bar > 2 || (close0 - level).abs() > atr * 2.0 {
            return;
        }

        let br_reclaim = if high0 - low0 > 0.0 {
            (close0 - open0).abs() / (high0 - low0)
        } else {
            0.0
        };
        let reclaim_ok = (mss_dir_int == 1 && close0 > level) || (mss_dir_int == -1 && close0 < level);
        if !reclaim_ok || br_reclaim < self.cal.reclaim_body_min {
            return;
        }

        let id = indicators::build_signal_id(cur, "D5", mss_dir_int, level, self.info.digits);
        if self.is_duplicate(id) || self.latch_fired(Detector::D5, mss_dir_int, cur) {
            return;
        }
        self.mark_latch(Detector::D5, mss_dir_int, cur);

        let mut sig = self.blank();
        sig.id = id;
        sig.entry_time = cur;
        sig.entry_bar_shift = 0;
        sig.symbol = self.info.symbol.clone();
        sig.direction = mss_dir_int;
        sig.entry_price = close0;
        sig.detector = String::from("D5");
        sig.mss_aligned = true;
        sig.mss_direction = mss_dir.clone();
        sig.mss_bars_ago_h4 = mss_bars;
        sig.mss_level = mss_level;
        sig.level_swept = level;
        sig.sweep_wick_ratio = wick_found;
        sig.reclaim_body_ratio = br_reclaim;
        sig.atr14 = atr / self.info.point;
        sig.session = ctx.session.to_string();
        sig.kill_zone = ctx.kill_zone.to_string();
        sig.trend_d1 = ctx.trend_d1.to_string();
        sig.estructura_direccion = self.estructura.dir_estructura.clone();
        sig.g1_compresion = self.g1;
        sig.g2_persistencia = self.g2;
        sig.g4_agotamiento = self.g4;
        sig.volume_ratio = ctx.vol_ratio;
        sig.vol_expanding = ctx.vol_exp;
        sig.vol_compressing = ctx.vol_comp;
        sig.tipo = classifiers::clasificar_d5(
            mss_bars, wick_found, br_reclaim, ctx.kill_zone, self.cal.sweep_wick_min, self.cal.reclaim_body_min,
        )
        .to_string();

        sig.calidad_sweep = quality::calidad_sweep(wick_found, br_reclaim, ctx.vol_ratio, sweep_bar, false);
        sig.calidad_mss = quality::calidad_mss(wick_found, br_reclaim, mss_bars, self.cal.mss_max_age_h4_bars);
        sig.salud_tendencial = quality::salud_tendencial(ctx.trend_velas, self.slope(atr), ctx.trend_d1, mss_dir_int);
        let (cs, dist) = structure::evaluar_contexto_estructural(&self.estructura, mss_dir_int, level, atr, self.info.point);
        sig.contexto_estructural = cs;
        sig.distancia_al_sweep = dist;
        sig.en_zona_estructural = self.estructura.en_zona;

        sig.conf_completa = self.confluence_completa(mss_dir_int, false, 0.0);
        out.push(sig);
    }
}
