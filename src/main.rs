use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use axlogger::logger_middleware;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, FromRef, State};
use axum::routing::post;
use axum_client_addr::{ClientIp, ClientIpConfig, ClientIpConfigSource};
use http::{HeaderValue, Method, StatusCode};
use sentry_tunnel::{SentryTunnelConfig, SentryTunnelError, handle_sentry_tunnel};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
struct AppState {
    tunnel_config: Arc<SentryTunnelConfig>,
    ip_config: ClientIpConfig,
}

impl FromRef<AppState> for Arc<SentryTunnelConfig> {
    fn from_ref(app_state: &AppState) -> Arc<SentryTunnelConfig> {
        app_state.tunnel_config.clone()
    }
}

impl ClientIpConfigSource for AppState {
    fn client_ip_config(&self) -> &ClientIpConfig {
        &self.ip_config
    }
}

async fn sentry_tunnel_handler(
    State(tunnel_config): State<Arc<SentryTunnelConfig>>,
    client_ip: ClientIp,
    body: Bytes,
) -> Result<StatusCode, SentryTunnelError> {
    handle_sentry_tunnel(tunnel_config, body, Some(&client_ip.to_string())).await
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let bind_host = env::var("TUNNEL_BIND").unwrap_or("0.0.0.0:8008".to_string());
    let path = env::var("TUNNEL_PATH").unwrap_or("/tunnel".to_string());
    let host = env::var("SENTRY_HOST").expect("SENTRY_HOST must be set");
    let project_ids = env::var("SENTRY_PROJECT_IDS")
        .expect("SENTRY_PROJECT_IDS must be set")
        .clone();
    let project_ids: Vec<_> = project_ids.split(',').map(|s| s.to_owned()).collect();
    let origins = env::var("TUNNEL_ORIGINS").expect("TUNNEL_ORIGINS must be set");
    let origins: Vec<_> = origins
        .split(',')
        .map(|o| o.parse::<HeaderValue>().expect("valid origin"))
        .collect();

    let tunnel_config = SentryTunnelConfig::new(host, project_ids).with_path(path.clone());
    let ip_config = ClientIpConfig::builder().trust_all_proxies().build();
    let cors_middleware = CorsLayer::new()
        .allow_methods([Method::POST])
        .allow_origin(origins);

    let app = Router::new()
        .route(&tunnel_config.path, post(sentry_tunnel_handler))
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(20 * 1024 * 1024))
                .layer(TraceLayer::new_for_http())
                .layer(axum::middleware::from_fn(logger_middleware))
                .layer(cors_middleware),
        )
        .with_state(AppState {
            tunnel_config: Arc::new(tunnel_config),
            ip_config,
        });

    let listener = tokio::net::TcpListener::bind(&bind_host).await.unwrap();
    println!("Listening on {bind_host}; tunnel endpoint: {path}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
