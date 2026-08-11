/// Clasificadores A/B/C/D por detector (espejo del EA).

pub fn clasificar_d1(br: f64, bs: f64, session: &str, body_ratio_min: f64) -> &'static str {
    if session == "ASIA" || session == "OUT" {
        if br > 0.70 && bs > 0.80 {
            return "B";
        }
        return "D";
    }
    if br > 0.70 && bs > 0.80 {
        return "A";
    }
    if br > 0.60 && bs > 0.50 {
        return "B";
    }
    if br > body_ratio_min && bs > 0.30 {
        return "C";
    }
    "D"
}

pub fn clasificar_d2(wick: f64, vol: f64, reclaim: f64, equal_hl: bool, sweep_wick_min: f64, reclaim_body_min: f64) -> &'static str {
    if equal_hl && wick > 0.70 && vol > 1.80 && reclaim > 0.70 {
        return "A";
    }
    if wick > 0.65 && vol > 1.50 && reclaim > 0.60 {
        return "B";
    }
    if wick > sweep_wick_min && reclaim > reclaim_body_min {
        return "C";
    }
    "D"
}

pub fn clasificar_d2_anticipacion(wick: f64, vol: f64, confluencias: i32) -> &'static str {
    if confluencias >= 3 && wick > 0.65 && vol > 1.50 {
        return "A";
    }
    if confluencias >= 2 && wick > 0.55 && vol > 1.20 {
        return "B";
    }
    if confluencias >= 2 {
        return "C";
    }
    "D"
}

pub fn clasificar_d3(fvg_size: f64, br: f64, trend: i32, _slope: f64, min_size_atr: f64, body_ratio: f64) -> &'static str {
    if fvg_size > 0.50 && br > 0.70 && trend >= 3 {
        return "A";
    }
    if fvg_size > 0.35 && br > 0.60 {
        return "B";
    }
    if fvg_size > min_size_atr && br > body_ratio {
        return "C";
    }
    "D"
}

pub fn clasificar_d4(impulso: f64, vol: f64, ob_bars: i32, ob_impulse_min: f64) -> &'static str {
    if impulso > 1.80 && vol > 1.50 && ob_bars <= 6 {
        return "A";
    }
    if impulso > 1.40 && vol > 1.20 {
        return "B";
    }
    if impulso >= ob_impulse_min {
        return "C";
    }
    "D"
}

pub fn clasificar_d5(mss_bars: i32, wick: f64, reclaim: f64, kill_zone: &str, sweep_wick_min: f64, reclaim_body_min: f64) -> &'static str {
    let in_kill = kill_zone == "LONDON_OPEN_KILL" || kill_zone == "NY_OPEN_KILL";
    if mss_bars <= 4 && wick > 0.70 && reclaim > 0.70 && in_kill {
        return "A";
    }
    if mss_bars <= 8 && wick > 0.60 && reclaim > 0.60 {
        return "B";
    }
    if wick > sweep_wick_min && reclaim > reclaim_body_min {
        return "C";
    }
    "D"
}
