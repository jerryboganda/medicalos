//! Shared telemetry init for API and workers (plan §20.1: tracing with
//! OpenTelemetry). OTLP export is wired in Phase 1 when the runtime lands;
//! today this provides local fmt logging with env-filtered levels.

pub fn init() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}
