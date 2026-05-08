mod config;
mod console;
mod launcher;
mod metrics;
mod michisuji;
mod monitoring;
mod public_site;
mod security;
mod sql;
mod terminal;

use actix_web::{HttpRequest, HttpResponse, web};
use daikoku::{DaikokuServer, ServerStatusSnapshot};
use omoikane_control::OmoikaneLaunchConfig;
use std::sync::{Arc, Mutex};

pub use config::load_launch_config_file;
pub use launcher::{parse_launch_args, run_omoikane_server, run_omoikane_server_from_env};
pub use metrics::OmoikaneRuntimeMetrics;
pub use michisuji::{
    MICHISUJI_TARGET_VERSION, MichisujiRackProfile, MichisujiScriptError, RB2011_ARCHITECTURE,
};
pub use omoikane_control::{
    KaminariControlError, KaminariDeviceProfile, KaminariMcpCatalog, KaminariMcpTool,
    KaminariOperation, LaunchConfigError, MamoriCheck, MamoriDevice, MamoriPlan, MamoriPlanError,
    OmoikaneLaunchConfig as WebLaunchConfig, OmoikaneLaunchManifest, OmoikanePublicationPlan,
    OmoikaneSubserver, OmoikaneVpsBlueprint, OverlayConfigError, OverlayFixedIpProfile,
    ServerEndpoint, stable_overlay_ip,
};
pub use security::OmoikaneSecurityState;
pub use sql::OmoikaneSqlState;
pub use terminal::{render_boot_panel, render_live_panel};

#[derive(Clone)]
pub struct OmoikaneHayateState {
    server: Arc<Mutex<DaikokuServer>>,
    launch_manifest: Arc<OmoikaneLaunchManifest>,
    metrics: Arc<OmoikaneRuntimeMetrics>,
    security: Arc<OmoikaneSecurityState>,
    sql: Option<OmoikaneSqlState>,
}

impl OmoikaneHayateState {
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
        let security = Arc::new(OmoikaneSecurityState::from_manifest(&launch_manifest));
        Self {
            server: Arc::new(Mutex::new(server)),
            launch_manifest: Arc::new(launch_manifest),
            metrics: Arc::new(OmoikaneRuntimeMetrics::default()),
            security,
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
            security: Arc::new(OmoikaneSecurityState::from_manifest(&launch_manifest)),
            launch_manifest: Arc::new(launch_manifest),
            metrics: Arc::new(OmoikaneRuntimeMetrics::default()),
            sql,
        }
    }

    pub fn shared_server(&self) -> Arc<Mutex<DaikokuServer>> {
        Arc::clone(&self.server)
    }

    pub fn status_snapshot(&self) -> Result<ServerStatusSnapshot, OmoikaneHayateError> {
        self.server
            .lock()
            .map(|server| server.status_snapshot())
            .map_err(|_| OmoikaneHayateError::ServerLockPoisoned)
    }

    pub fn launch_manifest(&self) -> Arc<OmoikaneLaunchManifest> {
        Arc::clone(&self.launch_manifest)
    }

    pub fn metrics(&self) -> Arc<OmoikaneRuntimeMetrics> {
        Arc::clone(&self.metrics)
    }

    pub fn security(&self) -> Arc<OmoikaneSecurityState> {
        Arc::clone(&self.security)
    }

    pub fn sql(&self) -> Option<OmoikaneSqlState> {
        self.sql.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmoikaneHayateError {
    ServerLockPoisoned,
}

pub fn configure_omoikane_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::get().to(console_home))
        .route("/site", web::get().to(public_site_page))
        .route("/console", web::get().to(console_home))
        .route("/health", web::get().to(health))
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
        .route("/network/dns", web::get().to(dns_manifest))
        .route("/network/publication", web::get().to(publication_manifest))
        .route(
            "/network/dns/routeros.rsc",
            web::get().to(routeros_dns_script),
        )
        .route("/network/dns/junos.set", web::get().to(junos_dns_set))
        .route("/servers", web::get().to(subservers_manifest))
        .route("/vps/reality-blueprint", web::get().to(vps_blueprint))
        .route(
            "/network/overlay/server.conf",
            web::get().to(overlay_server_config),
        )
        .route(
            "/network/overlay/peer.conf",
            web::get().to(overlay_peer_config),
        )
        .route("/automation/mamori", web::get().to(mamori_manifest))
        .route("/automation/kaminari/tools", web::get().to(kaminari_tools))
        .route(
            "/automation/kaminari/rpc/{tool}",
            web::get().to(kaminari_rpc),
        )
        .route(
            "/automation/michisuji/rb2011.rsc",
            web::get().to(michisuji_rb2011_script),
        )
        .route("/database/status", web::get().to(database_status))
        .route("/security/status", web::get().to(security_status))
        .route("/security/monitoring", web::get().to(security_monitoring))
        .route("/metrics", web::get().to(metrics))
        .route("/grakane/dashboard.json", web::get().to(grakane_dashboard));
}

