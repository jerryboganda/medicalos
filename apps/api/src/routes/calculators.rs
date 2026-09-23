//! QB-13: study calculators backed by the shared deterministic engine.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const DISCLAIMER: &str = "For exam practice only; not for clinical use.";

#[derive(Deserialize)]
pub struct CalculatorInput {
    pub weight_kg: Option<f64>,
    pub height_m: Option<f64>,
    pub height_cm: Option<f64>,
    pub systolic: Option<f64>,
    pub diastolic: Option<f64>,
    pub eye: Option<f64>,
    pub verbal: Option<f64>,
    pub motor: Option<f64>,
    pub age_years: Option<f64>,
    pub serum_creatinine_mg_dl: Option<f64>,
    pub female: Option<bool>,
    pub sodium_mmol_l: Option<f64>,
    pub chloride_mmol_l: Option<f64>,
    pub bicarbonate_mmol_l: Option<f64>,
    pub calcium_mg_dl: Option<f64>,
    pub albumin_g_dl: Option<f64>,
}

fn required(value: Option<f64>, name: &'static str) -> ApiResult<f64> {
    value.ok_or_else(|| {
        ApiError::unprocessable("invalid_calculator_input", format!("{name} is required"))
    })
}

fn score(value: Option<f64>, name: &'static str) -> ApiResult<u8> {
    let value = required(value, name)?;
    if value.fract() != 0.0 || !(1.0..=6.0).contains(&value) {
        return Err(ApiError::unprocessable(
            "invalid_calculator_input",
            format!("{name} must be a whole-number component score"),
        ));
    }
    Ok(value as u8)
}

fn calculation_value(kind: &str, input: CalculatorInput) -> ApiResult<(f64, &'static str)> {
    let result = match kind {
        "bmi" => (
            calc_engine::bmi(
                required(input.weight_kg, "weight_kg")?,
                required(input.height_m, "height_m")?,
            ),
            "kg/m²",
        ),
        "bsa" => (
            calc_engine::mosteller_bsa(
                required(input.weight_kg, "weight_kg")?,
                required(input.height_cm, "height_cm")?,
            ),
            "m²",
        ),
        "map" => (
            calc_engine::mean_arterial_pressure(
                required(input.systolic, "systolic")?,
                required(input.diastolic, "diastolic")?,
            ),
            "mmHg",
        ),
        "gcs" => (
            calc_engine::glasgow_coma_scale(
                score(input.eye, "eye")?,
                score(input.verbal, "verbal")?,
                score(input.motor, "motor")?,
            )
            .map(f64::from),
            "points",
        ),
        "creatinine-clearance" => (
            calc_engine::cockcroft_gault_crcl(
                required(input.age_years, "age_years")?,
                required(input.weight_kg, "weight_kg")?,
                required(input.serum_creatinine_mg_dl, "serum_creatinine_mg_dl")?,
                input.female.ok_or_else(|| {
                    ApiError::unprocessable(
                        "invalid_calculator_input",
                        "female is required for this formula",
                    )
                })?,
            ),
            "mL/min",
        ),
        "egfr" => (
            calc_engine::ckd_epi_2021_egfr(
                required(input.serum_creatinine_mg_dl, "serum_creatinine_mg_dl")?,
                required(input.age_years, "age_years")?,
                input.female.ok_or_else(|| {
                    ApiError::unprocessable(
                        "invalid_calculator_input",
                        "female is required for this formula",
                    )
                })?,
            ),
            "mL/min/1.73 m²",
        ),
        "anion-gap" => (
            calc_engine::anion_gap(
                required(input.sodium_mmol_l, "sodium_mmol_l")?,
                required(input.chloride_mmol_l, "chloride_mmol_l")?,
                required(input.bicarbonate_mmol_l, "bicarbonate_mmol_l")?,
            ),
            "mmol/L",
        ),
        "corrected-calcium" => (
            calc_engine::corrected_calcium(
                required(input.calcium_mg_dl, "calcium_mg_dl")?,
                required(input.albumin_g_dl, "albumin_g_dl")?,
            ),
            "mg/dL",
        ),
        _ => {
            return Err(ApiError::unprocessable(
                "unknown_calculator",
                "calculator must be bmi, bsa, map, gcs, creatinine-clearance, egfr, anion-gap, or corrected-calcium",
            ));
        }
    };
    let value = result
        .0
        .map_err(|error| ApiError::unprocessable("invalid_calculator_input", error.to_string()))?;
    if !value.is_finite() {
        return Err(ApiError::unprocessable(
            "invalid_calculator_input",
            "calculation result must be finite",
        ));
    }
    Ok((value, result.1))
}

pub async fn calculate(
    State(_state): State<Arc<AppState>>,
    _user: AuthUser,
    Path(kind): Path<String>,
    Json(input): Json<CalculatorInput>,
) -> ApiResult<Json<serde_json::Value>> {
    let (value, unit) = calculation_value(&kind, input)?;
    Ok(Json(json!({
        "calculator": kind,
        "value": value,
        "unit": unit,
        "disclaimer": DISCLAIMER
    })))
}

#[derive(Deserialize)]
pub struct ConvertInput {
    pub value: f64,
    #[serde(default)]
    pub analyte: String,
    pub from: String,
    pub to: String,
}

pub async fn convert(
    State(_state): State<Arc<AppState>>,
    _user: AuthUser,
    Json(input): Json<ConvertInput>,
) -> ApiResult<Json<serde_json::Value>> {
    let value = calc_engine::convert_unit(input.value, &input.analyte, &input.from, &input.to)
        .map_err(|error| ApiError::unprocessable("invalid_conversion", error.to_string()))?;
    Ok(Json(json!({
        "value": value,
        "unit": input.to,
        "disclaimer": DISCLAIMER
    })))
}
