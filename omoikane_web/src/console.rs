use crate::OmoikaneHayateState;
use actix_web::{HttpRequest, HttpResponse, http::header};

pub fn wants_console(req: &HttpRequest) -> bool {
    req.headers()
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|accept| accept.contains("text/html"))
}

pub fn console_response(req: &HttpRequest, state: &OmoikaneHayateState) -> HttpResponse {
    let manifest = state.launch_manifest();
    if let Some(admin) = manifest.config.grakane_admin_gmail.as_deref() {
        if let Some(gmail) = matching_query_gmail(req, admin) {
            return HttpResponse::Ok()
                .insert_header((
                    header::SET_COOKIE,
                    format!("omoikane_grakane={gmail}; Path=/; SameSite=Lax"),
                ))
                .insert_header((header::CACHE_CONTROL, "no-store"))
                .content_type("text/html; charset=utf-8")
                .body(render_console_page(true));
        }
        if !cookie_matches(req, admin) {
            return HttpResponse::Ok()
                .insert_header((header::CACHE_CONTROL, "no-store"))
                .content_type("text/html; charset=utf-8")
                .body(render_login_page());
        }
    }

    HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .content_type("text/html; charset=utf-8")
        .body(render_console_page(
            manifest.config.grakane_admin_gmail.is_some(),
        ))
}

