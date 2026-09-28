//! ARCH-01: native side of the wasm runtime parity check. The same vectors
//! run through the wasm build via scripts/wasm-parity.mjs in CI; both
//! runtimes must agree with the recorded expectations.
use calc_engine::{bmi, mean_arterial_pressure, mosteller_bsa};

const VECTORS: &str = include_str!("../parity-vectors.json");

fn run(name: &str, f: impl Fn(f64, f64) -> Result<f64, calc_engine::InvalidInput>) {
    let vectors: serde_json::Value = serde_json::from_str(VECTORS).expect("parity vectors parse");
    for entry in vectors[name].as_array().expect("vector list") {
        let args = entry["args"].as_array().expect("args");
        let expected = &entry["expected"];
        // The wasm seam maps a rejected input to NaN; the native side agrees.
        let actual = f(
            args[0].as_f64().expect("arg"),
            args[1].as_f64().expect("arg"),
        )
        .unwrap_or(f64::NAN);
        if expected.is_string() {
            assert_eq!(expected.as_str(), Some("NaN"), "only NaN strings are used");
            assert!(actual.is_nan(), "{name}: expected rejection, got {actual}");
        } else {
            let want = expected.as_f64().expect("numeric expectation");
            assert_eq!(
                actual, want,
                "{name}: native result must match the parity vector"
            );
        }
    }
}

#[test]
fn parity_vectors_hold_natively() {
    run("bmi", bmi);
    run("mosteller_bsa", mosteller_bsa);
    run("mean_arterial_pressure", mean_arterial_pressure);
}
