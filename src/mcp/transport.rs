//! MCP transport configuration and HTTP helpers.

use crate::mcp::deployment::DeploymentControls;
use axum::Router;
use axum::http::{HeaderValue, Method};
use rmcp::transport::streamable_http_server::StreamableHttpServerConfig;
use serde::Serialize;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};

const DEFAULT_TRANSPORT_MODE: &str = "stdio";
const DEFAULT_TRANSPORT_HOST: &str = "127.0.0.1";
const DEFAULT_TRANSPORT_PORT: u16 = 8080;
const DEFAULT_MCP_ENDPOINT: &str = "/mcp";
const DEFAULT_HEALTH_ENDPOINT: &str = "/health";
const DEFAULT_METRICS_ENDPOINT: &str = "/metrics";
const DEFAULT_ALLOWED_ORIGINS: &[&str] = &[
    "http://localhost",
    "https://localhost",
    "http://127.0.0.1",
    "https://127.0.0.1",
    "http://[::1]",
    "https://[::1]",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportMode {
    Stdio,
    StreamableHttp,
}

impl TransportMode {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::parse(
            &std::env::var("TRANSPORT_MODE").unwrap_or_else(|_| DEFAULT_TRANSPORT_MODE.to_string()),
        )
    }

    pub fn parse(value: &str) -> anyhow::Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "stdio" => Ok(Self::Stdio),
            "streamable-http" | "http" => Ok(Self::StreamableHttp),
            other => anyhow::bail!(
                "unsupported TRANSPORT_MODE '{other}', expected 'stdio' or 'streamable-http'"
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HttpSessionMode {
    Stateful,
    Stateless,
}

impl HttpSessionMode {
    fn is_stateful(self) -> bool {
        self == Self::Stateful
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CorsMode {
    Strict,
    Development,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpTransportConfig {
    pub host: String,
    pub port: u16,
    pub endpoint: String,
    pub health_endpoint: String,
    pub metrics_endpoint: String,
    pub cors_mode: CorsMode,
    pub allowed_origins: Vec<String>,
    pub allowed_hosts: Vec<String>,
    pub heartbeat_interval_secs: Option<u64>,
    pub session_mode: HttpSessionMode,
    pub deployment: DeploymentControls,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub server: &'static str,
    pub version: &'static str,
    pub transport: TransportMode,
    pub endpoint: String,
    pub metrics_endpoint: String,
    pub session_mode: HttpSessionMode,
    pub organization_allowlist_enabled: bool,
    pub rate_limit_global: Option<u32>,
    pub rate_limit_session: Option<u32>,
    pub tls_enabled: bool,
    pub allowed_hosts_enabled: bool,
    pub origin_validation_enabled: bool,
    pub heartbeat_interval_seconds: Option<u64>,
}

impl HttpTransportConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    fn from_lookup<F>(lookup: F) -> anyhow::Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let host = lookup("TRANSPORT_HOST").unwrap_or_else(|| DEFAULT_TRANSPORT_HOST.to_string());
        let port = parse_port(lookup("TRANSPORT_PORT"))?;
        let endpoint = normalize_endpoint(
            lookup("MCP_ENDPOINT").unwrap_or_else(|| DEFAULT_MCP_ENDPOINT.to_string()),
            "MCP_ENDPOINT",
        )?;
        let health_endpoint = normalize_endpoint(
            lookup("MCP_HEALTH_ENDPOINT").unwrap_or_else(|| DEFAULT_HEALTH_ENDPOINT.to_string()),
            "MCP_HEALTH_ENDPOINT",
        )?;
        let metrics_endpoint = normalize_endpoint(
            lookup("MCP_METRICS_ENDPOINT").unwrap_or_else(|| DEFAULT_METRICS_ENDPOINT.to_string()),
            "MCP_METRICS_ENDPOINT",
        )?;
        let cors_mode = parse_choice(
            "MCP_CORS_MODE",
            lookup("MCP_CORS_MODE"),
            "strict",
            &[
                ("", CorsMode::Strict),
                ("strict", CorsMode::Strict),
                ("development", CorsMode::Development),
                ("dev", CorsMode::Development),
                ("disabled", CorsMode::Disabled),
                ("off", CorsMode::Disabled),
                ("none", CorsMode::Disabled),
            ],
        )?;
        let allowed_origins = parse_csv(lookup("MCP_ALLOWED_ORIGINS"));
        let allowed_hosts = parse_csv(lookup("MCP_ALLOWED_HOSTS"));
        let heartbeat_interval_secs = parse_heartbeat_interval(lookup("MCP_HEARTBEAT_INTERVAL"))?;
        let session_mode = parse_choice(
            "MCP_SESSION_MODE",
            lookup("MCP_SESSION_MODE"),
            "stateful",
            &[
                ("", HttpSessionMode::Stateful),
                ("stateful", HttpSessionMode::Stateful),
                ("stateless", HttpSessionMode::Stateless),
            ],
        )?;
        let deployment = DeploymentControls::from_lookup(&lookup)?;

        Ok(Self {
            host,
            port,
            endpoint,
            health_endpoint,
            metrics_endpoint,
            cors_mode,
            allowed_origins,
            allowed_hosts,
            heartbeat_interval_secs,
            session_mode,
            deployment,
        })
    }

    pub fn socket_addr(&self) -> anyhow::Result<SocketAddr> {
        let ip: IpAddr = self
            .host
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid TRANSPORT_HOST '{}': {e}", self.host))?;
        Ok(SocketAddr::new(ip, self.port))
    }

    pub fn streamable_http_config(&self) -> StreamableHttpServerConfig {
        let mut config = StreamableHttpServerConfig::default()
            .with_legacy_session_mode(self.session_mode.is_stateful())
            .with_json_response(!self.session_mode.is_stateful())
            .with_sse_keep_alive(self.heartbeat_interval_secs.map(Duration::from_secs));
        if !self.allowed_hosts.is_empty() {
            config = config.with_allowed_hosts(self.allowed_hosts.clone());
        }
        config = config.with_allowed_origins(self.effective_allowed_origins());
        config
    }

    fn effective_allowed_origins(&self) -> Vec<String> {
        if self.allowed_origins.is_empty() {
            DEFAULT_ALLOWED_ORIGINS
                .iter()
                .map(|origin| (*origin).to_string())
                .collect()
        } else {
            self.allowed_origins.clone()
        }
    }

    pub fn health_response(&self) -> HealthResponse {
        HealthResponse {
            status: "ok",
            server: "tfmcp",
            version: env!("CARGO_PKG_VERSION"),
            transport: TransportMode::StreamableHttp,
            endpoint: self.endpoint.clone(),
            metrics_endpoint: self.metrics_endpoint.clone(),
            session_mode: self.session_mode,
            organization_allowlist_enabled: !self.deployment.organization_allowlist.is_empty(),
            rate_limit_global: self.deployment.rate_limit_global,
            rate_limit_session: self.deployment.rate_limit_session,
            tls_enabled: self.deployment.tls.is_some(),
            allowed_hosts_enabled: !self.allowed_hosts.is_empty(),
            origin_validation_enabled: true,
            heartbeat_interval_seconds: self.heartbeat_interval_secs,
        }
    }

    pub fn apply_cors(&self, router: Router) -> anyhow::Result<Router> {
        let Some(layer) = self.cors_layer()? else {
            return Ok(router);
        };
        Ok(router.layer(layer))
    }

    fn cors_layer(&self) -> anyhow::Result<Option<CorsLayer>> {
        let layer = CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers(Any);

        match self.cors_mode {
            CorsMode::Disabled => Ok(None),
            CorsMode::Development => Ok(Some(layer.allow_origin(Any))),
            CorsMode::Strict => {
                let origins = self
                    .effective_allowed_origins()
                    .into_iter()
                    .map(|origin| HeaderValue::from_str(&origin))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| anyhow::anyhow!("invalid MCP_ALLOWED_ORIGINS value: {e}"))?;
                Ok(Some(layer.allow_origin(origins)))
            }
        }
    }
}

fn parse_port(value: Option<String>) -> anyhow::Result<u16> {
    let Some(raw) = value else {
        return Ok(DEFAULT_TRANSPORT_PORT);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(DEFAULT_TRANSPORT_PORT);
    }
    trimmed
        .parse::<u16>()
        .map_err(|e| anyhow::anyhow!("invalid TRANSPORT_PORT '{raw}': {e}"))
}

fn parse_choice<T: Copy>(
    name: &str,
    value: Option<String>,
    default: &str,
    choices: &[(&str, T)],
) -> anyhow::Result<T> {
    let raw = value.unwrap_or_else(|| default.to_string());
    let normalized = raw.trim().to_ascii_lowercase();
    choices
        .iter()
        .find_map(|(candidate, parsed)| (*candidate == normalized).then_some(*parsed))
        .ok_or_else(|| anyhow::anyhow!("unsupported {name} '{}'", raw.trim()))
}

fn normalize_endpoint(value: String, name: &str) -> anyhow::Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{name} cannot be empty");
    }
    if !trimmed.starts_with('/') {
        anyhow::bail!("{name} must start with '/'");
    }
    Ok(trimmed.trim_end_matches('/').to_string())
}

