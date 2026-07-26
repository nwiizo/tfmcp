//! Lightweight in-process metrics with OTel-compatible names and attributes.

use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

static METRICS: LazyLock<Mutex<MetricsStore>> =
    LazyLock::new(|| Mutex::new(MetricsStore::default()));

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MetricSnapshot {
    pub name: String,
    pub description: String,
    pub unit: String,
    pub attributes: BTreeMap<String, String>,
    pub count: u64,
    pub sum: f64,
    pub max: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MetricKey {
    name: String,
    unit: String,
    description: String,
    attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
struct MetricValue {
    count: u64,
    sum: f64,
    max: f64,
}

#[derive(Default)]
struct MetricsStore {
    values: BTreeMap<MetricKey, MetricValue>,
}

pub fn record_http_request(
    method: &str,
    route: &str,
    status: u16,
    duration: Duration,
    request_size: Option<u64>,
    response_size: Option<u64>,
) {
    let attributes = BTreeMap::from([
        ("http.request.method".to_string(), method.to_string()),
        ("http.route".to_string(), route.to_string()),
        ("http.response.status_code".to_string(), status.to_string()),
    ]);
    record(
        "http.server.request.duration",
        "s",
        "HTTP server request duration",
        attributes.clone(),
        duration.as_secs_f64(),
    );
    if let Some(size) = request_size {
        record(
            "http.server.request.body.size",
            "By",
            "HTTP server request body size",
            attributes.clone(),
            size as f64,
        );
    }
    if let Some(size) = response_size {
        record(
            "http.server.response.body.size",
            "By",
            "HTTP server response body size",
            attributes,
            size as f64,
        );
    }
}

pub fn record_tool_call(tool_name: &str, success: bool, duration: Duration) {
    let attributes = BTreeMap::from([
        ("mcp.tool.name".to_string(), tool_name.to_string()),
        ("mcp.tool.success".to_string(), success.to_string()),
    ]);
    record(
        "mcp.server.tool.call.count",
        "{call}",
        "MCP tool call count",
        attributes.clone(),
        1.0,
    );
    record(
        "mcp.server.tool.call.duration",
        "s",
        "MCP tool call duration",
        attributes.clone(),
        duration.as_secs_f64(),
    );
    if !success {
        record(
            "mcp.server.tool.call.errors",
            "{error}",
            "MCP tool call error count",
            attributes,
            1.0,
        );
    }
}

pub fn snapshot() -> Vec<MetricSnapshot> {
    let store = METRICS.lock().expect("metrics mutex poisoned");
    store
        .values
        .iter()
        .map(|(key, value)| MetricSnapshot {
            name: key.name.clone(),
            description: key.description.clone(),
            unit: key.unit.clone(),
            attributes: key.attributes.clone(),
            count: value.count,
            sum: value.sum,
            max: value.max,
        })
        .collect()
}

#[cfg(test)]
pub fn reset_for_tests() {
    METRICS
        .lock()
        .expect("metrics mutex poisoned")
        .values
        .clear();
}

fn record(
    name: &str,
    unit: &str,
    description: &str,
    attributes: BTreeMap<String, String>,
    value: f64,
) {
    let key = MetricKey {
        name: name.to_string(),
        unit: unit.to_string(),
        description: description.to_string(),
        attributes,
    };
    let mut store = METRICS.lock().expect("metrics mutex poisoned");
    let metric = store.values.entry(key).or_default();
    metric.count += 1;
    metric.sum += value;
    metric.max = metric.max.max(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_tool_error_metric() {
        reset_for_tests();

        record_tool_call("create_workspace", false, Duration::from_millis(25));
        let metrics = snapshot();

        assert!(
            metrics
                .iter()
                .any(|metric| metric.name == "mcp.server.tool.call.errors")
        );
        assert!(
            metrics
                .iter()
                .any(|metric| metric.name == "mcp.server.tool.call.duration")
        );
    }
}
