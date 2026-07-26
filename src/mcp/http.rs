//! Streamable HTTP serving support for the MCP server.

use crate::core::tfmcp::TfMcp;
use crate::mcp::deployment::{RateLimitError, RateLimiter};
use crate::mcp::server::{TfMcpServer, ToolFilter};
use crate::mcp::transport::HttpTransportConfig;
use crate::shared::logging;
use crate::shared::metrics;
use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use std::time::Instant;

#[derive(Clone)]
struct HttpMetricRoutes {
    mcp: String,
    health: String,
    metrics: String,
}

impl TfMcpServer {
    /// Serve the MCP server over Streamable HTTP with health and CORS controls.
    pub async fn serve_streamable_http(
        tfmcp: TfMcp,
        tool_filter: ToolFilter,
        config: HttpTransportConfig,
    ) -> anyhow::Result<()> {
        let addr = config.socket_addr()?;
        let rmcp_config = config.streamable_http_config();
        let shutdown_token = rmcp_config.cancellation_token.clone();
        let server = Self::new(tfmcp, tool_filter);
        let router = Self::streamable_http_router_with_config(server, &config, rmcp_config)?;
        let listener = tokio::net::TcpListener::bind(addr).await?;

        logging::info(&format!(
            "Starting tfmcp MCP server via streamable HTTP at {}://{}{}",
            if config.deployment.tls.is_some() {
                "https"
            } else {
                "http"
            },
            addr,
            config.endpoint
        ));

        if let Some(tls) = &config.deployment.tls {
            drop(listener);
            let rustls =
                axum_server::tls_rustls::RustlsConfig::from_pem_file(&tls.cert_file, &tls.key_file)
                    .await?;
            axum_server::bind_rustls(addr, rustls)
                .serve(router.into_make_service())
                .await?;
        } else {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    if tokio::signal::ctrl_c().await.is_ok() {
                        shutdown_token.cancel();
                    }
                })
                .await?;
        }

        Ok(())
    }

    /// Build the Streamable HTTP router used by the server and transport tests.
    pub fn streamable_http_router(
        server: Self,
        config: &HttpTransportConfig,
    ) -> anyhow::Result<Router> {
        Self::streamable_http_router_with_config(server, config, config.streamable_http_config())
    }

    fn streamable_http_router_with_config(
        server: Self,
        config: &HttpTransportConfig,
        rmcp_config: StreamableHttpServerConfig,
    ) -> anyhow::Result<Router> {
        let server = server.with_deployment_controls(config.deployment.clone());
        let service: StreamableHttpService<Self, LocalSessionManager> =
            StreamableHttpService::new(move || Ok(server.clone()), Default::default(), rmcp_config);
        let rate_limiter = RateLimiter::new(&config.deployment);
        let metric_routes = HttpMetricRoutes {
            mcp: config.endpoint.clone(),
            health: config.health_endpoint.clone(),
            metrics: config.metrics_endpoint.clone(),
        };

        let health = config.health_response();
        let health_route = {
            let health = health.clone();
            get(move || {
                let health = health.clone();
                async move { Json(health) }
            })
        };

        let router = Router::new()
            .route(&config.health_endpoint, health_route)
            .route(&config.metrics_endpoint, get(metrics_handler))
            .nest_service(&config.endpoint, service)
            .layer(middleware::from_fn_with_state(
                rate_limiter,
                enforce_rate_limit,
            ))
            .layer(middleware::from_fn_with_state(
                metric_routes,
                record_http_metrics,
            ));
        config.apply_cors(router)
    }
}

async fn metrics_handler() -> Json<Vec<metrics::MetricSnapshot>> {
    Json(metrics::snapshot())
}

async fn record_http_metrics(
    State(routes): State<HttpMetricRoutes>,
    request: Request,
    next: Next,
) -> Response {
    let method = match *request.method() {
        axum::http::Method::GET => "GET",
        axum::http::Method::POST => "POST",
        axum::http::Method::DELETE => "DELETE",
        axum::http::Method::OPTIONS => "OPTIONS",
        _ => "OTHER",
    };
    let route = routes.metric_route(request.uri().path());
    let request_size = content_length(request.headers());
    let started = Instant::now();
    let response = next.run(request).await;
    let status = response.status().as_u16();
    let response_size = content_length(response.headers());

    metrics::record_http_request(
        method,
        route,
        status,
        started.elapsed(),
        request_size,
        response_size,
    );

    response
}

impl HttpMetricRoutes {
    fn metric_route(&self, path: &str) -> &str {
        if path == self.mcp {
            &self.mcp
        } else if path == self.health {
            &self.health
        } else if path == self.metrics {
            &self.metrics
        } else {
            "unmatched"
        }
    }
}

fn content_length(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
}

async fn enforce_rate_limit(
    State(rate_limiter): State<RateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let session_id = request
        .headers()
        .get("mcp-session-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    match rate_limiter.check(session_id.as_deref()) {
        Ok(()) => next.run(request).await,
        Err(error) => rate_limit_response(error),
    }
}

fn rate_limit_response(error: RateLimitError) -> Response {
    Response::builder()
        .status(StatusCode::TOO_MANY_REQUESTS)
        .body(axum::body::Body::from(error.status_message()))
        .expect("valid rate limit response")
}
