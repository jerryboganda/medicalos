// ARCH-01: wasm runtime parity check. Executes the shared parity vectors
// (crates/calc-engine/parity-vectors.json) against the wasm cdylib build of
// calc-engine and fails on any disagreement with the recorded expectations.
import { readFileSync } from 'node:fs';

const vectors = JSON.parse(
  readFileSync(new URL('../crates/calc-engine/parity-vectors.json', import.meta.url))
);
const wasmPath = new URL(
  '../target/wasm32-unknown-unknown/release/calc_engine.wasm',
  import.meta.url
);
const bytes = readFileSync(wasmPath);
const { instance } = await WebAssembly.instantiate(bytes, {});
const fns = {
  bmi: instance.exports.parity_bmi,
  mosteller_bsa: instance.exports.parity_mosteller_bsa,
  mean_arterial_pressure: instance.exports.parity_mean_arterial_pressure
};

let failures = 0;
let cases = 0;
for (const [name, casesForFn] of Object.entries(vectors)) {
  if (name === 'comment') continue;
  const fn = fns[name];
  if (typeof fn !== 'function') {
    console.error(`parity: missing wasm export parity_${name}`);
    failures += 1;
    continue;
  }
  for (const { args, expected } of casesForFn) {
    cases += 1;
    const actual = fn(...args);
    const expectedNumber = expected === 'NaN' ? Number.NaN : expected;
    const same =
      expected === 'NaN'
        ? Number.isNaN(actual)
        : Object.is(actual, expectedNumber);
    if (!same) {
      failures += 1;
      console.error(`parity mismatch ${name}(${args.join(', ')}): expected ${expected}, got ${actual}`);
    }
  }
}
console.log(`parity: ${cases} cases, ${failures} failures`);
process.exit(failures === 0 ? 0 : 1);
