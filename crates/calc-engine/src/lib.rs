//! Deterministic versioned medical calculators (plan §16, surfaced in the
//! session tool tray via QB-13). Standard published formulas, unit-tested
//! against worked examples. Every call site must label these for exam
//! practice — they are explicitly NOT for clinical use (§16).
//!
//! The same crate compiles for the server, the Tauri apps, and WebAssembly
//! (§20.1 shared core): no second implementation anywhere.

/// A caller error: bad units or impossible inputs. Not a panic — the engine
/// is on request paths (§31.1).
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidInput(&'static str);

impl core::fmt::Display for InvalidInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0)
    }
}

fn positive(value: f64, name: &'static str) -> Result<f64, InvalidInput> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(InvalidInput(name))
    }
}

fn non_negative(value: f64, name: &'static str) -> Result<f64, InvalidInput> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(InvalidInput(name))
    }
}

/// Body mass index, kg/m².
pub fn bmi(weight_kg: f64, height_m: f64) -> Result<f64, InvalidInput> {
    let h = positive(height_m, "height_m must be > 0")?;
    let w = positive(weight_kg, "weight_kg must be > 0")?;
    Ok(w / (h * h))
}

/// Mosteller body surface area, m².
pub fn mosteller_bsa(weight_kg: f64, height_cm: f64) -> Result<f64, InvalidInput> {
    let w = positive(weight_kg, "weight_kg must be > 0")?;
    let h = positive(height_cm, "height_cm must be > 0")?;
    Ok((w * h / 3600.0).sqrt())
}

/// Mean arterial pressure, mmHg: (SBP + 2·DBP) / 3.
pub fn mean_arterial_pressure(systolic: f64, diastolic: f64) -> Result<f64, InvalidInput> {
    let s = positive(systolic, "systolic must be > 0")?;
    let d = positive(diastolic, "diastolic must be > 0")?;
    if s < d {
        return Err(InvalidInput("systolic must be >= diastolic"));
    }
    Ok((s + 2.0 * d) / 3.0)
}

/// Glasgow Coma Scale from component scores (eye 1–4, verbal 1–5, motor 1–6).
pub fn glasgow_coma_scale(eye: u8, verbal: u8, motor: u8) -> Result<u8, InvalidInput> {
    if !(1..=4).contains(&eye) {
        return Err(InvalidInput("eye score must be 1..=4"));
    }
    if !(1..=5).contains(&verbal) {
        return Err(InvalidInput("verbal score must be 1..=5"));
    }
    if !(1..=6).contains(&motor) {
        return Err(InvalidInput("motor score must be 1..=6"));
    }
    Ok(eye + verbal + motor)
}

/// Cockcroft–Gault creatinine clearance, mL/min.
pub fn cockcroft_gault_crcl(
    age_years: f64,
    weight_kg: f64,
    serum_creatinine_mg_dl: f64,
    female: bool,
) -> Result<f64, InvalidInput> {
    let age = positive(age_years, "age_years must be > 0")?;
    let w = positive(weight_kg, "weight_kg must be > 0")?;
    let scr = positive(serum_creatinine_mg_dl, "serum_creatinine_mg_dl must be > 0")?;
    let base = (140.0 - age) * w / (72.0 * scr);
    Ok(if female { base * 0.85 } else { base })
}

/// CKD-EPI 2021 race-free eGFR, mL/min/1.73 m² (Inker et al. 2021).
pub fn ckd_epi_2021_egfr(
    serum_creatinine_mg_dl: f64,
    age_years: f64,
    female: bool,
) -> Result<f64, InvalidInput> {
    let scr = positive(serum_creatinine_mg_dl, "serum_creatinine_mg_dl must be > 0")?;
    let age = positive(age_years, "age_years must be > 0")?;
    let kappa = if female { 0.7 } else { 0.9 };
    let alpha = if female { -0.241 } else { -0.302 };
    let sex_factor = if female { 1.012 } else { 1.0 };
    let ratio = scr / kappa;
    let term = if ratio <= 1.0 {
        ratio.powf(alpha)
    } else {
        ratio.powf(-1.200)
    };
    Ok(142.0 * term * 0.9938_f64.powf(age) * sex_factor)
}

/// Anion gap, mmol/L: Na⁺ − (Cl⁻ + HCO₃⁻).
pub fn anion_gap(
    sodium_mmol_l: f64,
    chloride_mmol_l: f64,
    bicarbonate_mmol_l: f64,
) -> Result<f64, InvalidInput> {
    let na = positive(sodium_mmol_l, "sodium must be > 0")?;
    let cl = positive(chloride_mmol_l, "chloride must be > 0")?;
    let hco3 = positive(bicarbonate_mmol_l, "bicarbonate must be > 0")?;
    Ok(na - (cl + hco3))
}

/// Corrected calcium, mg/dL: Ca + 0.8 × (4.0 − albumin).
pub fn corrected_calcium(calcium_mg_dl: f64, albumin_g_dl: f64) -> Result<f64, InvalidInput> {
    let ca = positive(calcium_mg_dl, "calcium must be > 0")?;
    let alb = non_negative(albumin_g_dl, "albumin must be >= 0")?;
    Ok(ca + 0.8 * (4.0 - alb))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-2
    }

    #[test]
    fn worked_examples_match_published_values() {
        assert!(approx(bmi(70.0, 1.75).unwrap(), 22.86));
        assert!(approx(mosteller_bsa(70.0, 170.0).unwrap(), 1.82));
        assert!(approx(mean_arterial_pressure(120.0, 80.0).unwrap(), 93.33));
        assert_eq!(glasgow_coma_scale(4, 5, 6).unwrap(), 15);
        assert!(approx(
            cockcroft_gault_crcl(40.0, 70.0, 1.0, false).unwrap(),
            97.22
        ));
        // CKD-EPI 2021 reference example: male, 40y, SCr 1.0 -> ~97.6
        assert!(approx(ckd_epi_2021_egfr(1.0, 40.0, false).unwrap(), 97.6));
        assert!(approx(anion_gap(140.0, 102.0, 26.0).unwrap(), 12.0));
        assert!(approx(corrected_calcium(8.5, 2.0).unwrap(), 10.1));
    }

    #[test]
    fn female_factors_apply() {
        assert!(approx(
            cockcroft_gault_crcl(40.0, 70.0, 1.0, true).unwrap(),
            97.22 * 0.85
        ));
        // Same inputs, female multiplier 1.012 on the male result.
        let male = ckd_epi_2021_egfr(1.0, 40.0, false).unwrap();
        let female = ckd_epi_2021_egfr(0.7, 40.0, true).unwrap();
        assert!(female > male, "lower SCr with kappa 0.7 yields higher eGFR");
    }

    #[test]
    fn rejects_impossible_inputs() {
        assert!(bmi(70.0, 0.0).is_err());
        assert!(mean_arterial_pressure(80.0, 120.0).is_err());
        assert_eq!(
            glasgow_coma_scale(0, 5, 6),
            Err(InvalidInput("eye score must be 1..=4"))
        );
        assert_eq!(
            glasgow_coma_scale(4, 5, 7),
            Err(InvalidInput("motor score must be 1..=6"))
        );
        assert!(cockcroft_gault_crcl(40.0, 70.0, 0.0, false).is_err());
        assert!(corrected_calcium(8.5, -1.0).is_err());
    }
}
