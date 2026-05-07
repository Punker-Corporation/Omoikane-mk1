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
- `omoikane_web`: adaptador Actix Web, rotas HTTP e perfis RouterOS.
- `omoikane_app`: host de aplicacao e cortes verticais locais.

Essa separacao permite usar Actix Web para operacao real sem obrigar os crates
de simulacao, cliente, fisica ou renderer a conhecerem Actix.

## Rotas Actix Iniciais

`configure_omoikane_routes` registra:

- `GET /health` e `GET /healthz`
- `GET /status` e `HEAD /status`
- `GET /status.json` e `HEAD /status.json`

O corpo de `/status` e produzido por `DaikokuServer::status_snapshot`, escrito
pelo mesmo JSON estavel usado pelo nucleo HTTP/1 de `daikoku`. O Actix entra
como camada de transporte, nao como segunda fonte de estado.

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

1. Adicionar binario `omoikane-web` para subir `HttpServer` Actix de verdade.
2. Expor configuracao TOML/JSON para bind address, workers e rotas ativas.
3. Adicionar endpoint de metrica textual sem alocar estado de simulacao.
4. Criar benchmark local de `/health` e `/status` comparando:
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
- Nao deixar `daikoku` depender de Actix Web.
- Todo endpoint deve ler snapshots ou comandos explicitos; nada de mutacao
  acidental de simulacao por rota observacional.
