use crate::base_server::ServerState;
use jikan::GameTick;
use std::io::Write as _;
use std::str;

pub const MAX_HTTP_STATUS_REQUEST_BYTES: usize = 8 * 1024;
pub const MAX_HTTP_STATUS_REQUEST_LINE_BYTES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ServerQueueStats {
    pub sessions: usize,
    pub outbound_messages: usize,
    pub queued_inputs: usize,
    pub queued_entities: usize,
    pub queued_player_list_requests: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerStatusSnapshot {
    pub name: String,
    pub state: ServerState,
    pub tick: GameTick,
    pub tick_rate: u16,
    pub players: usize,
    pub max_players: usize,
    pub queues: ServerQueueStats,
}

impl ServerStatusSnapshot {
    pub fn write_json(&self, out: &mut Vec<u8>) {
        out.clear();
        out.push(b'{');
        write_json_string_field(out, "name", &self.name, true);
        write_json_string_field(out, "state", server_state_wire_str(self.state), false);
        write_json_u32_field(out, "tick", self.tick.value, false);
        write_json_u64_field(out, "tick_rate", self.tick_rate as u64, false);
        write_json_u64_field(out, "players", self.players as u64, false);
        write_json_u64_field(out, "max_players", self.max_players as u64, false);
        out.extend_from_slice(br#","queues":{"#);
        write_json_u64_field(out, "sessions", self.queues.sessions as u64, true);
        write_json_u64_field(
            out,
            "outbound_messages",
            self.queues.outbound_messages as u64,
            false,
        );
        write_json_u64_field(
            out,
            "queued_inputs",
            self.queues.queued_inputs as u64,
            false,
        );
        write_json_u64_field(
            out,
            "queued_entities",
            self.queues.queued_entities as u64,
            false,
        );
        write_json_u64_field(
            out,
            "queued_player_list_requests",
            self.queues.queued_player_list_requests as u64,
            false,
        );
        out.extend_from_slice(b"}}");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpStatusMethod {
    Get,
    Head,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpStatusRoute {
    Health,
    Status,
}

impl HttpStatusRoute {
    const fn body_kind(self) -> HttpStatusBodyKind {
        match self {
            Self::Health => HttpStatusBodyKind::Health,
            Self::Status => HttpStatusBodyKind::Status,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HttpStatusBodyKind {
    Health,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpStatusRequest<'a> {
    pub method: HttpStatusMethod,
    pub route: HttpStatusRoute,
    pub target: &'a str,
    pub keep_alive: bool,
    pub consumed_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpStatusRouteError {
    Incomplete,
    RequestTooLarge,
    RequestLineTooLarge,
    MalformedRequest,
    UnsupportedMethod,
    UnsupportedVersion,
    UnsupportedRoute,
}

#[derive(Debug, Default)]
pub struct HttpStatusService {
    body: Vec<u8>,
}

impl HttpStatusService {
    pub fn new() -> Self {
        Self {
            body: Vec::with_capacity(512),
        }
    }

    pub fn parse_request<'a>(
        &self,
        bytes: &'a [u8],
    ) -> Result<HttpStatusRequest<'a>, HttpStatusRouteError> {
        parse_http_status_request(bytes)
    }

    pub fn write_response(
        &mut self,
        snapshot: &ServerStatusSnapshot,
        request: HttpStatusRequest<'_>,
        out: &mut Vec<u8>,
    ) {
        self.body.clear();
        match request.route.body_kind() {
            HttpStatusBodyKind::Health => {
                self.body.extend_from_slice(br#"{"ok":true}"#);
            }
            HttpStatusBodyKind::Status => snapshot.write_json(&mut self.body),
        }

        write_http_response(
            out,
            200,
            "OK",
            "application/json",
            &self.body,
            request.method == HttpStatusMethod::Head,
            request.keep_alive,
        );
    }

    pub fn write_error_response(&mut self, error: HttpStatusRouteError, out: &mut Vec<u8>) {
        self.body.clear();
        let (status, reason, body): (u16, &str, &[u8]) = match error {
            HttpStatusRouteError::Incomplete => (400, "Bad Request", br#"{"error":"incomplete"}"#),
            HttpStatusRouteError::RequestTooLarge => (
                413,
                "Payload Too Large",
                br#"{"error":"request_too_large"}"#,
            ),
            HttpStatusRouteError::RequestLineTooLarge => (
                414,
                "URI Too Long",
                br#"{"error":"request_line_too_large"}"#,
            ),
            HttpStatusRouteError::MalformedRequest => {
                (400, "Bad Request", br#"{"error":"malformed_request"}"#)
            }
            HttpStatusRouteError::UnsupportedMethod => (
                405,
                "Method Not Allowed",
                br#"{"error":"unsupported_method"}"#,
            ),
            HttpStatusRouteError::UnsupportedVersion => (
                505,
                "HTTP Version Not Supported",
                br#"{"error":"unsupported_version"}"#,
            ),
            HttpStatusRouteError::UnsupportedRoute => {
                (404, "Not Found", br#"{"error":"unsupported_route"}"#)
            }
        };
        self.body.extend_from_slice(body);
        write_http_response(
            out,
            status,
            reason,
            "application/json",
            &self.body,
            false,
            false,
        );
    }

    pub fn handle(
        &mut self,
        snapshot: &ServerStatusSnapshot,
        bytes: &[u8],
        out: &mut Vec<u8>,
    ) -> Result<usize, HttpStatusRouteError> {
        match self.parse_request(bytes) {
            Ok(request) => {
                let consumed = request.consumed_bytes;
                self.write_response(snapshot, request, out);
                Ok(consumed)
            }
            Err(error) => {
                if error != HttpStatusRouteError::Incomplete {
                    self.write_error_response(error, out);
                }
                Err(error)
            }
        }
    }
}

pub fn parse_http_status_request(
    bytes: &[u8],
) -> Result<HttpStatusRequest<'_>, HttpStatusRouteError> {
    if bytes.len() > MAX_HTTP_STATUS_REQUEST_BYTES {
        return Err(HttpStatusRouteError::RequestTooLarge);
    }
    let Some(header_end) = find_header_end(bytes) else {
        return Err(HttpStatusRouteError::Incomplete);
    };
    let header = &bytes[..header_end];
    let (request_line_bytes, header_bytes) = if let Some(line_end) = find_crlf(header) {
        (&header[..line_end], &header[line_end + 2..])
    } else {
        (header, &[][..])
    };
    if request_line_bytes.len() > MAX_HTTP_STATUS_REQUEST_LINE_BYTES {
        return Err(HttpStatusRouteError::RequestLineTooLarge);
    }

    let request_line =
        str::from_utf8(request_line_bytes).map_err(|_| HttpStatusRouteError::MalformedRequest)?;
    let mut parts = request_line.split_ascii_whitespace();
    let method = parse_method(parts.next().ok_or(HttpStatusRouteError::MalformedRequest)?)?;
    let target = parts.next().ok_or(HttpStatusRouteError::MalformedRequest)?;
    let version = parts.next().ok_or(HttpStatusRouteError::MalformedRequest)?;
    if parts.next().is_some() {
        return Err(HttpStatusRouteError::MalformedRequest);
    }
    if !matches!(version, "HTTP/1.0" | "HTTP/1.1") {
        return Err(HttpStatusRouteError::UnsupportedVersion);
    }
    let route = parse_route(target)?;
    let headers =
        str::from_utf8(header_bytes).map_err(|_| HttpStatusRouteError::MalformedRequest)?;
    let keep_alive = parse_keep_alive(version, headers);

    Ok(HttpStatusRequest {
        method,
        route,
        target,
        keep_alive,
        consumed_bytes: header_end + 4,
    })
}

fn parse_method(method: &str) -> Result<HttpStatusMethod, HttpStatusRouteError> {
    match method {
        "GET" => Ok(HttpStatusMethod::Get),
        "HEAD" => Ok(HttpStatusMethod::Head),
        _ => Err(HttpStatusRouteError::UnsupportedMethod),
    }
}

fn parse_route(target: &str) -> Result<HttpStatusRoute, HttpStatusRouteError> {
    let path = target.split_once('?').map_or(target, |(path, _)| path);
    match path {
        "/health" | "/healthz" => Ok(HttpStatusRoute::Health),
        "/status" | "/status.json" => Ok(HttpStatusRoute::Status),
        _ => Err(HttpStatusRouteError::UnsupportedRoute),
    }
}

fn parse_keep_alive(version: &str, headers: &str) -> bool {
    let mut connection = None;
    for line in headers.split("\r\n") {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("connection") {
            connection = Some(value.trim());
        }
    }

    match (version, connection) {
        (_, Some(value)) if value.eq_ignore_ascii_case("close") => false,
        (_, Some(value)) if value.eq_ignore_ascii_case("keep-alive") => true,
        ("HTTP/1.1", _) => true,
        _ => false,
    }
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn find_crlf(bytes: &[u8]) -> Option<usize> {
    bytes.windows(2).position(|window| window == b"\r\n")
}

fn write_http_response(
    out: &mut Vec<u8>,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
    suppress_body: bool,
    keep_alive: bool,
) {
    out.clear();
    write!(
        out,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: {}\r\nServer: Omoikane\r\n\r\n",
        body.len(),
        if keep_alive { "keep-alive" } else { "close" }
    )
    .expect("writing HTTP header to Vec<u8> cannot fail");
    if !suppress_body {
        out.extend_from_slice(body);
    }
}

fn write_json_string_field(out: &mut Vec<u8>, name: &str, value: &str, first: bool) {
    if !first {
        out.push(b',');
    }
    out.push(b'"');
    escape_json_str_to(name, out);
    out.extend_from_slice(br#"":""#);
    escape_json_str_to(value, out);
    out.push(b'"');
}

fn write_json_u32_field(out: &mut Vec<u8>, name: &str, value: u32, first: bool) {
    write_json_u64_field(out, name, value as u64, first);
}

fn write_json_u64_field(out: &mut Vec<u8>, name: &str, value: u64, first: bool) {
    if !first {
        out.push(b',');
    }
    out.push(b'"');
    escape_json_str_to(name, out);
    out.extend_from_slice(br#"":"#);
    write!(out, "{value}").expect("writing JSON number to Vec<u8> cannot fail");
}

fn escape_json_str_to(value: &str, out: &mut Vec<u8>) {
    for ch in value.chars() {
        match ch {
            '"' => out.extend_from_slice(br#"\""#),
            '\\' => out.extend_from_slice(br#"\\"#),
            '\n' => out.extend_from_slice(br#"\n"#),
            '\r' => out.extend_from_slice(br#"\r"#),
            '\t' => out.extend_from_slice(br#"\t"#),
            ch if ch.is_control() => {
                write!(out, "\\u{:04x}", ch as u32)
                    .expect("writing JSON escape to Vec<u8> cannot fail");
            }
            ch => {
                let mut buf = [0; 4];
                out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
}

fn server_state_wire_str(state: ServerState) -> &'static str {
    match state {
        ServerState::Stopped => "stopped",
        ServerState::Running => "running",
        ServerState::ShuttingDown => "shutting_down",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HttpStatusMethod, HttpStatusRoute, HttpStatusRouteError, HttpStatusService,
        ServerQueueStats, ServerStatusSnapshot, parse_http_status_request,
    };
    use crate::ServerState;
    use jikan::GameTick;

    fn snapshot() -> ServerStatusSnapshot {
        ServerStatusSnapshot {
            name: "Omoikane \"mk1\"".to_string(),
            state: ServerState::Running,
            tick: GameTick::new(42),
            tick_rate: 60,
            players: 2,
            max_players: 64,
            queues: ServerQueueStats {
                sessions: 2,
                outbound_messages: 3,
                queued_inputs: 4,
                queued_entities: 5,
                queued_player_list_requests: 6,
            },
        }
    }

    #[test]
    fn status_snapshot_writes_stable_json() {
        let mut json = Vec::new();
        snapshot().write_json(&mut json);
        assert_eq!(
            String::from_utf8(json).unwrap(),
            "{\"name\":\"Omoikane \\\"mk1\\\"\",\"state\":\"running\",\"tick\":42,\"tick_rate\":60,\"players\":2,\"max_players\":64,\"queues\":{\"sessions\":2,\"outbound_messages\":3,\"queued_inputs\":4,\"queued_entities\":5,\"queued_player_list_requests\":6}}"
        );
    }

    #[test]
    fn http_status_parser_accepts_status_and_health_routes() {
        let status =
            parse_http_status_request(b"GET /status?verbose=1 HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .unwrap();
        assert_eq!(status.method, HttpStatusMethod::Get);
        assert_eq!(status.route, HttpStatusRoute::Status);
        assert!(status.keep_alive);
        assert_eq!(status.consumed_bytes, 51);

        let health =
            parse_http_status_request(b"HEAD /healthz HTTP/1.0\r\nConnection: keep-alive\r\n\r\n")
                .unwrap();
        assert_eq!(health.method, HttpStatusMethod::Head);
        assert_eq!(health.route, HttpStatusRoute::Health);
        assert!(health.keep_alive);
    }

    #[test]
    fn http_status_parser_rejects_unsupported_requests() {
        assert_eq!(
            parse_http_status_request(b"POST /status HTTP/1.1\r\n\r\n"),
            Err(HttpStatusRouteError::UnsupportedMethod)
        );
        assert_eq!(
            parse_http_status_request(b"GET /missing HTTP/1.1\r\n\r\n"),
            Err(HttpStatusRouteError::UnsupportedRoute)
        );
        assert_eq!(
            parse_http_status_request(b"GET /status HTTP/2\r\n\r\n"),
            Err(HttpStatusRouteError::UnsupportedVersion)
        );
    }

    #[test]
    fn http_status_service_writes_get_and_head_responses() {
        let mut service = HttpStatusService::new();
        let mut out = Vec::new();
        let request = service
            .parse_request(b"GET /status HTTP/1.1\r\nConnection: close\r\n\r\n")
            .unwrap();
        service.write_response(&snapshot(), request, &mut out);
        let response = std::str::from_utf8(&out).unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.contains("Content-Type: application/json\r\n"));
        assert!(response.contains("Connection: close\r\n"));
        assert!(response.ends_with("\"queued_player_list_requests\":6}}"));

        let mut out = Vec::new();
        let consumed = service
            .handle(&snapshot(), b"HEAD /health HTTP/1.1\r\n\r\n", &mut out)
            .unwrap();
        let response = std::str::from_utf8(&out).unwrap();
        assert_eq!(consumed, 25);
        assert!(response.contains("Content-Length: 11\r\n"));
        assert!(response.ends_with("\r\n\r\n"));
    }

    #[test]
    fn http_status_service_writes_error_responses_for_complete_bad_requests() {
        let mut service = HttpStatusService::new();
        let mut out = Vec::new();
        assert_eq!(
            service.handle(&snapshot(), b"GET /missing HTTP/1.1\r\n\r\n", &mut out),
            Err(HttpStatusRouteError::UnsupportedRoute)
        );
        let response = std::str::from_utf8(&out).unwrap();
        assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"));
        assert!(response.ends_with("{\"error\":\"unsupported_route\"}"));

        out.clear();
        assert_eq!(
            service.handle(&snapshot(), b"GET /status HTTP/1.1\r\n", &mut out),
            Err(HttpStatusRouteError::Incomplete)
        );
        assert!(out.is_empty());
    }
}
