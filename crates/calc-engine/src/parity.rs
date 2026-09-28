//! ARCH-01 wasm runtime parity seam: extern "C" wrappers over the same
//! deterministic calculators the native tests cover. NaN is the documented
//! rejection sentinel (every wrapped function rejects non-positive input).
//! Enabled only with the `parity` feature so normal builds stay lean.

/// BMI, kg/m². NaN when the inputs are rejected.
#[no_mangle]
pub extern "C" fn parity_bmi(weight_kg: f64, height_m: f64) -> f64 {
    crate::bmi(weight_kg, height_m).unwrap_or(f64::NAN)
}

/// Mosteller BSA, m². NaN when the inputs are rejected.
#[no_mangle]
pub extern "C" fn parity_mosteller_bsa(weight_kg: f64, height_cm: f64) -> f64 {
    crate::mosteller_bsa(weight_kg, height_cm).unwrap_or(f64::NAN)
}

/// Mean arterial pressure, mmHg. NaN when the inputs are rejected.
#[no_mangle]
pub extern "C" fn parity_mean_arterial_pressure(systolic: f64, diastolic: f64) -> f64 {
    crate::mean_arterial_pressure(systolic, diastolic).unwrap_or(f64::NAN)
}
