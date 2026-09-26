# ARCH-02 code-generation research

## Decision

Use `ts-rs` to generate the client bindings from the Rust API DTOs already
serialized by the server. Keep the existing TypeScript request functions and
authentication transport; replace their hand-maintained data shapes with
generated declarations incrementally.

## Primary-source findings

- The `ts-rs` maintainers document `TS` derives, Serde compatibility, automatic
  dependency export, and test-driven file generation through `#[ts(export)]`.
  The export directory is configurable with `TS_RS_EXPORT_DIR`.
- The crate's documented UUID integration is enabled with `uuid-impl`; the
  default `serde-compat` feature reads supported Serde naming and omission
  attributes.
- The current documented release is 12.0.1 and its stated minimum Rust version
  is 1.88. The repository uses stable Rust in GitHub Actions.

Sources: [ts-rs README](https://github.com/aleph-alpha/ts-rs),
[TS trait documentation](https://docs.rs/ts-rs/latest/ts_rs/trait.TS.html),
[v12 release notes](https://github.com/aleph-alpha/ts-rs/releases/tag/v12.0.0).

## Scope limits

The first vertical slice exports the `TodayResponse` DTO and removes its
duplicated TypeScript shapes. Request transport and unrelated client contracts
stay unchanged. ARCH-02 remains in progress until every client-consumed public
request and response shape has moved to Rust-owned generated declarations and
CI rejects stale output.
