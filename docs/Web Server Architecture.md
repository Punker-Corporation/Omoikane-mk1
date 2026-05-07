# Omoikane Web Server Architecture

Data: 2026-05-07

Este documento registra o primeiro corte web da Omoikane. A direcao e soldar
uma superficie web Rust de alto desempenho ao runtime, sem transformar o
servidor autoritativo em dependencia de framework e sem vender a alma da engine
para uma pilha externa cedo demais.

## Decisao Inicial

`omoikane_web` integra `actix-web` como borda HTTP ergonomica e performatica.
`daikoku` continua sendo a fonte autoritativa de estado. A fronteira fica assim:

- `daikoku`: estado autoritativo, snapshot de status e codec HTTP/1 minimo.
- `omoikane_control`: manifestos de lancamento, overlay fixo, automacao de rack
  e catalogo NETCONF.
- `omoikane_web`: adaptador Actix Web, rotas HTTP, terminal de servidor e perfis
  RouterOS.
- `omoikane_app`: host de aplicacao e cortes verticais locais.

Essa separacao permite usar Actix Web para operacao real sem obrigar os crates
de simulacao, cliente, fisica ou renderer a conhecerem Actix.

## Rotas Actix Iniciais

`configure_omoikane_routes` registra:

- `GET /health` e `GET /healthz`
- `GET /status` e `HEAD /status`
- `GET /status.json` e `HEAD /status.json`
- `GET /launch` e `GET /launch.json`
- `GET /network/overlay`
- `GET /network/overlay/server.conf`
- `GET /network/overlay/peer.conf`
- `GET /automation/robot`
- `GET /automation/junos/tools`
- `GET /automation/junos/rpc/{tool}`
- `GET /database/status`
- `GET /metrics`
- `GET /grafana/dashboard.json`

O corpo de `/status` e produzido por `DaikokuServer::status_snapshot`, escrito
pelo mesmo JSON estavel usado pelo nucleo HTTP/1 de `daikoku`. O Actix entra
como camada de transporte, nao como segunda fonte de estado.

## Launcher de Servidor

O binario `omoikane-server` sobe um `HttpServer` Actix real usando
`DaikokuServer` como nucleo autoritativo. Ao iniciar, ele imprime um terminal
Omoikane com:

- endereco de bind local
- URL local de status
- IP overlay fixo deterministico
- URL de status pelo overlay
- perfil NETCONF de rack
- plano de automacao de aceitacao

Exemplo:

```powershell
cargo run -p omoikane_web --bin omoikane-server -- --name Omoikane --bind 0.0.0.0 --port 8080 --overlay-seed rack-a
```

O IP overlay e derivado de `overlay_seed + server_name` dentro de `100.104.0.0/16`.
Isso da uma identidade estavel ao servidor sem depender de port forwarding. O
tunel real ainda precisa de chaves e endpoint legitimos fornecidos pelo operador.

O arquivo `omoikane.exe` na raiz e uma copia precompilada do launcher Rust para
Windows. Ele existe para rodar a estrutura inteira com um comando direto, sem
transformar a raiz do repositorio em pacote Cargo.

## SQLx

O launcher aceita:

```powershell
.\omoikane.exe --database-url postgres://user:pass@host/db
```

A integracao usa SQLx `0.8.6`, sem features default, com drivers tipados de
Postgres. SQLite nao e habilitado neste corte para evitar carregar `libsqlite3`.
MySQL/MariaDB tambem fica fora deste corte porque a cadeia atual do driver puxa
um advisory de RSA sem patch seguro. Quando a URL de banco e fornecida, a
Omoikane abre pool async, redige a senha no terminal e expoe `/database/status`
com liveness `SELECT 1`.

## Terminal e Observabilidade

O terminal vivo usa ANSI SGR direto, sem dependencia de TUI. Ele limpa e
redesenha o painel a cada segundo com:

- estado autoritativo
- tick e tick rate
- jogadores e capacidade
- uptime
- requests HTTP
- checks/erros SQL
- URL local e URL publica overlay
- sistema operacional, arquitetura, familia, PID e paralelismo visivel

A rota `/metrics` emite texto Prometheus-style. A rota
`/grafana/dashboard.json` gera um dashboard JSON importavel em Grafana com
paineis para uptime, requests, tick, players, SQL errors e paralelismo. A
Omoikane nao copia codigo do Grafana; ela produz uma interpretacao nativa do
modelo de observabilidade para que o operador conecte a ferramenta que quiser.

## Automacao de Rack e NETCONF

`omoikane_control` substitui scripts externos por contratos Rust:

- `NetworkRobotPlan` descreve dispositivos, checks criticos e rollback.
- `JunosMcpCatalog` lista operacoes NETCONF estruturadas para leitura,
  validacao de commit e commit confirmado.
- perfis read-only bloqueiam operacoes de escrita por padrao.
- `/automation/junos/rpc/{tool}` materializa o XML NETCONF de operacoes
  permitidas e retorna `403` para operacoes bloqueadas.
- os manifestos sao expostos por rotas JSON para dashboards, CLI e testes.

Esse desenho permite soldar automacao de rede a Omoikane sem transformar a
engine em uma colecao de scripts Python ou contenedores auxiliares.

## RouterOS e RB2011

O suporte a MikroTik fica como geracao de perfil, nao como firmware embutido.
RouterOS e distribuido e licenciado pela MikroTik nos equipamentos; por isso a
Omoikane nao deve vendorizar pacotes, imagens ou binarios RouterOS.

O crate `omoikane_web` inclui `RouterOsRackProfile`, que gera comandos RouterOS
para um RB2011 em rack:

- versao stable esperada: `7.22.2`
- arquitetura esperada: `mipsbe`
- identidade do roteador
- canal de upgrade `stable`
- regra de acesso ao endpoint Omoikane
- NAT para o servidor interno
- FastTrack opcional para trafego estabelecido
- bridge RSTP para perfil de rack

A saida deve ser revisada antes de producao, porque enderecos, interfaces,
politica de firewall e topologia real variam por rack.

## Proximos Cortes

1. Expor configuracao TOML/JSON para bind address, workers e rotas ativas.
2. Adicionar aplicacao real do perfil WireGuard quando houver backend de tunel
   legitimamente configurado no host.
3. Criar benchmark local de `/health`, `/status`, `/metrics` e
   `/database/status` comparando:
   - codec HTTP/1 minimo de `daikoku`
   - borda Actix Web de `omoikane_web`
5. Adicionar um gerador `xtask routeros-rb2011-profile` para materializar o
   script RouterOS em arquivo auditavel.
6. Adicionar modo de TCP real so depois dos testes de contrato ficarem
   confortaveis.

## Regras

- Nao copiar codigo do repositorio Actix Web para dentro da Omoikane; usar
  dependencia versionada e auditar licenca.
- Nao embutir RouterOS ou qualquer firmware proprietario.
- Nao embutir ativadores, cracks, clientes VPN proprietarios ou downloads por
  short-link; overlay deve ser configuracao Rust auditavel.
- Nao deixar `daikoku` depender de Actix Web.
- Todo endpoint deve ler snapshots ou comandos explicitos; nada de mutacao
  acidental de simulacao por rota observacional.
