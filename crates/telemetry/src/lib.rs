//! Shared telemetry init for API and workers (plan §20.1: tracing with
//! OpenTelemetry). Local fmt logging is always on with env-filtered levels.
//! When `OTEL_EXPORTER_OTLP_ENDPOINT` is set — and the crate is built with
//! its `otlp` feature — spans additionally export to that collector over
//! OTLP/HTTP. No endpoint or no feature: exactly the fmt-only subscriber.

pub fn init() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    match std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        Ok(endpoint) if !endpoint.trim().is_empty() => init_with_otlp(&endpoint, filter),
        _ => tracing_subscriber::fmt().with_env_filter(filter).init(),
    }
}

#[cfg(feature = "otlp")]
fn init_with_otlp(endpoint: &str, filter: tracing_subscriber::EnvFilter) {
    use opentelemetry::trace::TracerProvider as _;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(format!(
            "{}/v1/traces",
            endpoint.trim().trim_end_matches('/')
        ))
        .build()
        .expect("build OTLP span exporter");
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(opentelemetry_sdk::Resource::builder().with_service_name(
            std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "medicalos-api".into()),
        ))
        .build();
    let tracer = provider.tracer("telemetry");
    // The batch exporter owns its own flush scheduling; the process-lifetime
    // provider is intentionally never shut down.
    std::mem::forget(provider);
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();
}

#[cfg(not(feature = "otlp"))]
fn init_with_otlp(_endpoint: &str, filter: tracing_subscriber::EnvFilter) {
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
