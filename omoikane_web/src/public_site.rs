use crate::{OmoikaneHayateError, OmoikaneHayateState};
use actix_web::{HttpRequest, HttpResponse, http::header};
use daikoku::ServerStatusSnapshot;
use omoikane_control::{OmoikaneLaunchManifest, is_private_or_local_target};
use std::fmt::Write as _;

pub fn should_serve_public_site(req: &HttpRequest, manifest: &OmoikaneLaunchManifest) -> bool {
    if !manifest.config.public_site_enabled || req.path() != "/" {
        return false;
    }

    let Some(host) = req
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(host_without_port)
    else {
        return false;
    };

    let configured = manifest
        .config
        .public_dns_name
        .as_deref()
        .unwrap_or(&manifest.dns.selected_host);
    !is_local_host(&host) && host.eq_ignore_ascii_case(configured)
}

pub fn public_site_response(state: &OmoikaneHayateState) -> HttpResponse {
    let manifest = state.launch_manifest();
    if !manifest.config.public_site_enabled {
        return HttpResponse::NotFound()
            .content_type("application/json")
            .body(r#"{"error":"public_site_disabled"}"#);
    }
    match state.status_snapshot() {
        Ok(snapshot) => HttpResponse::Ok()
            .insert_header((header::CACHE_CONTROL, "no-store"))
            .content_type("text/html; charset=utf-8")
            .body(render_public_site(&manifest, &snapshot)),
        Err(error) => public_site_error(error),
    }
}

fn public_site_error(error: OmoikaneHayateError) -> HttpResponse {
    match error {
        OmoikaneHayateError::ServerLockPoisoned => HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(r#"{"error":"server_lock_poisoned"}"#),
    }
}

fn render_public_site(
    manifest: &OmoikaneLaunchManifest,
    snapshot: &ServerStatusSnapshot,
) -> String {
    let mut subservers = String::new();
    for server in &manifest.publication.subservers {
        writeln!(
            subservers,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td><a href=\"{}\">{}</a></td></tr>",
            html_escape(&server.id),
            html_escape(&server.protocol),
            html_escape(&server.status),
            html_escape(&server.public_url),
            html_escape(&server.public_url)
        )
        .expect("writing public site HTML to String cannot fail");
    }

    let reachability = if is_private_or_local_target(&manifest.publication.target_host) {
        "publicacao externa exige DNS autoritativo, NAT ou tunnel apontando para um IP publico"
    } else {
        "alvo publico pronto para DNS autoritativo"
    };

    format!(
        r#"<!doctype html>
<html lang="pt-br">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{name}</title>
<style>
:root {{
  color-scheme: dark;
  --bg:#07090a; --ink:#edf5f2; --muted:#9da9a5; --line:#263238;
  --cyan:#42e8d2; --green:#98f07f; --amber:#f4c05f; --blue:#78a8ff; --red:#ff7474;
}}
* {{ box-sizing:border-box; }}
html, body {{ margin:0; min-height:100%; background:var(--bg); color:var(--ink); font:15px/1.5 Arial, sans-serif; letter-spacing:0; }}
body {{ overflow-x:hidden; }}
.hero {{ min-height:72vh; display:grid; grid-template-columns:minmax(0, 1.1fr) minmax(280px, .9fr); gap:22px; align-items:center; padding:32px clamp(18px, 5vw, 70px); border-bottom:1px solid var(--line); background:#080b0c; }}
.copy {{ max-width:760px; }}
.eyebrow {{ color:var(--cyan); text-transform:uppercase; font-size:12px; font-weight:700; }}
h1 {{ margin:8px 0 10px; font-size:clamp(38px, 7vw, 86px); line-height:.95; font-weight:800; }}
.lead {{ margin:0; max-width:620px; color:#c9d5d2; font-size:18px; }}
.status {{ display:grid; grid-template-columns:repeat(4, minmax(120px, 1fr)); gap:10px; margin-top:24px; }}
.stat {{ border:1px solid var(--line); background:#0d1213; padding:12px; min-height:86px; }}
.stat span {{ display:block; color:var(--muted); font-size:12px; }}
.stat strong {{ display:block; margin-top:8px; font-size:24px; }}
.stage {{ min-height:360px; border:1px solid #2a3b3e; background:#050708; position:relative; overflow:hidden; }}
#mesh {{ width:100%; height:100%; display:block; }}
.badge {{ position:absolute; left:14px; bottom:14px; color:var(--green); background:#07100d; border:1px solid #244b35; padding:8px 10px; font:13px Consolas, monospace; }}
.section {{ padding:24px clamp(18px, 5vw, 70px); }}
.section h2 {{ margin:0 0 12px; font-size:22px; }}
table {{ width:100%; border-collapse:collapse; border:1px solid var(--line); background:#0b0f10; }}
th, td {{ text-align:left; padding:11px 10px; border-bottom:1px solid #1f2a2e; vertical-align:top; overflow-wrap:anywhere; }}
th {{ color:var(--muted); font-weight:600; }}
a {{ color:var(--cyan); }}
.notice {{ margin-top:14px; color:var(--amber); }}
@media (max-width: 880px) {{
  .hero {{ grid-template-columns:1fr; min-height:auto; }}
  .stage {{ height:320px; }}
  .status {{ grid-template-columns:repeat(2, minmax(120px, 1fr)); }}
}}
@media (max-width: 520px) {{
  .status {{ grid-template-columns:1fr; }}
  th:nth-child(2), td:nth-child(2) {{ display:none; }}
}}
</style>
</head>
<body>
<main>
  <section class="hero">
    <div class="copy">
      <div class="eyebrow">runtime global rust</div>
      <h1>Omoikane</h1>
      <p class="lead">Servidor Hayate ativo, simulacao Daikoku em execucao e plano de publicacao pronto para DNS, rack e borda VPS.</p>
      <div class="status">
        <div class="stat"><span>estado</span><strong>{state}</strong></div>
        <div class="stat"><span>tick</span><strong>{tick}</strong></div>
        <div class="stat"><span>jogadores</span><strong>{players}/{max_players}</strong></div>
        <div class="stat"><span>dns</span><strong>{dns}</strong></div>
      </div>
    </div>
    <div class="stage">
      <canvas id="mesh"></canvas>
      <div class="badge">public target :: {target}</div>
    </div>
  </section>
  <section class="section">
    <h2>Subservidores</h2>
    <table>
      <thead><tr><th>id</th><th>protocolo</th><th>estado</th><th>url</th></tr></thead>
      <tbody>{subservers}</tbody>
    </table>
    <div class="notice">{reachability}</div>
  </section>
</main>
<script>
const canvas = document.getElementById('mesh');
const ctx = canvas.getContext('2d');
let t = 0;
function resize() {{
  const rect = canvas.parentElement.getBoundingClientRect();
  canvas.width = Math.max(320, Math.floor(rect.width * devicePixelRatio));
  canvas.height = Math.max(260, Math.floor(rect.height * devicePixelRatio));
}}
function draw() {{
  t += 0.012;
  const w = canvas.width, h = canvas.height;
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = '#050708';
  ctx.fillRect(0, 0, w, h);
  const cx = w * .5, cy = h * .5;
  for (let i = 0; i < 42; i++) {{
    const a = t + i * .48;
    const r = Math.min(w, h) * (.18 + (i % 7) * .045);
    const x = cx + Math.cos(a) * r;
    const y = cy + Math.sin(a * 1.31) * r * .62;
    ctx.beginPath();
    ctx.arc(x, y, 2.2 * devicePixelRatio, 0, Math.PI * 2);
    ctx.fillStyle = i % 3 === 0 ? '#42e8d2' : (i % 3 === 1 ? '#98f07f' : '#78a8ff');
    ctx.fill();
    if (i > 0) {{
      const b = t + (i - 1) * .48;
      const pr = Math.min(w, h) * (.18 + ((i - 1) % 7) * .045);
      ctx.beginPath();
      ctx.moveTo(cx + Math.cos(b) * pr, cy + Math.sin(b * 1.31) * pr * .62);
      ctx.lineTo(x, y);
      ctx.strokeStyle = 'rgba(66, 232, 210, .16)';
      ctx.lineWidth = devicePixelRatio;
      ctx.stroke();
    }}
  }}
  ctx.strokeStyle = 'rgba(244, 192, 95, .45)';
  ctx.lineWidth = 2 * devicePixelRatio;
  ctx.strokeRect(w * .14, h * .18, w * .72, h * .64);
  requestAnimationFrame(draw);
}}
addEventListener('resize', resize);
resize();
draw();
</script>
</body>
</html>"#,
        name = html_escape(&manifest.config.server_name),
        state = html_escape(&format!("{:?}", snapshot.state)),
        tick = snapshot.tick.value,
        players = snapshot.players,
        max_players = snapshot.max_players,
        dns = html_escape(&manifest.dns.selected_host),
        target = html_escape(&manifest.publication.target_host),
        subservers = subservers,
        reachability = html_escape(reachability)
    )
}

fn host_without_port(value: &str) -> String {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('[')
        && let Some((host, _)) = rest.split_once(']')
    {
        return host.to_string();
    }
    if value.matches(':').count() == 1 {
        return value
            .split_once(':')
            .map(|(host, _)| host.to_string())
            .unwrap_or_else(|| value.to_string());
    }
    value.to_string()
}

fn is_local_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host == "0.0.0.0"
}

fn html_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{host_without_port, should_serve_public_site};
    use actix_web::{http::header, test};
    use omoikane_control::OmoikaneLaunchConfig;

    #[test]
    fn public_site_host_gate_matches_configured_dns() {
        let manifest = OmoikaneLaunchConfig {
            public_dns_name: Some("omoikane.example".to_string()),
            ..OmoikaneLaunchConfig::default()
        }
        .build_manifest()
        .unwrap();
        let req = test::TestRequest::get()
            .uri("/")
            .insert_header((header::HOST, "omoikane.example:8080"))
            .to_http_request();

        assert!(should_serve_public_site(&req, &manifest));
    }

    #[test]
    fn public_site_does_not_replace_local_console() {
        let manifest = OmoikaneLaunchConfig::default().build_manifest().unwrap();
        let req = test::TestRequest::get()
            .uri("/")
            .insert_header((header::HOST, "127.0.0.1:8080"))
            .to_http_request();

        assert!(!should_serve_public_site(&req, &manifest));
        assert_eq!(host_without_port("[::1]:8080"), "::1");
    }
}
