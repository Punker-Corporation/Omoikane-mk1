mod launcher;
mod metrics;
mod routeros;
mod sql;
mod terminal;

use actix_web::{HttpResponse, web};
use daikoku::{DaikokuServer, ServerStatusSnapshot};
use omoikane_control::OmoikaneLaunchConfig;
use std::sync::{Arc, Mutex};

pub use launcher::{parse_launch_args, run_omoikane_server, run_omoikane_server_from_env};
pub use metrics::OmoikaneRuntimeMetrics;
pub use omoikane_control::{
    JunosControlError, JunosDeviceProfile, JunosMcpCatalog, JunosMcpTool, JunosOperation,
    LaunchConfigError, NetworkRobotCheck, NetworkRobotDevice, NetworkRobotPlan,
    NetworkRobotPlanError, OmoikaneLaunchConfig as WebLaunchConfig, OmoikaneLaunchManifest,
    OverlayConfigError, OverlayFixedIpProfile, ServerEndpoint, stable_overlay_ip,
};
pub use routeros::{
    RB2011_ARCHITECTURE, ROUTEROS_STABLE_VERSION, RouterOsRackProfile, RouterOsScriptError,
};
pub use sql::OmoikaneSqlState;
pub use terminal::{render_boot_panel, render_live_panel};

#[derive(Clone)]
pub struct OmoikaneActixState {
    server: Arc<Mutex<DaikokuServer>>,
    launch_manifest: Arc<OmoikaneLaunchManifest>,
    metrics: Arc<OmoikaneRuntimeMetrics>,
    sql: Option<OmoikaneSqlState>,
}

impl OmoikaneActixState {
    pub fn new(server: DaikokuServer) -> Self {
        let snapshot = server.status_snapshot();
        let launch_config = launch_config_from_snapshot(&snapshot);
        Self::with_launch_config(server, launch_config)
    }

    pub fn with_launch_config(server: DaikokuServer, config: OmoikaneLaunchConfig) -> Self {
        let launch_manifest = config
            .build_manifest()
            .expect("default Omoikane launch config must be valid");
        Self::with_launch_manifest(server, launch_manifest)
    }

    pub fn with_launch_manifest(
        server: DaikokuServer,
        launch_manifest: OmoikaneLaunchManifest,
    ) -> Self {
        Self {
            server: Arc::new(Mutex::new(server)),
            launch_manifest: Arc::new(launch_manifest),
            metrics: Arc::new(OmoikaneRuntimeMetrics::default()),
            sql: None,
        }
    }

    pub fn from_shared(server: Arc<Mutex<DaikokuServer>>) -> Self {
        let launch_config = server
            .lock()
            .map(|server| launch_config_from_snapshot(&server.status_snapshot()))
            .unwrap_or_else(|_| OmoikaneLaunchConfig::default());
        let launch_manifest = launch_config
            .build_manifest()
            .expect("default Omoikane launch config must be valid");
        Self::from_shared_with_launch_manifest(server, launch_manifest)
    }

    pub fn from_shared_with_launch_manifest(
        server: Arc<Mutex<DaikokuServer>>,
        launch_manifest: OmoikaneLaunchManifest,
    ) -> Self {
        Self::from_shared_with_runtime(server, launch_manifest, None)
    }

    pub fn from_shared_with_runtime(
        server: Arc<Mutex<DaikokuServer>>,
        launch_manifest: OmoikaneLaunchManifest,
        sql: Option<OmoikaneSqlState>,
    ) -> Self {
        Self {
            server,
            launch_manifest: Arc::new(launch_manifest),
            metrics: Arc::new(OmoikaneRuntimeMetrics::default()),
            sql,
        }
    }

    pub fn shared_server(&self) -> Arc<Mutex<DaikokuServer>> {
        Arc::clone(&self.server)
    }

    pub fn status_snapshot(&self) -> Result<ServerStatusSnapshot, OmoikaneActixError> {
        self.server
            .lock()
            .map(|server| server.status_snapshot())
            .map_err(|_| OmoikaneActixError::ServerLockPoisoned)
    }

    pub fn launch_manifest(&self) -> Arc<OmoikaneLaunchManifest> {
        Arc::clone(&self.launch_manifest)
    }

    pub fn metrics(&self) -> Arc<OmoikaneRuntimeMetrics> {
        Arc::clone(&self.metrics)
    }

