use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;

pub fn init_metrics() {
    // Initialize Prometheus recorder
    // By default, it listens on 0.0.0.0:9000, but we can customize it.
    // We'll use port 9100 which is standard for node_exporter/prometheus metrics.
    let addr: SocketAddr = "0.0.0.0:9100".parse().expect("Invalid metrics address");

    let builder = PrometheusBuilder::new().with_http_listener(addr);

    builder
        .install()
        .expect("Failed to install Prometheus recorder");
}

// Helper constants for metric names
pub const METRIC_CONNECTIONS_TOTAL: &str = "hexagondb_connections_total";
pub const METRIC_COMMANDS_TOTAL: &str = "hexagondb_commands_total";
pub const METRIC_COMMAND_LATENCY: &str = "hexagondb_command_latency_seconds";
pub const METRIC_ACTIVE_CONNECTIONS: &str = "hexagondb_active_connections";
pub const METRIC_KEYS_TOTAL: &str = "hexagondb_keys_total";
