# Arquitetura de Controle de Rede da Omoikane

Data: 2026-05-07

Este documento registra a camada de controle de rede da Omoikane no estagio
funcional. O objetivo e fazer a engine subir como servidor operacional, publicar
a propria topologia e preparar automacao de rack sem scripts externos, binarios
opacos ou firmware embutido.

## Crates e Nomes Operacionais

`omoikane_control` permanece Rust puro. Ele contem:

- `OverlayFixedIpProfile`: gera IP overlay fixo em `100.104.0.0/16`, configs
  WireGuard-style e URL publica de status.
- `OmoikaneLaunchConfig` e `OmoikaneLaunchManifest`: descrevem bind, porta,
  tick rate, jogadores, overlay, banco e automacao.
- `KaminariMcpCatalog`: cataloga operacoes NETCONF como ferramentas
  estruturadas e bloqueia escrita por padrao.
- `MamoriPlan`: descreve dispositivos, checks, criticidade e rollback.

`omoikane_web` consome esses contratos para Hayate, Grakane e Michisuji, mas o
crate de controle nao depende de `daikoku`, `omoikane_web`, firmware ou scripts
externos.

## Fluxo de Boot

1. Ler CLI e, opcionalmente, arquivo TOML/JSON.
2. Criar `OmoikaneLaunchConfig`.
3. Gerar `OmoikaneLaunchManifest`.
4. Inicializar `DaikokuServer`.
5. Abrir pool SQLx opcional para PostgreSQL.
6. Imprimir terminal Omoikane com overlay, SQL, metricas e endpoints.
7. Subir loop de tick autoritativo.
8. Subir Hayate com as rotas Omoikane.

O `omoikane.exe` da raiz executa esse fluxo no Windows usando o mesmo binario
Rust do crate web.

## Overlay Fixo

O IP fixo e deterministico:

```text
stable_overlay_ip(seed, server_name) -> 100.104.X.Y
```

O algoritmo usa hash FNV-1a estavel para manter a mesma saida entre execucoes
sem depender de crate externo. A Omoikane gera a configuracao e deixa a parte
fisica do tunel explicita: peers, chaves e endpoints precisam ser legitimos e
fornecidos pelo operador.

## Rotas de Controle

- `/launch`: manifesto completo de lancamento.
- `/network/overlay`: perfil overlay em JSON.
- `/network/overlay/server.conf`: config WireGuard-style do lado servidor.
- `/network/overlay/peer.conf`: bloco peer para cliente ou roteador.
- `/automation/mamori`: plano de checks e rollback.
- `/automation/kaminari/tools`: catalogo NETCONF.
- `/automation/kaminari/rpc/{tool}`: XML NETCONF gerado para ferramenta
  permitida.
- `/automation/michisuji/rb2011.rsc`: script RB2011 revisavel.
- `/database/status`: liveness SQLx quando banco esta configurado.
- `/metrics`: metricas de runtime.
- `/grakane/dashboard.json`: painel Grakane em JSON.

## Comandos de Manutencao

```bash
cargo run -p xtask -- michisuji-rb2011-profile --server 100.104.1.10 --port 8080
cargo run -p xtask -- web-smoke --host 127.0.0.1 --port 8080
cargo run -p xtask -- web-bench --host 127.0.0.1 --port 8080 --path /status --requests 128
```

## Guardrails

- Nao ha download de ativador, cliente proprietario ou firmware.
- Kaminari nasce read-only.
- Escrita NETCONF precisa de perfil explicitamente liberado.
- Mamori valida duplicidade de devices, duplicidade de checks, timeout invalido
  e referencia a device inexistente.
- Overlay exige seed, nome, porta e chaves nao vazias antes de renderizar
  config.
- URLs SQL sao mascaradas no terminal e nos endpoints.
- Grakane e formato de painel proprio da Omoikane, nao copia codigo externo de
  observabilidade.

## Marco Funcional

Os cortes pendentes do controle de rede foram fechados neste marco:

- arquivo TOML/JSON para configuracao do launcher;
- gerador `xtask` de perfil Michisuji/RB2011;
- smoke test ativo contra servidor local;
- benchmark simples de endpoints HTTP;
- renomeacao das superficies externas para nomes Omoikane.
