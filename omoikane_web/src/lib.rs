mod routeros;

use actix_web::{HttpResponse, web};
use daikoku::{DaikokuServer, ServerStatusSnapshot};
use std::sync::{Arc, Mutex};

pub use routeros::{
    RB2011_ARCHITECTURE, ROUTEROS_STABLE_VERSION, RouterOsRackProfile, RouterOsScriptError,
};

#[derive(Clone)]
pub struct OmoikaneActixState {
    server: Arc<Mutex<DaikokuServer>>,
}

impl OmoikaneActixState {
    pub fn new(server: DaikokuServer) -> Self {
        Self {
            server: Arc::new(Mutex::new(server)),
        }
    }

    pub fn from_shared(server: Arc<Mutex<DaikokuServer>>) -> Self {
        Self { server }
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
        .route("/status.json", web::head().to(status_head));
}

pub async fn health() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"ok":true}"#)
}

pub async fn status(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    match write_status_body(&state) {
        Ok(body) => HttpResponse::Ok()
            .content_type("application/json")
            .body(body),
        Err(error) => status_error_response(error),
    }
}

pub async fn status_head(state: web::Data<OmoikaneActixState>) -> HttpResponse {
    match write_status_body(&state) {
        Ok(body) => HttpResponse::Ok()
            .content_type("application/json")
            .insert_header(("Content-Length", body.len().to_string()))
            .finish(),
        Err(error) => status_error_response(error),
    }
}

fn write_status_body(state: &OmoikaneActixState) -> Result<Vec<u8>, OmoikaneActixError> {
    let snapshot = state.status_snapshot()?;
    let mut body = Vec::with_capacity(512);
    snapshot.write_json(&mut body);
    Ok(body)
}

fn status_error_response(error: OmoikaneActixError) -> HttpResponse {
    match error {
        OmoikaneActixError::ServerLockPoisoned => HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(r#"{"error":"server_lock_poisoned"}"#),
    }
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
}
