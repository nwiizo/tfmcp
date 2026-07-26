//! Deployment controls for remote MCP operation.

use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const MAX_TRACKED_SESSIONS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TlsConfig {
    pub cert_file: PathBuf,
    pub key_file: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DeploymentControls {
    pub organization_allowlist: Vec<String>,
    pub rate_limit_global: Option<u32>,
    pub rate_limit_session: Option<u32>,
    pub tls: Option<TlsConfig>,
}

#[derive(Clone)]
pub struct RateLimiter {
    global_limit: Option<u32>,
    session_limit: Option<u32>,
    state: Arc<Mutex<RateLimitState>>,
}

#[derive(Debug, Default)]
struct RateLimitState {
    global: WindowCounter,
    sessions: HashMap<String, WindowCounter>,
}

#[derive(Debug)]
struct WindowCounter {
    window_started: Instant,
    count: u32,
}

impl Default for WindowCounter {
    fn default() -> Self {
        Self {
            window_started: Instant::now(),
            count: 0,
        }
    }
}

impl DeploymentControls {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup<F>(lookup: F) -> anyhow::Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let tls = parse_tls_config(
            lookup("MCP_TLS_CERT_FILE").map(PathBuf::from),
            lookup("MCP_TLS_KEY_FILE").map(PathBuf::from),
        )?;

        Ok(Self {
            organization_allowlist: parse_csv(lookup("MCP_ORGANIZATION_ALLOWLIST")),
            rate_limit_global: parse_optional_u32("MCP_RATE_LIMIT_GLOBAL", &lookup)?,
            rate_limit_session: parse_optional_u32("MCP_RATE_LIMIT_SESSION", &lookup)?,
            tls,
        })
    }

    pub fn organization_allowed(&self, organization: &str) -> bool {
        self.organization_allowlist.is_empty()
            || self
                .organization_allowlist
                .iter()
                .any(|allowed| allowed == organization)
    }
}

impl RateLimiter {
    pub fn new(controls: &DeploymentControls) -> Self {
        Self {
            global_limit: controls.rate_limit_global,
            session_limit: controls.rate_limit_session,
            state: Arc::new(Mutex::new(RateLimitState::default())),
        }
    }

    pub fn check(&self, session_id: Option<&str>) -> Result<(), RateLimitError> {
        let mut state = self.state.lock().expect("rate limit state mutex poisoned");
        if let Some(limit) = self.global_limit {
            increment_counter(&mut state.global, limit).map_err(|_| RateLimitError::Global)?;
        }
        if let (Some(limit), Some(session_id)) = (self.session_limit, session_id) {
            state
                .sessions
                .retain(|_, counter| counter.window_started.elapsed() < RATE_LIMIT_WINDOW);
            if !state.sessions.contains_key(session_id)
                && state.sessions.len() >= MAX_TRACKED_SESSIONS
            {
                return Err(RateLimitError::Session);
            }
            let counter = state.sessions.entry(session_id.to_string()).or_default();
            increment_counter(counter, limit).map_err(|_| RateLimitError::Session)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitError {
    Global,
    Session,
}

impl RateLimitError {
    pub fn status_message(self) -> &'static str {
        match self {
            Self::Global => "global MCP rate limit exceeded",
            Self::Session => "session MCP rate limit exceeded",
        }
    }
}

fn increment_counter(counter: &mut WindowCounter, limit: u32) -> Result<(), ()> {
    if limit == 0 {
        return Ok(());
    }
    if counter.window_started.elapsed() >= RATE_LIMIT_WINDOW {
        counter.window_started = Instant::now();
        counter.count = 0;
    }
    if counter.count >= limit {
        return Err(());
    }
    counter.count += 1;
    Ok(())
}

fn parse_optional_u32<F>(name: &str, lookup: &F) -> anyhow::Result<Option<u32>>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(name)
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            value
                .trim()
                .parse::<u32>()
                .map_err(|e| anyhow::anyhow!("invalid {name} '{}': {e}", value.trim()))
        })
        .transpose()
}

fn parse_tls_config(
    cert_file: Option<PathBuf>,
    key_file: Option<PathBuf>,
) -> anyhow::Result<Option<TlsConfig>> {
    match (cert_file, key_file) {
        (None, None) => Ok(None),
        (Some(cert_file), Some(key_file)) => Ok(Some(TlsConfig {
            cert_file,
            key_file,
        })),
        (Some(_), None) => {
            anyhow::bail!("MCP_TLS_KEY_FILE is required when MCP_TLS_CERT_FILE is set")
        }
        (None, Some(_)) => {
            anyhow::bail!("MCP_TLS_CERT_FILE is required when MCP_TLS_KEY_FILE is set")
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_deployment_controls() {
        let vars = HashMap::from([
            (
                "MCP_ORGANIZATION_ALLOWLIST".to_string(),
                "org-a, org-b".to_string(),
            ),
            ("MCP_RATE_LIMIT_GLOBAL".to_string(), "10".to_string()),
            ("MCP_RATE_LIMIT_SESSION".to_string(), "3".to_string()),
            ("MCP_TLS_CERT_FILE".to_string(), "/tmp/cert.pem".to_string()),
            ("MCP_TLS_KEY_FILE".to_string(), "/tmp/key.pem".to_string()),
        ]);

        let controls = DeploymentControls::from_lookup(|key| vars.get(key).cloned()).unwrap();

        assert!(controls.organization_allowed("org-a"));
        assert!(!controls.organization_allowed("org-c"));
        assert_eq!(controls.rate_limit_global, Some(10));
        assert_eq!(controls.rate_limit_session, Some(3));
        assert_eq!(
            controls.tls.as_ref().map(|tls| tls.cert_file.as_path()),
            Some(std::path::Path::new("/tmp/cert.pem"))
        );
    }

    #[test]
    fn rate_limiter_enforces_global_limit() {
        let limiter = RateLimiter::new(&DeploymentControls {
            rate_limit_global: Some(1),
            ..DeploymentControls::default()
        });

        assert!(limiter.check(None).is_ok());
        assert_eq!(limiter.check(None), Err(RateLimitError::Global));
    }

    #[test]
    fn rate_limiter_prunes_expired_sessions() {
        let limiter = RateLimiter::new(&DeploymentControls {
            rate_limit_session: Some(1),
            ..DeploymentControls::default()
        });
        {
            let mut state = limiter.state.lock().unwrap();
            state.sessions.insert(
                "expired".to_string(),
                WindowCounter {
                    window_started: Instant::now() - RATE_LIMIT_WINDOW,
                    count: 1,
                },
            );
        }

        assert!(limiter.check(Some("current")).is_ok());
        let state = limiter.state.lock().unwrap();
        assert!(!state.sessions.contains_key("expired"));
        assert!(state.sessions.contains_key("current"));
    }
}
