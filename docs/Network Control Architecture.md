# Omoikane Network Control Architecture

Data: 2026-05-07

Este documento registra a solda de controle de rede da Omoikane. O objetivo e
fazer a engine subir como servidor operacional, publicar a propria topologia e
preparar automacao de rack sem depender de scripts externos, containers
auxiliares ou binarios opacos.

## Crate

`omoikane_control` e Rust puro e nao adiciona dependencias externas. Ele contem:

- `OverlayFixedIpProfile`: gera IP overlay fixo dentro de `100.104.0.0/16`,
  configs WireGuard-style e URL publica de status.
- `OmoikaneLaunchConfig` e `OmoikaneLaunchManifest`: descrevem bind, porta,
  tick rate, max players, overlay, rack e automacao.
- `JunosMcpCatalog`: cataloga operacoes NETCONF como ferramentas estruturadas,
  bloqueando escrita por padrao.
- `NetworkRobotPlan`: descreve devices, checks e rollback em formato nativo.

O crate e consumido por `omoikane_web`, mas nao depende de `daikoku`, Actix,
RouterOS, Python ou firmware de rede.

## Launcher

O binario `omoikane-server` fica em `omoikane_web` porque ele precisa de Actix e
do servidor autoritativo. O fluxo de boot e:

1. Ler argumentos simples de CLI.
2. Criar `OmoikaneLaunchConfig`.
3. Gerar `OmoikaneLaunchManifest`.
4. Inicializar `DaikokuServer`.
5. Imprimir o terminal Omoikane com IP overlay e endpoints.
6. Subir `actix_web::HttpServer` com as rotas Omoikane.

Esse terminal nao e apenas estetico. Ele e a primeira superficie operacional da
engine, reunindo status local, status overlay, perfil de rack e plano de
aceitacao em um lugar so.

## Overlay Fixo

O IP fixo e deterministico:

```text
stable_overlay_ip(seed, server_name) -> 100.104.X.Y
```

O algoritmo usa hash FNV-1a estavel para evitar dependencia externa e manter a
mesma saida entre execucoes. Isso resolve identidade local e configuracao de
tunel. Ele nao promete obter IP publico sozinho, porque isso exige um peer,
relay, provedor ou endpoint real. A Omoikane gera a configuracao e deixa a parte
fisica do tunel explicita e auditavel.

## Rotas

`omoikane_web` expoe:

- `/launch`: manifesto completo de lancamento.
- `/network/overlay`: perfil overlay em JSON.
- `/network/overlay/server.conf`: config WireGuard-style do lado servidor.
- `/network/overlay/peer.conf`: bloco peer para cliente/roteador.
- `/automation/robot`: plano de checks e rollback.
- `/automation/junos/tools`: catalogo de operacoes NETCONF.
- `/automation/junos/rpc/{tool}`: XML NETCONF gerado para uma ferramenta
  permitida.

## Guardrails

- Nao ha download de ativador, crack, cliente VPN proprietario ou binario de
  firmware.
- O catalogo NETCONF nasce read-only.
- Operacoes de escrita precisam de perfil explicitamente liberado.
- O plano de automacao valida devices duplicados, checks duplicados, timeout
  invalido e referencia a device inexistente.
- O overlay exige seed, nome, porta e chaves nao vazias antes de renderizar
  config.

## Proximos Cortes

1. Adicionar suporte a arquivo de configuracao TOML/JSON para o launcher.
2. Gerar arquivos de perfil em `xtask` para aplicar em laboratorio.
3. Criar checks ativos contra o servidor local em teste de integracao.
4. Adicionar um backend opcional de tunnel apply que apenas chame ferramentas
   legitimas ja instaladas pelo operador.
5. Medir latencia de `/status` local e overlay quando houver ambiente real.