    pub fn sql(&self) -> Option<OmoikaneSqlState> {
        self.sql.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmoikaneActixError {
    ServerLockPoisoned,
}

pub fn configure_omoikane_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health))
        .route("/healthz", web::get().to(health))
        .route("/status", web::get().to(status))
        .route("/status", web::head().to(status_head))
        .route("/status.json", web::get().to(status))
        .route("/status.json", web::head().to(status_head))
        .route("/launch", web::get().to(launch_manifest))
        .route("/launch", web::head().to(launch_manifest_head))
        .route("/launch.json", web::get().to(launch_manifest))
        .route("/launch.json", web::head().to(launch_manifest_head))
        .route("/network/overlay", web::get().to(overlay_manifest))
        .route(
            "/network/overlay/server.conf",
            web::get().to(overlay_server_config),
        )
        .route(
            "/network/overlay/peer.conf",
            web::get().to(overlay_peer_config),
        )
        .route("/automation/robot", web::get().to(robot_manifest))
        .route("/automation/junos/tools", web::get().to(junos_tools))
        .route("/automation/junos/rpc/{tool}", web::get().to(junos_rpc))
        .route("/database/status", web::get().to(database_status))
        .route("/metrics", web::get().to(metrics))
        .route("/grafana/dashboard.json", web::get().to(grafana_dashboard));
}

pub async fn health() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"ok":true}"#)
}

pub async fn status(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    match write_status_body(&state) {
        Ok(body) => HttpResponse::Ok()
            .content_type("application/json")
            .body(body),
        Err(error) => status_error_response(error),
    }
}

pub async fn status_head(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    match write_status_body(&state) {
        Ok(body) => HttpResponse::Ok()
            .content_type("application/json")
            .insert_header(("Content-Length", body.len().to_string()))
            .finish(),
        Err(error) => status_error_response(error),
    }
}

