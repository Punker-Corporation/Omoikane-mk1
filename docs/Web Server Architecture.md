# Arquitetura do Servidor Web Omoikane

Data: 2026-05-07

Este documento registra o corte funcional do servidor web da Omoikane. A
direcao e manter `daikoku` como nucleo autoritativo e expor uma borda HTTP
operacional chamada Hayate, com metricas, terminal vivo, configuracao por
arquivo, SQL opcional e automacao de rack.

## Decisao Funcional

`omoikane_web` usa uma dependencia HTTP Rust versionada para servir rotas, mas
a identidade operacional da camada e Hayate. O runtime fica dividido assim:

- `daikoku`: estado autoritativo, snapshot de status e codec HTTP/1 minimo.
- `omoikane_control`: manifestos de lancamento, overlay fixo, Kaminari e
  Mamori.
- `omoikane_web`: Hayate HTTP, launcher, terminal, SQLx, Grakane e Michisuji.
- `omoikane_app`: host de aplicacao e cortes verticais locais.

Essa divisao permite operacao real sem obrigar simulacao, cliente, fisica ou
renderer a conhecerem a borda web.

## Rotas Hayate

`configure_omoikane_routes` registra:

- `GET /health` e `GET /healthz`
- `GET /status` e `HEAD /status`
- `GET /status.json` e `HEAD /status.json`
- `GET /launch` e `HEAD /launch`
- `GET /launch.json` e `HEAD /launch.json`
- `GET /network/overlay`
- `GET /network/overlay/server.conf`
- `GET /network/overlay/peer.conf`
- `GET /automation/mamori`
- `GET /automation/kaminari/tools`
- `GET /automation/kaminari/rpc/{tool}`
- `GET /automation/michisuji/rb2011.rsc`
- `GET /database/status`
- `GET /metrics`
- `GET /grakane/dashboard.json`

O corpo de `/status` vem de `DaikokuServer::status_snapshot`, no mesmo JSON
estavel usado pelo nucleo HTTP/1 de `daikoku`. Hayate entra como transporte e
observabilidade, nao como segunda fonte de estado.

## Launcher

O binario `omoikane-server` sobe o servidor inteiro. O arquivo `omoikane.exe`
na raiz e uma copia precompilada do mesmo launcher Rust para Windows, criada
para rodar a estrutura com um comando direto sem transformar a raiz do
repositorio em pacote Cargo.

Exemplo:

```powershell
.\omoikane.exe --name Omoikane --bind 0.0.0.0 --port 8080 --overlay-seed rack-a
```

Config TOML ou JSON tambem pode ser carregada:

```powershell
.\omoikane.exe --config .\omoikane.toml --port 8080
```

Campos aceitos:

- `server_name`
- `bind_host`
- `port`
- `max_players`
- `tick_rate`
- `overlay_seed`
- `overlay_enabled`
- `overlay_endpoint_hint`
- `database_url`
- `database_max_connections`
- `kaminari_host`
- `kaminari_username`

Argumentos de CLI aplicados depois de `--config` sobrescrevem o arquivo.

## Overlay

O IP overlay fixo e derivado de `overlay_seed + server_name` dentro de
`100.104.0.0/16`. Isso cria identidade estavel para o servidor sem depender de
port forwarding. O tunel real ainda exige chaves e endpoint legitimos fornecidos
pelo operador.

## SQLx

O launcher aceita:

```powershell
.\omoikane.exe --database-url postgres://user:pass@host/db
```

A integracao usa SQLx com drivers tipados de PostgreSQL. SQLite, MySQL e
MariaDB ficam fora deste corte para manter o grafo auditavel e evitar
dependencias que nao fazem parte do caminho funcional atual. Quando a URL e
fornecida, a Omoikane abre pool async, mascara senha no terminal e expoe
`/database/status` com liveness `SELECT 1`.

## Terminal e Observabilidade

O terminal vivo usa ANSI SGR direto, sem dependencia TUI. Ele redesenha o painel
a cada segundo com:

- estado autoritativo;
- tick e tick rate;
- jogadores e capacidade;
- uptime;
- requests HTTP;
- checks e erros SQL;
- URL local e URL publica overlay;
- sistema operacional, arquitetura, familia, PID e paralelismo visivel.

`/metrics` emite texto compativel com coletores de metricas. `/grakane/dashboard.json`
gera o painel Grakane da Omoikane: um dashboard JSON proprio, com uptime,
requests, tick, players, erros SQL e paralelismo da maquina.

## Automacao de Rack

Kaminari substitui o nome operacional antigo de catalogo NETCONF. Ele expoe
ferramentas read-only por padrao e bloqueia commit/rollback quando o perfil nao
libera escrita.

Mamori descreve plano de aceitacao, checks e rollback em JSON.

Michisuji gera script RB2011 auditavel pela rota
`/automation/michisuji/rb2011.rsc` e pelo comando:

```bash
cargo run -p xtask -- michisuji-rb2011-profile --server 100.104.1.10 --port 8080
```

O script deve ser revisado antes de producao, porque enderecos, interfaces,
politicas e topologia variam por rack.

## Ferramentas de Validacao

Os cortes que estavam pendentes agora possuem comandos:

- `cargo run -p xtask -- architecture-map --dot`
- `cargo run -p xtask -- architecture-map --json`
- `cargo run -p xtask -- web-smoke --host 127.0.0.1 --port 8080`
- `cargo run -p xtask -- web-bench --host 127.0.0.1 --port 8080 --path /status --requests 128`
- `cargo run -p xtask -- michisuji-rb2011-profile --server 100.104.1.10 --port 8080`

## Regras

- Nao deixar `daikoku` depender de `omoikane_web`.
- Nao embutir firmware, pacotes proprietarios, ativadores ou clientes opacos.
- Nao expor mutacao de simulacao por rota observacional.
- Mascara de credenciais SQL e obrigatoria em terminal e JSON.
- Todo novo endpoint deve ler snapshots, manifestos ou comandos explicitamente
  permitidos.
