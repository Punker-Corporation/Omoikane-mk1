# Omoikane mk1

Omoikane mk1 e uma engine e runtime de servidor 100% Rust, organizada como
workspace direto na raiz. A base atual ja deixou de ser uma simples migracao e
passou a ter identidade propria: simulacao autoritativa, cliente previsivel,
renderer validado em CPU, automacao de rack auditavel e servidor web funcional.

## Estado Funcional

Esta linha marca o fim do estagio inicial e a entrada no estagio funcional.
Agora a Omoikane possui:

- launcher raiz `omoikane.exe` para Windows;
- binario Rust `omoikane-server` dentro de `omoikane_web`;
- borda HTTP Hayate baseada em dependencia Rust versionada;
- console unico Mikado em `/` e `/console`, com visual de terminal de rack;
- site publico de teste em `/site` e na raiz quando o Host bate com o DNS
  publico configurado;
- painel Grakane em `/grakane/dashboard.json`;
- DNS automatico Rust-native em `/network/dns`;
- plano de publicacao global em `/network/publication` e `/servers`;
- scripts DNS auditaveis para RouterOS e Junos em
  `/network/dns/routeros.rsc` e `/network/dns/junos.set`;
- firewall logico anti-DDoS e sentinel de monitoramento em `/security/status`;
- automacao Kaminari para catalogo NETCONF;
- plano Mamori para aceitacao de rack;
- gerador Michisuji para RB2011 em `/automation/michisuji/rb2011.rsc`;
- blueprint VPS/VLESS Reality em `/vps/reality-blueprint`, sem vendorizar Xray
  ou scripts externos;
- suporte SQL opcional via SQLx/PostgreSQL;
- terminal ANSI vivo com metricas e analise de maquina;
- ferramentas `xtask` para mapa arquitetural, smoke test, bench e perfil de
  rack.

## Crates

- `keisan`: matematica deterministica, cores, vetores, matrizes e geometria.
- `jikan`: ticks, timers, tempo de frame e suporte a game loop.
- `butsuri`: colisao, broadphase, corpos, fixtures, joints e ray queries.
- `sekai`: ECS compartilhado, mapas, transforms, serializacao, fisica e game
  state.
- `daikoku`: servidor autoritativo, filas de rede, PVS e inputs de prediction.
- `shinobi`: aplicacao de estado no cliente, interpolacao, prediction e
  sistemas locais.
- `hikari`: renderer CPU-only, render graph, recursos, command lists,
  pipelines e fronteira futura de runtime grafico.
- `omoikane_app`: host headless para orquestracao local de servidor, cliente,
  sandbox e cortes verticais.
- `omoikane_control`: manifestos de lancamento, overlay fixo, publicacao
  global, subservidores, Kaminari, Mamori e automacao de rede em Rust puro.
- `omoikane_web`: launcher, Hayate HTTP, SQLx, terminal vivo, metricas,
  Grakane, site publico, publicacao global e Michisuji.
- `xtask`: verificacoes e ferramentas de manutencao do repositorio.

## Comandos

```bash
cargo run -p xtask -- architecture-map
cargo run -p xtask -- architecture-map --dot
cargo run -p xtask -- architecture-map --json
cargo run -p xtask -- verify-architecture
cargo run -p xtask -- verify-layout
cargo run -p xtask -- michisuji-rb2011-profile --server 100.104.1.10 --port 8080
cargo run -p omoikane_app --example headless_sandbox
cargo run -p omoikane_web --bin omoikane-server -- --help
.\omoikane.exe --help
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Com o servidor rodando:

```bash
cargo run -p xtask -- web-smoke --host 127.0.0.1 --port 8080
cargo run -p xtask -- web-bench --host 127.0.0.1 --port 8080 --path /status --requests 128
```

Ao abrir `omoikane.exe` por duplo clique, o launcher tenta usar a porta `8080`.
Sem argumentos, ele abre o terminal Mikado para escolher IP disponivel, DNS
publico/local e Gmail administrador do Grakane. Se a porta ja estiver ocupada e
nenhuma porta tiver sido passada por `--port`, ele usa a primeira porta livre
entre `8081` e `8099` e mostra o endereco no terminal. Em erro fatal, a janela
permanece aberta ate Enter para que a mensagem possa ser lida.

## Servidor

O launcher raiz e uma copia precompilada do binario Rust de servidor. Ele sobe
a estrutura inteira, imprime o terminal da Omoikane e expoe:

- `/health` e `/healthz`;
- `/` e `/console`;
- `/site`;
- `/status` e `/status.json`;
- `/launch` e `/launch.json`;
- `/metrics`;
- `/database/status`;
- `/grakane/dashboard.json`;
- `/network/dns`;
- `/network/publication`;
- `/network/dns/routeros.rsc`;
- `/network/dns/junos.set`;
- `/network/overlay`;
- `/network/overlay/server.conf`;
- `/network/overlay/peer.conf`;
- `/servers`;
- `/vps/reality-blueprint`;
- `/security/status`;
- `/security/monitoring`;
- `/automation/mamori`;
- `/automation/kaminari/tools`;
- `/automation/kaminari/rpc/{tool}`;
- `/automation/michisuji/rb2011.rsc`.

Config TOML ou JSON pode ser carregada com:

```powershell
.\omoikane.exe --config .\omoikane.toml --port 8080
```

Campos aceitos: `server_name`, `bind_host`, `port`, `max_players`,
`tick_rate`, `overlay_seed`, `overlay_enabled`, `overlay_endpoint_hint`,
`database_url`, `database_max_connections`, `kaminari_host` e
`kaminari_username`, `public_dns_name`, `public_dns_target`,
`grakane_admin_gmail`, `anti_ddos_enabled`, `anti_ddos_window_seconds`,
`anti_ddos_max_requests`, `public_site_enabled`, `game_server_enabled`,
`game_server_port`, `vps_mode_enabled` e `vps_reality_sni`.

Exemplo de publicacao para um dominio real:

```powershell
.\omoikane.exe --name Omoikane --bind 0.0.0.0 --port 8080 --public-dns Omoikane.com --public-dns-target edge.seu-dominio.net --game-server --vps-mode --vps-reality-sni front.seu-dominio.net
```

Omoikane gera o plano, os subservidores e os scripts de roteador. Para que
`Omoikane.com` abra de outra rede, o operador ainda precisa controlar o
dominio, apontar o DNS autoritativo para um IP publico real e liberar NAT,
firewall ou tunnel ate a maquina que roda o servidor.

Quando `/status`, `/metrics`, `/launch` ou `/grakane/dashboard.json` sao
abertos por navegador com `Accept: text/html`, Hayate entrega o console unico.
Clientes tecnicos continuam recebendo JSON ou texto Prometheus ao pedir
`application/json` ou `text/plain`.

## Documentacao

- `docs/Omoikane Roadmap.md`: roteiro tecnico e marco funcional.
- `docs/Web Server Architecture.md`: Hayate, Grakane, SQLx e rotas.
- `docs/Network Control Architecture.md`: overlay, Kaminari, Mamori e
  Michisuji.
- `docs/Renderer Architecture.md`: contrato do `hikari`.
- `docs/Map Format.md`: formato de mapa.
- `docs/Research Intake.md`: intake tecnico e cientifico.
- `docs/Estagio Funcional.md`: checklist de transicao do estagio inicial para
  o estagio funcional.

## Legal

Veja `legal.md` e os arquivos de licenca na raiz.