pub async fn launch_manifest(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    let body = write_launch_body(&state);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn launch_manifest_head(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    let body = write_launch_body(&state);
    HttpResponse::Ok()
        .content_type("application/json")
        .insert_header(("Content-Length", body.len().to_string()))
        .finish()
}

pub async fn overlay_manifest(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    let manifest = state.launch_manifest();
    let mut body = String::new();
    body.push('{');
    body.push_str("\"overlay\":");
    if let Some(overlay) = &manifest.overlay {
        overlay.write_json(&mut body);
    } else {
        body.push_str("null");
    }
    body.push('}');
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn overlay_server_config(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    let manifest = state.launch_manifest();
    let Some(overlay) = &manifest.overlay else {
        return HttpResponse::NotFound()
            .content_type("application/json")
            .body(r#"{"error":"overlay_disabled"}"#);
    };

    match overlay.render_server_config() {
        Ok(config) => HttpResponse::Ok().content_type("text/plain").body(config),
        Err(_) => HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(r#"{"error":"overlay_config_invalid"}"#),
    }
}

pub async fn overlay_peer_config(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    let manifest = state.launch_manifest();
    let Some(overlay) = &manifest.overlay else {
        return HttpResponse::NotFound()
            .content_type("application/json")
            .body(r#"{"error":"overlay_disabled"}"#);
    };

    match overlay.render_client_peer_config() {
        Ok(config) => HttpResponse::Ok().content_type("text/plain").body(config),
        Err(_) => HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(r#"{"error":"overlay_config_invalid"}"#),
    }
}

pub async fn robot_manifest(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    let manifest = state.launch_manifest();
    let mut body = String::new();
    manifest.robot.write_json(&mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn junos_tools(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    let manifest = state.launch_manifest();
    let mut body = String::new();
    manifest.junos.write_json(&mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn junos_rpc(
    state: web::Data<OmoikaneActixState>,
    tool: web::Path<String>,
) -> HttpResponse {
    state.metrics.record_http_request();
    let manifest = state.launch_manifest();
    match manifest.junos.rpc_for_tool(&tool) {
        Ok(rpc) => HttpResponse::Ok().content_type("application/xml").body(rpc),
        Err(JunosControlError::UnknownTool(_)) => HttpResponse::NotFound()
            .content_type("application/json")
            .body(r#"{"error":"unknown_junos_tool"}"#),
        Err(JunosControlError::WriteBlocked(_)) => HttpResponse::Forbidden()
            .content_type("application/json")
            .body(r#"{"error":"junos_write_blocked"}"#),
        Err(_) => HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(r#"{"error":"junos_catalog_invalid"}"#),
    }
}

pub async fn database_status(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    let Some(sql) = state.sql() else {
        return HttpResponse::Ok()
            .content_type("application/json")
            .body(r#"{"enabled":false,"status":"disabled"}"#);
    };

    state.metrics.record_sql_check();
    match sql.check().await {
        Ok(()) => HttpResponse::Ok()
            .content_type("application/json")
            .body(format!(
                r#"{{"enabled":true,"status":"ok","url":"{}","max_connections":{}}}"#,
                json_escape(sql.redacted_url()),
                sql.max_connections()
            )),
        Err(_) => {
            state.metrics.record_sql_error();
            HttpResponse::ServiceUnavailable()
                .content_type("application/json")
                .body(r#"{"enabled":true,"status":"error"}"#)
        }
    }
}

pub async fn metrics(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    match state.status_snapshot() {
        Ok(snapshot) => {
            let manifest = state.launch_manifest();
            let mut body = String::new();
            state
                .metrics
                .write_prometheus(&snapshot, &manifest, state.sql.is_some(), &mut body);
            HttpResponse::Ok().content_type("text/plain").body(body)
        }
        Err(error) => status_error_response(error),
    }
}

pub async fn grafana_dashboard(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    state.metrics.record_http_request();
    let mut body = String::new();
    state.metrics.write_grafana_dashboard(&mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

fn write_status_body(state: &OmoikaneActixState) -> Result<Vec<u8>, OmoikaneActixError> {
    let snapshot = state.status_snapshot()?;
    let mut body = Vec::with_capacity(512);
    snapshot.write_json(&mut body);
    Ok(body)
}

fn write_launch_body(state: &OmoikaneActixState) -> String {
    let manifest = state.launch_manifest();
    let mut body = String::new();
    manifest.write_json(&mut body);
    body
}

fn status_error_response(error: OmoikaneActixError) -> HttpResponse {
    match error {
        OmoikaneActixError::ServerLockPoisoned => HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(r#"{"error":"server_lock_poisoned"}"#),
    }
}

fn launch_config_from_snapshot(snapshot: &ServerStatusSnapshot) -> OmoikaneLaunchConfig {
    OmoikaneLaunchConfig {
        server_name: snapshot.name.clone(),
        max_players: snapshot.max_players,
        tick_rate: snapshot.tick_rate,
        ..OmoikaneLaunchConfig::default()
    }
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{OmoikaneActixState, configure_omoikane_routes};
    use actix_web::{App, http::StatusCode, test, web};
    use daikoku::{DaikokuServer, ServerOptions};

    fn status_server() -> DaikokuServer {
        let mut server = DaikokuServer::new(ServerOptions {
            server_name: "Omoikane Actix".to_string(),
            max_players: 128,
            tick_rate: 144,
        });
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        server.tick_update(1.0 / 144.0);
        server
    }

    #[test]
    fn actix_routes_serve_daikoku_status() {
        actix_web::rt::System::new().block_on(async {
            let state = web::Data::new(OmoikaneActixState::new(status_server()));
            let app = test::init_service(
                App::new()
                    .app_data(state.clone())
                    .configure(configure_omoikane_routes),
            )
            .await;

            let response =
                test::call_service(&app, test::TestRequest::get().uri("/status").to_request())
                    .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("\"name\":\"Omoikane Actix\""));
            assert!(body.contains("\"tick_rate\":144"));
            assert!(body.contains("\"players\":1"));
        });
    }

    #[test]
    fn actix_routes_serve_health_and_head_without_body() {
        actix_web::rt::System::new().block_on(async {
            let state = web::Data::new(OmoikaneActixState::new(status_server()));
            let app = test::init_service(
                App::new()
                    .app_data(state.clone())
                    .configure(configure_omoikane_routes),
            )
            .await;

            let response =
                test::call_service(&app, test::TestRequest::get().uri("/healthz").to_request())
                    .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            assert_eq!(body, r#"{"ok":true}"#);

            let response = test::call_service(
                &app,
                test::TestRequest::default()
                    .method(actix_web::http::Method::HEAD)
                    .uri("/status.json")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(test::read_body(response).await.len(), 0);
        });
    }

    #[test]
    fn actix_routes_serve_launch_and_overlay_manifests() {
        actix_web::rt::System::new().block_on(async {
            let state = web::Data::new(OmoikaneActixState::new(status_server()));
            let app = test::init_service(
                App::new()
                    .app_data(state.clone())
                    .configure(configure_omoikane_routes),
            )
            .await;

            let response =
                test::call_service(&app, test::TestRequest::get().uri("/launch").to_request())
                    .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("\"server_name\":\"Omoikane Actix\""));
            assert!(body.contains("\"overlay\""));
            assert!(body.contains("\"junos\""));
            assert!(body.contains("\"robot\""));

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/network/overlay/peer.conf")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("[Peer]"));
            assert!(body.contains("AllowedIPs = 100.104."));

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/automation/junos/rpc/junos.interface_terse")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("<get-interface-information>"));

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/automation/junos/rpc/junos.commit_confirmed")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);

            let response =
                test::call_service(&app, test::TestRequest::get().uri("/metrics").to_request())
                    .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("omoikane_http_requests_total"));
            assert!(body.contains("omoikane_machine_parallelism"));

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/grafana/dashboard.json")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("\"title\":\"Omoikane Runtime\""));

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/database/status")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            assert_eq!(body, r#"{"enabled":false,"status":"disabled"}"#);
        });
    }
}
