//! ts-rs generation (§2): every IPC-crossing type lands in `src/ipc/gen/`
//! via this cargo-test export hook. Run `cargo test -p nameshift
//! export_bindings` after changing any `#[derive(TS)]` type.

use ts_rs::{Config, TS};

#[test]
fn export_bindings() {
    let out = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/ipc/gen");
    // Version counters and sizes stay far below 2^53 — plain numbers on the
    // TS side, not bigint.
    let config = Config::new().with_out_dir(out).with_large_int("number");
    // Top-level payloads pull in every dependency transitively.
    nameshift_lib::app_state::StateSnapshot::export_all(&config).unwrap();
    nameshift_lib::app_state::PreviewPayload::export_all(&config).unwrap();
    nameshift_lib::app_state::RevertPreviewPayload::export_all(&config).unwrap();
    nameshift_lib::app_state::SnapshotMeta::export_all(&config).unwrap();
    nameshift_lib::app_state::PlatformInfo::export_all(&config).unwrap();
    nameshift_lib::app_state::InspectorPayload::export_all(&config).unwrap();
    nameshift_lib::error::AppError::export_all(&config).unwrap();
    nameshift_lib::events::StateChanged::export_all(&config).unwrap();
    nameshift_lib::events::Processing::export_all(&config).unwrap();
    nameshift_lib::events::ApplyFinished::export_all(&config).unwrap();
    nameshift_lib::events::RevertFinished::export_all(&config).unwrap();
    nameshift_lib::events::Alert::export_all(&config).unwrap();
    nameshift_lib::events::OpenPaths::export_all(&config).unwrap();
    nameshift_engine::csv::CsvMatchReport::export_all(&config).unwrap();
    nameshift_engine::csv::CsvMatch::export_all(&config).unwrap();
    nameshift_engine::FilterMode::export_all(&config).unwrap();
}