fn render_login_page() -> String {
    r#"<!doctype html>
<html lang="pt-br">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Omoikane Grakane</title>
<style>
:root { color-scheme: dark; --bg:#0b0d0f; --panel:#13181b; --line:#2c383d; --text:#e8f0ef; --muted:#8ea09d; --cyan:#38d6c6; --amber:#f2bd57; --red:#ff6b68; }
* { box-sizing: border-box; }
body { margin:0; min-height:100vh; display:grid; place-items:center; background:radial-gradient(circle at 20% 0%, #122326 0, #0b0d0f 35%, #090a0b 100%); color:var(--text); font:14px/1.5 Consolas, "Cascadia Mono", monospace; letter-spacing:0; }
main { width:min(520px, calc(100vw - 28px)); border:1px solid var(--line); background:linear-gradient(180deg, #151b1e, #0f1315); box-shadow:0 24px 80px rgba(0,0,0,.45); }
.bar { display:flex; align-items:center; justify-content:space-between; padding:12px 14px; border-bottom:1px solid var(--line); background:#0f1416; }
.brand { color:var(--cyan); font-weight:700; }
.body { padding:22px; }
h1 { margin:0 0 8px; font-size:20px; font-weight:700; }
p { margin:0 0 18px; color:var(--muted); }
label { display:block; margin:0 0 8px; color:var(--amber); }
input { width:100%; padding:13px 12px; border:1px solid var(--line); background:#090c0d; color:var(--text); outline:none; font:inherit; }
button { margin-top:14px; width:100%; border:1px solid #2a8d84; background:#102d2b; color:var(--text); padding:12px; font:inherit; cursor:pointer; }
button:hover { border-color:var(--cyan); }
.err { min-height:22px; color:var(--red); margin-top:10px; }
</style>
</head>
<body>
<main>
  <div class="bar"><span class="brand">OMOIKANE :: GRAKANE</span><span>host gate</span></div>
  <form class="body" id="login">
    <h1>Controle do host</h1>
    <p>Grakane esta protegido pelo Gmail configurado no terminal de lancamento.</p>
    <label for="gmail">Gmail administrador</label>
    <input id="gmail" name="gmail" autocomplete="email" inputmode="email" placeholder="nome@gmail.com">
    <button>Entrar no console</button>
    <div class="err" id="err"></div>
  </form>
</main>
<script>
document.getElementById('login').addEventListener('submit', event => {
  event.preventDefault();
  const email = document.getElementById('gmail').value.trim().toLowerCase();
  if (!email.endsWith('@gmail.com') || email.startsWith('@') || /\s/.test(email)) {
    document.getElementById('err').textContent = 'Use um Gmail valido.';
    return;
  }
  const url = new URL(location.href);
  url.searchParams.set('gmail', email);
  location.href = url.toString();
});
</script>
</body>
</html>"#
    .to_string()
}

fn render_console_page(grakane_locked: bool) -> String {
    format!(
        r#"<!doctype html>
<html lang="pt-br">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Omoikane Mikado Console</title>
<style>
:root {{
  color-scheme: dark;
  --bg:#090b0c; --rail:#0e1214; --panel:#121719; --panel2:#171e21; --line:#2a363a;
  --text:#e8f0ef; --muted:#91a4a1; --soft:#c5d6d3; --cyan:#38d6c6; --green:#8ee88c;
  --amber:#f2bd57; --red:#ff6b68; --violet:#b7a7ff; --blue:#68a8ff;
}}
* {{ box-sizing:border-box; }}
html, body {{ margin:0; min-height:100%; background:var(--bg); color:var(--text); font:14px/1.45 Consolas, "Cascadia Mono", "SFMono-Regular", monospace; letter-spacing:0; }}
body {{ overflow-x:hidden; }}
.app {{ min-height:100vh; display:grid; grid-template-columns:58px 1fr; background:linear-gradient(180deg,#0a0d0e,#0b0d0f); }}
.rail {{ background:var(--rail); border-right:1px solid var(--line); padding:10px 8px; display:flex; flex-direction:column; align-items:center; gap:10px; position:sticky; top:0; height:100vh; }}
.logo {{ width:38px; height:38px; border:1px solid #2c9188; display:grid; place-items:center; color:var(--cyan); font-weight:800; background:#10201f; }}
.navbtn {{ width:38px; height:34px; border:1px solid var(--line); background:#121719; color:var(--soft); display:grid; place-items:center; cursor:pointer; }}
.navbtn:hover {{ border-color:var(--cyan); color:var(--cyan); }}
.main {{ min-width:0; }}
.top {{ height:56px; display:flex; align-items:center; justify-content:space-between; gap:12px; padding:0 18px; border-bottom:1px solid var(--line); background:#101416; position:sticky; top:0; z-index:2; }}
.brand {{ display:flex; flex-direction:column; min-width:0; }}
.brand strong {{ color:var(--cyan); font-size:15px; }}
.brand span {{ color:var(--muted); font-size:12px; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }}
.actions {{ display:flex; align-items:center; gap:8px; flex-wrap:wrap; justify-content:flex-end; }}
button, select, input {{ font:inherit; letter-spacing:0; }}
button, .linkbtn {{ border:1px solid var(--line); color:var(--text); background:#121719; padding:8px 10px; cursor:pointer; text-decoration:none; white-space:nowrap; }}
button:hover, .linkbtn:hover {{ border-color:var(--cyan); color:var(--cyan); }}
.statusdot {{ width:9px; height:9px; border-radius:50%; background:var(--green); box-shadow:0 0 18px var(--green); display:inline-block; }}
.grid {{ display:grid; grid-template-columns:minmax(360px, 1.25fr) minmax(320px, .75fr); gap:12px; padding:12px; }}
.panel {{ border:1px solid var(--line); background:var(--panel); min-width:0; }}
.panelhead {{ height:38px; display:flex; align-items:center; justify-content:space-between; gap:10px; padding:0 12px; border-bottom:1px solid var(--line); background:var(--panel2); }}
.panelhead h2 {{ margin:0; color:var(--soft); font-size:13px; font-weight:700; }}
.panelhead small {{ color:var(--muted); }}
.body {{ padding:12px; }}
.wide {{ grid-column:1 / -1; }}
.metrics {{ display:grid; grid-template-columns:repeat(4, minmax(120px,1fr)); gap:8px; }}
.metric {{ border:1px solid #223035; background:#0d1112; padding:10px; min-height:78px; }}
.metric span {{ color:var(--muted); display:block; font-size:12px; }}
.metric strong {{ display:block; margin-top:8px; font-size:22px; color:var(--text); }}
.metric em {{ display:block; margin-top:3px; color:var(--cyan); font-style:normal; font-size:12px; }}
.term {{ height:410px; overflow:auto; background:#050707; color:#d6f7ef; padding:12px; border:1px solid #1c292c; box-shadow:inset 0 0 0 1px #071011; }}
.line {{ white-space:pre-wrap; word-break:break-word; }}
.prompt {{ color:var(--cyan); }}
.warn {{ color:var(--amber); }}
.bad {{ color:var(--red); }}
.ok {{ color:var(--green); }}
.muted {{ color:var(--muted); }}
.split {{ display:grid; grid-template-columns:1fr 1fr; gap:8px; }}
.kv {{ display:grid; grid-template-columns:120px 1fr; gap:5px 10px; align-items:start; }}
.kv span:nth-child(odd) {{ color:var(--muted); }}
.kv span:nth-child(even) {{ color:var(--text); overflow-wrap:anywhere; }}
.choices {{ display:grid; gap:7px; }}
.choice {{ display:grid; grid-template-columns:22px 1fr auto; gap:8px; align-items:center; padding:8px; border:1px solid #223035; background:#0d1112; }}
.choice b {{ font-weight:700; color:var(--soft); }}
.choice small {{ color:var(--muted); display:block; margin-top:2px; }}
.pill {{ border:1px solid #315254; color:var(--cyan); padding:2px 6px; font-size:12px; }}
.cmdbar {{ display:flex; gap:8px; margin-top:8px; }}
.cmdbar input {{ flex:1; min-width:0; border:1px solid var(--line); background:#070a0b; color:var(--text); padding:9px 10px; outline:none; }}
.cmdbar input:focus {{ border-color:var(--cyan); }}
.table {{ width:100%; border-collapse:collapse; }}
.table th, .table td {{ text-align:left; border-bottom:1px solid #223035; padding:8px; vertical-align:top; }}
.table th {{ color:var(--muted); font-weight:400; }}
.table td {{ color:var(--text); overflow-wrap:anywhere; }}
@media (max-width: 980px) {{
  .app {{ grid-template-columns:1fr; }}
  .rail {{ display:none; }}
  .grid {{ grid-template-columns:1fr; }}
  .metrics {{ grid-template-columns:repeat(2, minmax(120px,1fr)); }}
  .top {{ height:auto; min-height:56px; align-items:flex-start; padding:10px 12px; flex-direction:column; }}
}}
@media (max-width: 560px) {{
  .metrics, .split {{ grid-template-columns:1fr; }}
  .kv {{ grid-template-columns:1fr; }}
  .term {{ height:340px; }}
}}
</style>
</head>
<body>
<div class="app">
  <aside class="rail">
    <div class="logo">御</div>
    <button class="navbtn" data-jump="runtime" title="Runtime">RT</button>
    <button class="navbtn" data-jump="network" title="Rede">IP</button>
    <button class="navbtn" data-jump="security" title="Seguranca">FW</button>
    <button class="navbtn" data-jump="terminal" title="Terminal">$_</button>
  </aside>
  <main class="main">
    <header class="top">
      <div class="brand">
        <strong>Omoikane Mikado Console</strong>
        <span id="subtitle">carregando runtime...</span>
      </div>
      <div class="actions">
        <span><i class="statusdot"></i> running</span>
        <a class="linkbtn" href="/" id="consoleLink">console</a>
        <a class="linkbtn" href="/metrics" id="metricsLink">metrics</a>
        <button id="refreshNow">atualizar</button>
      </div>
    </header>
    <section class="grid">
      <section class="panel wide" id="runtime">
        <div class="panelhead"><h2>Runtime unico</h2><small id="refreshStamp">--</small></div>
        <div class="body metrics">
          <div class="metric"><span>estado</span><strong id="state">--</strong><em id="serverName">Omoikane</em></div>
          <div class="metric"><span>tick</span><strong id="tick">0</strong><em id="tickRate">0 Hz</em></div>
          <div class="metric"><span>jogadores</span><strong id="players">0/0</strong><em>fila integrada</em></div>
          <div class="metric"><span>http</span><strong id="httpTotal">0</strong><em>req total</em></div>
        </div>
      </section>
      <section class="panel" id="network">
        <div class="panelhead"><h2>IP e DNS de lancamento</h2><small>auto dns rust-native</small></div>
        <div class="body">
          <div class="kv" id="endpointKv"></div>
          <div style="height:10px"></div>
          <div class="choices" id="dnsChoices"></div>
        </div>
      </section>
      <section class="panel" id="security">
        <div class="panelhead"><h2>Firewall, anti-DDoS e SQLx</h2><small id="securityMode">--</small></div>
        <div class="body split">
          <div class="kv" id="securityKv"></div>
          <div class="kv" id="databaseKv"></div>
        </div>
      </section>
      <section class="panel wide" id="terminal">
        <div class="panelhead"><h2>Terminal Michisuji/Kaminari/Grakane</h2><small>{lock_label}</small></div>
        <div class="body">
          <div class="term" id="terminalOut"></div>
          <div class="cmdbar">
            <input id="command" autocomplete="off" spellcheck="false" placeholder="status | dns | firewall | sqlx | grakane | routeros | juniper | sentinel | metrics">
            <button id="runCommand">executar</button>
          </div>
        </div>
      </section>
      <section class="panel">
        <div class="panelhead"><h2>Grakane</h2><small>monitoramento em tempo real</small></div>
        <div class="body">
          <table class="table" id="grakaneTable"></table>
        </div>
      </section>
      <section class="panel">
        <div class="panelhead"><h2>Sentinel</h2><small>phishing watch e forense</small></div>
        <div class="body">
          <div class="kv" id="sentinelKv"></div>
        </div>
      </section>
    </section>
  </main>
</div>
<script>
const locked = {locked};
const $ = (id) => document.getElementById(id);
const terminal = $('terminalOut');
const state = {{ status:null, launch:null, security:null, db:null, grakane:null, sentinel:null, metrics:'' }};

function esc(value) {{
  return String(value ?? '').replace(/[&<>"']/g, ch => ({{'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}}[ch]));
}}
function write(lines, cls = '') {{
  const items = Array.isArray(lines) ? lines : [lines];
  for (const line of items) {{
    const div = document.createElement('div');
    div.className = 'line ' + cls;
    div.innerHTML = esc(line);
    terminal.appendChild(div);
  }}
  terminal.scrollTop = terminal.scrollHeight;
}}
async function getJson(url) {{
  const res = await fetch(url, {{ headers: {{ Accept: 'application/json' }}, cache: 'no-store' }});
  if (!res.ok) throw new Error(url + ' -> ' + res.status);
  return res.json();
}}
async function getText(url) {{
  const res = await fetch(url, {{ headers: {{ Accept: 'text/plain' }}, cache: 'no-store' }});
  if (!res.ok) throw new Error(url + ' -> ' + res.status);
  return res.text();
}}
function kv(node, rows) {{
  node.innerHTML = rows.map(([k,v]) => `<span>${{esc(k)}}</span><span>${{esc(v)}}</span>`).join('');
}}
function metricValue(name) {{
  const match = state.metrics.match(new RegExp('^' + name + '\\\\s+([^\\\\n]+)', 'm'));
  return match ? match[1] : '0';
}}
function render() {{
  const s = state.status || {{}};
  const launch = state.launch || {{}};
  const endpoint = launch.endpoint || {{}};
  const dns = launch.dns || {{ choices: [] }};
  const sec = state.security || {{}};
  const db = state.db || {{}};
  const sentinel = state.sentinel || {{}};
  $('subtitle').textContent = `${{endpoint.bind_host || '--'}}:${{endpoint.port || '--'}}  ->  ${{endpoint.public_status_url || '--'}}`;
  $('state').textContent = s.state || '--';
  $('serverName').textContent = s.name || launch?.config?.server_name || 'Omoikane';
  $('tick').textContent = s.tick ?? '0';
  $('tickRate').textContent = `${{s.tick_rate ?? launch?.config?.tick_rate ?? 0}} Hz`;
  $('players').textContent = `${{s.players ?? 0}}/${{s.max_players ?? launch?.config?.max_players ?? 0}}`;
  $('httpTotal').textContent = metricValue('omoikane_http_requests_total');
  $('refreshStamp').textContent = new Date().toLocaleTimeString();
  $('consoleLink').href = dns.console_url || '/';
  $('metricsLink').href = '/metrics';
  kv($('endpointKv'), [
    ['bind', `${{endpoint.bind_host || '--'}}:${{endpoint.port || '--'}}`],
    ['local', endpoint.local_status_url || '--'],
    ['publico', endpoint.public_status_url || '--'],
    ['dns ativo', dns.selected_host || '--'],
    ['overlay', launch.overlay ? launch.overlay.address : 'desativado']
  ]);
  $('dnsChoices').innerHTML = (dns.choices || []).map(choice => `
    <div class="choice">
      <span class="${{choice.selected ? 'ok' : 'muted'}}">${{choice.selected ? '●' : '○'}}</span>
      <span><b>${{esc(choice.label)}}</b><small>${{esc(choice.host)}} via ${{esc(choice.provider_hint)}} · ${{esc(choice.record_type)}} · ttl ${{esc(choice.ttl_seconds)}}s</small></span>
      <span class="pill">${{choice.selected ? 'ativo' : 'lista'}}</span>
    </div>`).join('');
  $('securityMode').textContent = sec.enabled ? `${{sec.max_requests}}/${{sec.window_seconds}}s` : 'desativado';
  kv($('securityKv'), [
    ['anti-DDoS', sec.enabled ? 'ativo' : 'desativado'],
    ['bloqueios', sec.blocked_total ?? 0],
    ['eventos', sec.events_total ?? 0],
    ['clientes', sec.active_clients ?? 0],
    ['postura', sec.posture || '--']
  ]);
  kv($('databaseKv'), [
    ['sqlx', db.enabled ? db.status : 'desativado'],
    ['url', db.url || '--'],
    ['pool', db.max_connections || '--'],
    ['erros sql', metricValue('omoikane_sql_errors_total')]
  ]);
  const panels = (state.grakane && state.grakane.panels) || [];
  $('grakaneTable').innerHTML = '<tr><th>painel</th><th>expressao</th></tr>' + panels.map(panel => `<tr><td>${{esc(panel.title)}}</td><td>${{esc(panel.targets?.[0]?.expr || '--')}}</td></tr>`).join('');
  kv($('sentinelKv'), [
    ['dns', sentinel.dns_host || '--'],
    ['risco phishing', sentinel.phishing_risk_score ?? 0],
    ['sinal', (sentinel.phishing_signals || ['--']).join(', ')],
    ['maquina', sentinel.forensics ? `${{sentinel.forensics.os}}/${{sentinel.forensics.arch}} pid=${{sentinel.forensics.pid}}` : '--'],
    ['modo', sentinel.mode || '--']
  ]);
}}
async function refresh() {{
  try {{
    const [status, launch, security, db, grakane, sentinel, metrics] = await Promise.all([
      getJson('/status'), getJson('/launch'), getJson('/security/status'), getJson('/database/status'),
      getJson('/grakane/dashboard.json'), getJson('/security/monitoring'), getText('/metrics')
    ]);
    Object.assign(state, {{ status, launch, security, db, grakane, sentinel, metrics }});
    render();
  }} catch (err) {{
    write('falha de refresh: ' + err.message, 'bad');
  }}
}}
async function command(value) {{
  const cmd = value.trim().toLowerCase();
  if (!cmd) return;
  write('omoikane> ' + cmd, 'prompt');
  try {{
    if (cmd === 'status') {{
      write([`estado=${{state.status?.state}} tick=${{state.status?.tick}} players=${{state.status?.players}}`, `public=${{state.launch?.endpoint?.public_status_url}}`], 'ok');
    }} else if (cmd === 'dns') {{
      write((state.launch?.dns?.choices || []).map(c => `${{c.selected ? '*' : ' '}} ${{c.label}} -> ${{c.host}} (${{c.record_type}})`));
    }} else if (cmd === 'firewall') {{
      write([`anti-ddos=${{state.security?.enabled}} budget=${{state.security?.max_requests}}/${{state.security?.window_seconds}}s`, `blocked=${{state.security?.blocked_total}} active_clients=${{state.security?.active_clients}}`], 'warn');
    }} else if (cmd === 'sqlx') {{
      write(`sqlx=${{state.db?.enabled ? state.db.status : 'disabled'}} pool=${{state.db?.max_connections || '--'}}`, 'ok');
    }} else if (cmd === 'grakane') {{
      write((state.grakane?.panels || []).map(p => `${{p.title}} :: ${{p.targets?.[0]?.expr}}`), 'ok');
    }} else if (cmd === 'routeros') {{
      write(await getText('/automation/michisuji/rb2011.rsc'));
    }} else if (cmd === 'juniper') {{
      const tools = await getJson('/automation/kaminari/tools');
      write((tools.tools || []).map(t => `${{t.name}} :: ${{t.operation}} :: ${{t.mode}}`));
    }} else if (cmd === 'sentinel') {{
      write(JSON.stringify(state.sentinel, null, 2));
    }} else if (cmd === 'metrics') {{
      write((state.metrics || '').trim().split('\n').slice(0, 30));
    }} else {{
      write('comandos: status, dns, firewall, sqlx, grakane, routeros, juniper, sentinel, metrics', 'muted');
    }}
  }} catch (err) {{
    write('erro: ' + err.message, 'bad');
  }}
}}
$('refreshNow').addEventListener('click', refresh);
$('runCommand').addEventListener('click', () => {{ command($('command').value); $('command').value=''; }});
$('command').addEventListener('keydown', event => {{ if (event.key === 'Enter') {{ command(event.currentTarget.value); event.currentTarget.value=''; }} }});
document.querySelectorAll('[data-jump]').forEach(btn => btn.addEventListener('click', () => document.getElementById(btn.dataset.jump).scrollIntoView({{ behavior:'smooth', block:'start' }})));
write(['Omoikane Mikado Console iniciado.', locked ? 'Grakane autenticado por Gmail do host.' : 'Grakane em modo local sem Gmail configurado.', 'Digite status, dns, routeros, juniper ou sentinel.'], 'ok');
refresh();
setInterval(refresh, 1000);
</script>
</body>
</html>"#,
        locked = if grakane_locked { "true" } else { "false" },
        lock_label = if grakane_locked {
            "Gmail host ativo"
        } else {
            "modo host local"
        }
    )
}

fn matching_query_gmail(req: &HttpRequest, admin: &str) -> Option<String> {
    let gmail = query_value(req.query_string(), "gmail")?;
    if gmail.eq_ignore_ascii_case(admin) && is_valid_gmail(&gmail) {
        Some(gmail.to_ascii_lowercase())
    } else {
        None
    }
}

fn cookie_matches(req: &HttpRequest, admin: &str) -> bool {
    let Some(cookie) = req
        .headers()
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    cookie.split(';').any(|part| {
        let Some((name, value)) = part.trim().split_once('=') else {
            return false;
        };
        name == "omoikane_grakane" && value.eq_ignore_ascii_case(admin)
    })
}

fn query_value(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if key == name {
            Some(percent_decode(value))
        } else {
            None
        }
    })
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                    if let Ok(byte) = u8::from_str_radix(hex, 16) {
                        out.push(byte as char);
                        index += 3;
                        continue;
                    }
                }
                out.push('%');
                index += 1;
            }
            byte => {
                out.push(byte as char);
                index += 1;
            }
        }
    }
    out
}

fn is_valid_gmail(value: &str) -> bool {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    !value.is_empty()
        && !value.starts_with('@')
        && !value.chars().any(char::is_whitespace)
        && lower.ends_with("@gmail.com")
}

#[cfg(test)]
mod tests {
    use super::{percent_decode, wants_console};
    use actix_web::{http::header, test};

    #[test]
    fn browser_accept_header_selects_console() {
        let req = test::TestRequest::get()
            .insert_header((header::ACCEPT, "text/html,application/xhtml+xml"))
            .to_http_request();

        assert!(wants_console(&req));
    }

    #[test]
    fn query_percent_decode_supports_gmail() {
        assert_eq!(percent_decode("host%40gmail.com"), "host@gmail.com");
    }
}
