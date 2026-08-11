use crate::calibration::Calibration;
use crate::structure::{EstructuraRef, zona_premium_discount};
use crate::types::{Bar, Signal};

fn fmt_price(v: f64, digits: i32) -> String {
    format!("{:.*}", digits.max(0) as usize, v)
}

/// CalcularVencimiento (espejo).
pub fn calcular_vencimiento(sig: &Signal) -> i32 {
    let atr = sig.atr14; // en puntos
    match sig.detector.as_str() {
        "D1" => {
            if atr > 20.0 {
                2
            } else {
                1
            }
        }
        "D2" | "D2_ANTICIPACION" => {
            if sig.kill_zone != "NONE" {
                1
            } else if atr > 15.0 {
                1
            } else {
                2
            }
        }
        "D3" | "D3_DEF" => {
            if sig.detector == "D3_DEF" {
                2
            } else if atr > 20.0 {
                2
            } else {
                1
            }
        }
        "D4" => {
            if sig.ob_confluence {
                1
            } else {
                2
            }
        }
        "D5" => {
            if sig.kill_zone != "NONE" {
                2
            } else {
                4
            }
        }
        _ => 2,
    }
}

/// GenerarHipotesis (espejo completo, con `hipotesis_objetivo`).
pub fn generar_hipotesis(
    sig: &mut Signal,
    calib: &Calibration,
    estructura: &EstructuraRef,
    atr_buffer0: f64,
    m15: &[Bar],
    point: f64,
    digits: i32,
) {
    sig.hipotesis_expiry_velas = calcular_vencimiento(sig);
    sig.hipotesis_expiry_minutos = sig.hipotesis_expiry_velas * 15;

    let (ok_zona, zona) = zona_premium_discount(m15, sig.entry_price);
    if ok_zona {
        sig.hipotesis_zona = zona.clone();
    }

    // Objetivo numérico
    if estructura.sweep_nivel > 0.0 {
        sig.hipotesis_objetivo = estructura.sweep_nivel;
    } else {
        let mut atr = sig.atr14 * point;
        if atr <= 0.0 {
            atr = atr_buffer0;
        }
        sig.hipotesis_objetivo = if sig.direction == 1 {
            sig.entry_price + atr * 1.5
        } else {
            sig.entry_price - atr * 1.5
        };
    }

    let mut prob_base = 55i32;
    let mut atr = sig.atr14 * point;
    if atr <= 0.0 {
        atr = atr_buffer0;
    }
    if atr <= 0.0 {
        return;
    }

    let mut causa = String::new();
    let mut efecto = String::new();
    let mut razon = String::new();
    let mut invalidez = String::new();

    match sig.detector.as_str() {
        "D1" => {
            let dir_ruptura = if sig.direction == 1 { "alcista" } else { "bajista" };
            let nivel = fmt_price(sig.nivel_estructural, digits);
            let estructura = sig.estructura_direccion.clone();
            let invalidez_nivel = if sig.direction == 1 {
                fmt_price(sig.nivel_estructural - atr * 0.3, digits)
            } else {
                fmt_price(sig.nivel_estructural + atr * 0.3, digits)
            };

            causa = format!("Ruptura {} de {}", dir_ruptura, nivel);
            efecto = format!(
                "va a provocar continuación {} hacia {}",
                dir_ruptura,
                fmt_price(sig.hipotesis_objetivo, digits)
            );
            razon = format!(
                "porque la vela actual rompe con fuerza (BR={:.2}) y la tendencia {} confirma",
                sig.br, estructura
            );
            invalidez = format!("Si rompe {} en contra, se invalida", invalidez_nivel);

            prob_base = calib.prob_base_for("D1");
            if sig.br > 0.70 {
                prob_base += 5;
            }
            if sig.bs > 1.0 {
                prob_base += 5;
            }
            if sig.g1_compresion >= 60.0 {
                prob_base += 5;
            }
            if sig.g2_persistencia >= 60.0 {
                prob_base += 5;
            }
            if sig.kill_zone != "NONE" {
                prob_base += 5;
            }
        }
        "D2" | "D2_ANTICIPACION" => {
            let zona_text = sig.hipotesis_zona.clone();
            let accion = if sig.direction == 1 { "rebote alcista" } else { "rechazo bajista" };
            let nivel = fmt_price(sig.level_swept, digits);
            let estructura = sig.estructura_direccion.clone();
            let invalidez_nivel = if sig.direction == 1 {
                fmt_price(sig.level_swept - atr * 0.3, digits)
            } else {
                fmt_price(sig.level_swept + atr * 0.3, digits)
            };

            causa = format!("Sweep en {} en zona {}", nivel, zona_text);
            efecto = format!(
                "va a provocar {} hacia {}",
                accion,
                fmt_price(sig.hipotesis_objetivo, digits)
            );
            razon = format!("porque el sweep liquida stops y la tendencia {} confirma", estructura);
            invalidez = format!("Si rompe {}, se invalida", invalidez_nivel);

            prob_base = calib.prob_base_for("D2");
            if sig.equal_hl_detected {
                prob_base += 5;
            }
            if sig.hipotesis_zona == "PREMIUM" && sig.direction == -1 {
                prob_base += 5;
            }
            if sig.hipotesis_zona == "DISCOUNT" && sig.direction == 1 {
                prob_base += 5;
            }
            if sig.kill_zone != "NONE" {
                prob_base += 5;
            }
            if sig.sweep_volume_ratio > 1.8 {
                prob_base += 5;
            }
            if sig.g4_agotamiento >= 65.0 {
                prob_base -= 10;
            }
        }
        "D3" | "D3_DEF" => {
            let dir_fvg = if sig.direction == -1 { "BAJISTA" } else { "ALCISTA" };
            let zona_text = sig.hipotesis_zona.clone();
            let accion = if sig.direction == -1 { "rechazo bajista" } else { "rebote alcista" };
            let defensa = if sig.detector == "D3_DEF" {
                if sig.direction == -1 {
                    "Los vendedores defienden la zona".to_string()
                } else {
                    "Los compradores defienden la zona".to_string()
                }
            } else {
                "La zona está activa".to_string()
            };
            let estructura = sig.estructura_direccion.clone();
            let invalidez_nivel = if sig.direction == -1 {
                fmt_price(sig.fvg_top, digits)
            } else {
                fmt_price(sig.fvg_bottom, digits)
            };
            let direccion_invalidez = if sig.direction == -1 { "al alza" } else { "a la baja" };

            causa = format!("FVG {} en zona {}", dir_fvg, zona_text);
            efecto = format!(
                "va a provocar {} hacia {}",
                accion,
                fmt_price(sig.hipotesis_objetivo, digits)
            );
            razon = format!("porque {} y la tendencia {} confirma", defensa, estructura);
            invalidez = format!("Si rompe {} {}, se invalida", invalidez_nivel, direccion_invalidez);

            prob_base = calib.prob_base_for("D3");
            if sig.detector == "D3_DEF" {
                prob_base += 5;
            }
            if sig.hipotesis_zona == "PREMIUM" && sig.direction == -1 {
                prob_base += 5;
            }
            if sig.hipotesis_zona == "DISCOUNT" && sig.direction == 1 {
                prob_base += 5;
            }
            if sig.kill_zone != "NONE" {
                prob_base += 5;
            }
            if sig.g1_compresion >= 60.0 {
                prob_base += 5;
            }
            if sig.mss_aligned {
                prob_base += 5;
            }
            if sig.g4_agotamiento >= 65.0 {
                prob_base -= 10;
            }
        }
        "D4" => {
            let accion = if sig.direction == 1 { "rebote alcista" } else { "rechazo bajista" };
            let nivel = fmt_price((sig.ob_high + sig.ob_low) / 2.0, digits);
            let estructura = sig.estructura_direccion.clone();
            let invalidez_nivel = if sig.direction == 1 {
                fmt_price(sig.ob_low, digits)
            } else {
                fmt_price(sig.ob_high, digits)
            };

            causa = format!("Order Block en {}", nivel);
            efecto = format!(
                "va a provocar {} hacia {}",
                accion,
                fmt_price(sig.hipotesis_objetivo, digits)
            );
            razon = format!(
                "porque el OB representa acumulación/distribución y la tendencia {} confirma",
                estructura
            );
            invalidez = format!("Si rompe {}, se invalida", invalidez_nivel);

            prob_base = calib.prob_base_for("D4");
            if sig.ob_impulse_atr > 1.5 {
                prob_base += 5;
            }
            if sig.ob_bars_ago <= 3 {
                prob_base += 5;
            }
            if sig.kill_zone != "NONE" {
                prob_base += 5;
            }
            if sig.g1_compresion >= 60.0 {
                prob_base += 5;
            }
        }
        "D5" => {
            let dir_mss = sig.mss_direction.clone();
            let accion = if sig.direction == 1 { "continuación alcista" } else { "continuación bajista" };
            let nivel = fmt_price(sig.level_swept, digits);
            let invalidez_nivel = if sig.direction == 1 {
                fmt_price(sig.level_swept - atr * 0.5, digits)
            } else {
                fmt_price(sig.level_swept + atr * 0.5, digits)
            };

            causa = format!("MSS H4 {} con sweep en {}", dir_mss, nivel);
            efecto = format!(
                "va a provocar {} hacia {}",
                accion,
                fmt_price(sig.hipotesis_objetivo, digits)
            );
            razon = format!(
                "porque el cambio de estructura en H4 confirma la dirección y el sweep valida la entrada"
            );
            invalidez = format!("Si rompe {}, se invalida", invalidez_nivel);

            prob_base = calib.prob_base_for("D5");
            if sig.mss_bars_ago_h4 <= 4 {
                prob_base += 5;
            }
            if sig.kill_zone != "NONE" {
                prob_base += 5;
            }
            if sig.g1_compresion >= 60.0 {
                prob_base += 5;
            }
            if sig.g2_persistencia >= 60.0 {
                prob_base += 5;
            }
        }
        _ => {}
    }

    prob_base = prob_base.min(95).max(30);
    sig.hipotesis_prob_min = (prob_base - 5).max(30);
    sig.hipotesis_prob_max = (prob_base + 5).min(95);

    sig.hipotesis_causa = causa;
    sig.hipotesis_efecto = efecto;
    sig.hipotesis_razon = razon;
    sig.hipotesis_invalidez = invalidez;
    sig.hipotesis_texto = format!(
        "{}\n{}\n{}\n{}",
        sig.hipotesis_causa, sig.hipotesis_efecto, sig.hipotesis_razon, sig.hipotesis_invalidez
    );
}