pub async fn health() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"ok":true}"#)
}

pub async fn console_home(req: HttpRequest, state: web::Data<OmoikaneHayateState>) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    if public_site::should_serve_public_site(&req, &state.launch_manifest()) {
        return public_site::public_site_response(state.get_ref());
    }
    console::console_response(&req, state.get_ref())
}

pub async fn public_site_page(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    public_site::public_site_response(state.get_ref())
}

pub async fn status(req: HttpRequest, state: web::Data<OmoikaneHayateState>) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    if console::wants_console(&req) {
        return console::console_response(&req, state.get_ref());
    }
    match write_status_body(&state) {
        Ok(body) => HttpResponse::Ok()
            .content_type("application/json")
            .body(body),
        Err(error) => status_error_response(error),
    }
}

pub async fn status_head(req: HttpRequest, state: web::Data<OmoikaneHayateState>) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    match write_status_body(&state) {
        Ok(body) => HttpResponse::Ok()
            .content_type("application/json")
            .insert_header(("Content-Length", body.len().to_string()))
            .finish(),
        Err(error) => status_error_response(error),
    }
}

pub async fn launch_manifest(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    if console::wants_console(&req) {
        return console::console_response(&req, state.get_ref());
    }
    let body = write_launch_body(&state);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn launch_manifest_head(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let body = write_launch_body(&state);
    HttpResponse::Ok()
        .content_type("application/json")
        .insert_header(("Content-Length", body.len().to_string()))
        .finish()
}

pub async fn overlay_manifest(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
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

pub async fn dns_manifest(req: HttpRequest, state: web::Data<OmoikaneHayateState>) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let manifest = state.launch_manifest();
    let mut body = String::new();
    manifest.dns.write_json(&mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn publication_manifest(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let manifest = state.launch_manifest();
    let mut body = String::new();
    manifest.publication.write_json(&mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn subservers_manifest(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let manifest = state.launch_manifest();
    let mut body = String::new();
    manifest.publication.write_subservers_json(&mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn routeros_dns_script(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let manifest = state.launch_manifest();
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(manifest.publication.render_routeros_dns_script())
}

pub async fn junos_dns_set(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let manifest = state.launch_manifest();
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(manifest.publication.render_junos_dns_set())
}

pub async fn vps_blueprint(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let manifest = state.launch_manifest();
    let mut body = String::new();
    manifest.publication.write_vps_json(&mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn overlay_server_config(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
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

pub async fn overlay_peer_config(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
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

pub async fn mamori_manifest(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let manifest = state.launch_manifest();
    let mut body = String::new();
    manifest.mamori.write_json(&mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn kaminari_tools(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let manifest = state.launch_manifest();
    let mut body = String::new();
    manifest.kaminari.write_json(&mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn kaminari_rpc(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
    tool: web::Path<String>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let manifest = state.launch_manifest();
    match manifest.kaminari.rpc_for_tool(&tool) {
        Ok(rpc) => HttpResponse::Ok().content_type("application/xml").body(rpc),
        Err(KaminariControlError::UnknownTool(_)) => HttpResponse::NotFound()
            .content_type("application/json")
            .body(r#"{"error":"unknown_kaminari_tool"}"#),
        Err(KaminariControlError::WriteBlocked(_)) => HttpResponse::Forbidden()
            .content_type("application/json")
            .body(r#"{"error":"kaminari_write_blocked"}"#),
        Err(_) => HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(r#"{"error":"kaminari_catalog_invalid"}"#),
    }
}

pub async fn michisuji_rb2011_script(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let manifest = state.launch_manifest();
    let server_address = manifest
        .overlay
        .as_ref()
        .map(|overlay| overlay.address.to_string())
        .unwrap_or_else(|| manifest.endpoint.bind_host.clone());
    let profile = MichisujiRackProfile::rb2011(server_address, manifest.endpoint.port)
        .with_identity(format!("{}-rb2011", manifest.config.server_name));

    match profile.render_script() {
        Ok(script) => HttpResponse::Ok()
            .content_type("text/plain; charset=utf-8")
            .body(script),
        Err(_) => HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(r#"{"error":"michisuji_profile_invalid"}"#),
    }
}

pub async fn database_status(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
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

pub async fn security_status(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let mut body = String::new();
    state.security.write_json(&mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn security_monitoring(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    let manifest = state.launch_manifest();
    let mut body = String::new();
    monitoring::write_sentinel_json(&manifest, &state.security, &mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

pub async fn metrics(req: HttpRequest, state: web::Data<OmoikaneHayateState>) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    if console::wants_console(&req) {
        return console::console_response(&req, state.get_ref());
    }
    match state.status_snapshot() {
        Ok(snapshot) => {
            let manifest = state.launch_manifest();
            let mut body = String::new();
            state
                .metrics
                .write_prometheus(&snapshot, &manifest, state.sql.is_some(), &mut body);
            state.security.write_prometheus(&mut body);
            HttpResponse::Ok().content_type("text/plain").body(body)
        }
        Err(error) => status_error_response(error),
    }
}

pub async fn grakane_dashboard(
    req: HttpRequest,
    state: web::Data<OmoikaneHayateState>,
) -> HttpResponse {
    state.metrics.record_http_request();
    if let Some(response) = state.security.guard(&req) {
        return response;
    }
    if console::wants_console(&req) {
        return console::console_response(&req, state.get_ref());
    }
    let mut body = String::new();
    state.metrics.write_grakane_dashboard(&mut body);
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
}

fn write_status_body(state: &OmoikaneHayateState) -> Result<Vec<u8>, OmoikaneHayateError> {
    let snapshot = state.status_snapshot()?;
    let mut body = Vec::with_capacity(512);
    snapshot.write_json(&mut body);
    Ok(body)
}

fn write_launch_body(state: &OmoikaneHayateState) -> String {
    let manifest = state.launch_manifest();
    let mut body = String::new();
    manifest.write_json(&mut body);
    body
}

fn status_error_response(error: OmoikaneHayateError) -> HttpResponse {
    match error {
        OmoikaneHayateError::ServerLockPoisoned => HttpResponse::InternalServerError()
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
    use super::{OmoikaneHayateState, configure_omoikane_routes};
    use actix_web::{App, http::StatusCode, test, web};
    use daikoku::{DaikokuServer, ServerOptions};

    fn status_server() -> DaikokuServer {
        let mut server = DaikokuServer::new(ServerOptions {
            server_name: "Omoikane Hayate".to_string(),
            max_players: 128,
            tick_rate: 144,
        });
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        server.tick_update(1.0 / 144.0);
        server
    }

    #[test]
    fn hayate_routes_serve_daikoku_status() {
        actix_web::rt::System::new().block_on(async {
            let state = web::Data::new(OmoikaneHayateState::new(status_server()));
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
            assert!(body.contains("\"name\":\"Omoikane Hayate\""));
            assert!(body.contains("\"tick_rate\":144"));
            assert!(body.contains("\"players\":1"));
        });
    }

    #[test]
    fn hayate_routes_serve_health_and_head_without_body() {
        actix_web::rt::System::new().block_on(async {
            let state = web::Data::new(OmoikaneHayateState::new(status_server()));
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
    fn hayate_routes_serve_launch_and_overlay_manifests() {
        actix_web::rt::System::new().block_on(async {
            let state = web::Data::new(OmoikaneHayateState::new(status_server()));
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
            assert!(body.contains("\"server_name\":\"Omoikane Hayate\""));
            assert!(body.contains("\"overlay\""));
            assert!(body.contains("\"kaminari\""));
            assert!(body.contains("\"mamori\""));
            assert!(body.contains("\"publication\""));

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
                    .uri("/automation/kaminari/rpc/kaminari.interface_terse")
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
                    .uri("/automation/kaminari/rpc/kaminari.commit_confirmed")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/automation/mamori")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("\"name\":\"omoikane-rack-acceptance\""));

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/automation/michisuji/rb2011.rsc")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("Omoikane Michisuji rack profile"));

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/network/publication")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("\"subservers\""));
            assert!(body.contains("\"routeros_script_url\""));

            let response =
                test::call_service(&app, test::TestRequest::get().uri("/servers").to_request())
                    .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("\"id\":\"site\""));

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/network/dns/routeros.rsc")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("Omoikane public DNS publication plan"));

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/network/dns/junos.set")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("Omoikane public DNS publication plan"));

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/vps/reality-blueprint")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("VLESS Reality"));

            let response =
                test::call_service(&app, test::TestRequest::get().uri("/site").to_request()).await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("<h1>Omoikane</h1>"));

            let response =
                test::call_service(&app, test::TestRequest::get().uri("/metrics").to_request())
                    .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("omoikane_http_requests_total"));
            assert!(body.contains("omoikane_machine_parallelism"));
            assert!(body.contains("omoikane_subservers_configured"));

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/grakane/dashboard.json")
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("\"title\":\"Omoikane Grakane Runtime\""));

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

    #[test]
    fn hayate_root_serves_public_site_for_matching_dns_host() {
        actix_web::rt::System::new().block_on(async {
            let config = omoikane_control::OmoikaneLaunchConfig {
                public_dns_name: Some("omoikane.example".to_string()),
                public_dns_target: Some("203.0.113.10".to_string()),
                ..omoikane_control::OmoikaneLaunchConfig::default()
            };
            let state = web::Data::new(OmoikaneHayateState::with_launch_config(
                status_server(),
                config,
            ));
            let app = test::init_service(
                App::new()
                    .app_data(state.clone())
                    .configure(configure_omoikane_routes),
            )
            .await;

            let response = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/")
                    .insert_header((actix_web::http::header::HOST, "omoikane.example:8080"))
                    .to_request(),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = test::read_body(response).await;
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("Servidor Hayate ativo"));
            assert!(body.contains("203.0.113.10"));
        });
    }
}
