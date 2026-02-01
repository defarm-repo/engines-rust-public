use lazy_static::lazy_static;
use prometheus::{IntCounter, IntGauge, Registry, TextEncoder, Encoder};
use std::sync::Arc;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    // Counters
    pub static ref DFIDS_GENERATED_TOTAL: IntCounter = IntCounter::new(
        "dfid_generated_total",
        "Total number of DFIDs generated"
    )
    .expect("metric can be created");

    pub static ref DFIDS_VALIDATED_TOTAL: IntCounter = IntCounter::new(
        "dfid_validated_total",
        "Total number of DFID validations performed"
    )
    .expect("metric can be created");

    pub static ref REQUESTS_TOTAL: IntCounter = IntCounter::new(
        "http_requests_total",
        "Total number of HTTP requests"
    )
    .expect("metric can be created");

    // Gauges
    pub static ref CURRENT_SEQUENCE: IntGauge = IntGauge::new(
        "dfid_current_sequence",
        "Current DFID sequence number"
    )
    .expect("metric can be created");
}

pub fn init_metrics() {
    REGISTRY
        .register(Box::new(DFIDS_GENERATED_TOTAL.clone()))
        .expect("collector can be registered");

    REGISTRY
        .register(Box::new(DFIDS_VALIDATED_TOTAL.clone()))
        .expect("collector can be registered");

    REGISTRY
        .register(Box::new(REQUESTS_TOTAL.clone()))
        .expect("collector can be registered");

    REGISTRY
        .register(Box::new(CURRENT_SEQUENCE.clone()))
        .expect("collector can be registered");
}

pub fn encode_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
