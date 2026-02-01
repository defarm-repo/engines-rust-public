use lazy_static::lazy_static;
use prometheus::{
    register_counter, register_gauge, register_histogram, Counter, Encoder, Gauge, Histogram,
    TextEncoder,
};

lazy_static! {
    // ====================
    // ITEMS ENGINE METRICS
    // ====================
    pub static ref ITEMS_CREATED_TOTAL: Counter = register_counter!(
        "items_created_total",
        "Total number of items created"
    )
    .expect("metric can be created");

    pub static ref ITEMS_ENRICHED_TOTAL: Counter = register_counter!(
        "items_enriched_total",
        "Total number of items enriched with additional data"
    )
    .expect("metric can be created");

    pub static ref ITEMS_MERGED_TOTAL: Counter = register_counter!(
        "items_merged_total",
        "Total number of items merged"
    )
    .expect("metric can be created");

    pub static ref ITEMS_ACTIVE: Gauge = register_gauge!(
        "items_active",
        "Current number of active items in storage"
    )
    .expect("metric can be created");

    // ====================
    // CIRCUITS ENGINE METRICS
    // ====================
    pub static ref CIRCUITS_CREATED_TOTAL: Counter = register_counter!(
        "circuits_created_total",
        "Total number of circuits created"
    )
    .expect("metric can be created");

    pub static ref CIRCUIT_PUSHES_TOTAL: Counter = register_counter!(
        "circuit_pushes_total",
        "Total number of items pushed to circuits"
    )
    .expect("metric can be created");

    pub static ref CIRCUIT_PUSHES_FAILED: Counter = register_counter!(
        "circuit_pushes_failed",
        "Total number of failed circuit push operations"
    )
    .expect("metric can be created");

    pub static ref CIRCUIT_UPLOAD_FAILURES: Counter = register_counter!(
        "circuit_upload_failures",
        "Total number of adapter upload failures during circuit push"
    )
    .expect("metric can be created");

    pub static ref CIRCUITS_ACTIVE: Gauge = register_gauge!(
        "circuits_active",
        "Current number of active circuits"
    )
    .expect("metric can be created");

    // ====================
    // EVENTS ENGINE METRICS
    // ====================
    pub static ref EVENTS_CREATED_TOTAL: Counter = register_counter!(
        "events_created_total",
        "Total number of events created"
    )
    .expect("metric can be created");

    pub static ref EVENTS_DEDUPLICATED: Counter = register_counter!(
        "events_deduplicated",
        "Total number of events deduplicated (already existed)"
    )
    .expect("metric can be created");

    // ====================
    // HTTP REQUEST METRICS
    // ====================
    pub static ref HTTP_REQUESTS_TOTAL: Counter = register_counter!(
        "http_requests_total",
        "Total HTTP requests received"
    )
    .expect("metric can be created");

    pub static ref HTTP_REQUESTS_DURATION: Histogram = register_histogram!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds"
    )
    .expect("metric can be created");

    // ====================
    // EXTERNAL SERVICE METRICS
    // ====================
    pub static ref DFID_SERVICE_CALLS_TOTAL: Counter = register_counter!(
        "dfid_service_calls_total",
        "Total calls to DFID Service"
    )
    .expect("metric can be created");

    pub static ref DFID_SERVICE_FAILURES: Counter = register_counter!(
        "dfid_service_failures",
        "Total DFID Service call failures"
    )
    .expect("metric can be created");

    pub static ref INDEX_SERVICE_CALLS_TOTAL: Counter = register_counter!(
        "index_service_calls_total",
        "Total calls to Index Service"
    )
    .expect("metric can be created");

    pub static ref INDEX_SERVICE_FAILURES: Counter = register_counter!(
        "index_service_failures",
        "Total Index Service call failures"
    )
    .expect("metric can be created");

    pub static ref INDEX_RETRY_QUEUE_SIZE: Gauge = register_gauge!(
        "index_retry_queue_size",
        "Current size of Index Service retry queue"
    )
    .expect("metric can be created");

    // ====================
    // ADAPTER METRICS
    // ====================
    pub static ref ADAPTER_UPLOADS_TOTAL: Counter = register_counter!(
        "adapter_uploads_total",
        "Total adapter upload attempts"
    )
    .expect("metric can be created");

    pub static ref ADAPTER_UPLOADS_SUCCESS: Counter = register_counter!(
        "adapter_uploads_success",
        "Total successful adapter uploads"
    )
    .expect("metric can be created");

    pub static ref ADAPTER_UPLOADS_FAILED: Counter = register_counter!(
        "adapter_uploads_failed",
        "Total failed adapter uploads"
    )
    .expect("metric can be created");
}

/// Initialize all metrics (called at startup)
pub fn init_metrics() {
    lazy_static::initialize(&ITEMS_CREATED_TOTAL);
    lazy_static::initialize(&ITEMS_ENRICHED_TOTAL);
    lazy_static::initialize(&ITEMS_MERGED_TOTAL);
    lazy_static::initialize(&ITEMS_ACTIVE);

    lazy_static::initialize(&CIRCUITS_CREATED_TOTAL);
    lazy_static::initialize(&CIRCUIT_PUSHES_TOTAL);
    lazy_static::initialize(&CIRCUIT_PUSHES_FAILED);
    lazy_static::initialize(&CIRCUIT_UPLOAD_FAILURES);
    lazy_static::initialize(&CIRCUITS_ACTIVE);

    lazy_static::initialize(&EVENTS_CREATED_TOTAL);
    lazy_static::initialize(&EVENTS_DEDUPLICATED);

    lazy_static::initialize(&HTTP_REQUESTS_TOTAL);
    lazy_static::initialize(&HTTP_REQUESTS_DURATION);

    lazy_static::initialize(&DFID_SERVICE_CALLS_TOTAL);
    lazy_static::initialize(&DFID_SERVICE_FAILURES);
    lazy_static::initialize(&INDEX_SERVICE_CALLS_TOTAL);
    lazy_static::initialize(&INDEX_SERVICE_FAILURES);
    lazy_static::initialize(&INDEX_RETRY_QUEUE_SIZE);

    lazy_static::initialize(&ADAPTER_UPLOADS_TOTAL);
    lazy_static::initialize(&ADAPTER_UPLOADS_SUCCESS);
    lazy_static::initialize(&ADAPTER_UPLOADS_FAILED);
}

/// Encode all metrics to Prometheus text format
pub fn encode_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();

    encoder
        .encode(&metric_families, &mut buffer)
        .expect("Failed to encode metrics");

    String::from_utf8(buffer).expect("Metrics should be valid UTF-8")
}