fn parse_csv(value: Option<String>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn parse_heartbeat_interval(value: Option<String>) -> anyhow::Result<Option<u64>> {
    let Some(raw) = value else {
        return Ok(Some(15));
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Some(15));
    }
    let seconds = trimmed
        .parse::<u64>()
        .map_err(|e| anyhow::anyhow!("invalid MCP_HEARTBEAT_INTERVAL '{raw}': {e}"))?;
    Ok((seconds > 0).then_some(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn transport_mode_defaults_to_stdio() {
        assert_eq!(TransportMode::parse("").unwrap(), TransportMode::Stdio);
        assert_eq!(
            TransportMode::parse("streamable-http").unwrap(),
            TransportMode::StreamableHttp
        );
    }

    #[test]
    fn http_config_uses_safe_defaults() {
        let vars = HashMap::<String, String>::new();
        let config = HttpTransportConfig::from_lookup(|key| vars.get(key).cloned()).unwrap();

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.endpoint, "/mcp");
        assert_eq!(config.health_endpoint, "/health");
        assert_eq!(config.metrics_endpoint, "/metrics");
        assert_eq!(config.cors_mode, CorsMode::Strict);
        assert!(config.allowed_hosts.is_empty());
        assert_eq!(config.heartbeat_interval_secs, Some(15));
        assert_eq!(config.session_mode, HttpSessionMode::Stateful);
        assert!(config.deployment.organization_allowlist.is_empty());
        assert_eq!(config.deployment.rate_limit_global, None);
        let rmcp_config = config.streamable_http_config();
        assert!(
            rmcp_config
                .allowed_origins
                .contains(&"http://localhost".to_string())
        );
        assert!(
            !rmcp_config
                .allowed_origins
                .contains(&"https://attacker.example".to_string())
        );
    }

    #[test]
    fn http_config_parses_remote_settings() {
        let vars = HashMap::from([
            ("TRANSPORT_HOST".to_string(), "0.0.0.0".to_string()),
            ("TRANSPORT_PORT".to_string(), "9090".to_string()),
            ("MCP_ENDPOINT".to_string(), "/terraform-mcp".to_string()),
            ("MCP_CORS_MODE".to_string(), "development".to_string()),
            (
                "MCP_ALLOWED_HOSTS".to_string(),
                "mcp.example.com,localhost".to_string(),
            ),
            ("MCP_HEARTBEAT_INTERVAL".to_string(), "0".to_string()),
            ("MCP_SESSION_MODE".to_string(), "stateless".to_string()),
            (
                "MCP_ORGANIZATION_ALLOWLIST".to_string(),
                "org-a,org-b".to_string(),
            ),
            ("MCP_RATE_LIMIT_GLOBAL".to_string(), "100".to_string()),
        ]);

        let config = HttpTransportConfig::from_lookup(|key| vars.get(key).cloned()).unwrap();

        assert_eq!(config.socket_addr().unwrap().to_string(), "0.0.0.0:9090");
        assert_eq!(config.endpoint, "/terraform-mcp");
        assert_eq!(config.cors_mode, CorsMode::Development);
        assert_eq!(config.allowed_hosts, vec!["mcp.example.com", "localhost"]);
        assert_eq!(config.heartbeat_interval_secs, None);
        assert_eq!(config.session_mode, HttpSessionMode::Stateless);
        assert!(config.deployment.organization_allowed("org-a"));
        assert_eq!(config.deployment.rate_limit_global, Some(100));
        assert!(!config.streamable_http_config().legacy_session_mode);
        assert!(config.streamable_http_config().json_response);
    }

    #[test]
    fn endpoint_must_start_with_slash() {
        let vars = HashMap::from([("MCP_ENDPOINT".to_string(), "mcp".to_string())]);
        let error = HttpTransportConfig::from_lookup(|key| vars.get(key).cloned())
            .expect_err("missing leading slash should fail");

        assert!(error.to_string().contains("MCP_ENDPOINT"));
    }
}
