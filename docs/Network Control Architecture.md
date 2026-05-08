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
- `OmoikaneDnsPlan`: gera lista DNS/local/overlay para publicacao automatica
  dos links.
- `OmoikanePublicationPlan`: gera subservidores, registros DNS pretendidos,
  scripts RouterOS/Junos de resolvedor local e blueprint VPS/Reality.
- `OmoikaneLaunchConfig` e `OmoikaneLaunchManifest`: descrevem bind, porta,
  tick rate, jogadores, overlay, DNS, Gmail do Grakane, budget anti-DDoS,
  banco e automacao.
- `KaminariMcpCatalog`: cataloga operacoes NETCONF como ferramentas
  estruturadas e bloqueia escrita por padrao.
- `MamoriPlan`: descreve dispositivos, checks, criticidade e rollback.

`omoikane_web` consome esses contratos para Hayate, Grakane e Michisuji, mas o
crate de controle nao depende de `daikoku`, `omoikane_web`, firmware ou scripts
externos.

## Fluxo de Boot

1. Ler CLI e, opcionalmente, arquivo TOML/JSON.
2. Criar `OmoikaneLaunchConfig`.
3. Em duplo clique, abrir terminal Mikado para escolher IP, DNS e Gmail do
   Grakane.
4. Gerar `OmoikaneLaunchManifest`.
5. Inicializar `DaikokuServer`.
6. Abrir pool SQLx opcional para PostgreSQL.
7. Imprimir terminal Omoikane com overlay, DNS, SQL, firewall, metricas e
   endpoints.
8. Subir loop de tick autoritativo.
9. Subir Hayate com as rotas Omoikane.

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

## DNS, Subservidores e Publicacao

O DNS automatico agora tem duas camadas:

1. `OmoikaneDnsPlan` escolhe a identidade de acesso do servidor.
2. `OmoikanePublicationPlan` transforma essa identidade em subservidores e
   registros de publicacao.

Quando `public_dns_name` e um dominio publicavel, o plano gera um registro apex
para `public_dns_target` e CNAMEs para `console`, `status`, `metrics`, `game` e
`vps`. Quando o host escolhido e local, IP cru ou `.localhost`, o plano nao
finge DNS publico: ele retorna scripts com comentarios e deixa o operador ver
que nao ha registros autoritativos aplicaveis.

RouterOS e Junos entram como saidas auditaveis:

- RouterOS recebe entradas em `/ip dns static`, adequadas para resolvedor local
  do rack.
- Junos recebe `set system static-host-mapping` para registros A/AAAA e comenta
  CNAMEs, porque static host mapping nao substitui DNS autoritativo.

Essas saidas nao registram dominio em provedor externo. Elas ajudam a preparar
o rack, validar nomes localmente e reduzir erro operacional antes da mudanca no
DNS real.

O site publico de teste fica em `/site`. A raiz `/` so entrega esse site quando
o header `Host` bate com `public_dns_name`; acessos locais continuam vendo o
console Mikado.

O blueprint VPS/Reality registra a opcao de rodar a Omoikane atras de uma borda
VPS gerenciada pelo operador. O manifesto publica host, porta, SNI e entradas
necessarias, mas nao baixa, instala ou empacota Xray, 3x-ui, chaves ou scripts
de terceiros.

## Rotas de Controle

- `/launch`: manifesto completo de lancamento.
- `/network/dns`: plano DNS automatico e escolhas disponiveis.
- `/network/publication`: plano de publicacao global, subservidores e DNS.
- `/network/dns/routeros.rsc`: script RouterOS para entradas DNS locais.
- `/network/dns/junos.set`: comandos Junos de static host mapping.
- `/network/overlay`: perfil overlay em JSON.
- `/network/overlay/server.conf`: config WireGuard-style do lado servidor.
- `/network/overlay/peer.conf`: bloco peer para cliente ou roteador.
- `/automation/mamori`: plano de checks e rollback.
- `/automation/kaminari/tools`: catalogo NETCONF.
- `/automation/kaminari/rpc/{tool}`: XML NETCONF gerado para ferramenta
  permitida.
- `/automation/michisuji/rb2011.rsc`: script RB2011 revisavel.
- `/database/status`: liveness SQLx quando banco esta configurado.
- `/security/status`: budget anti-DDoS e contadores do guardiao HTTP.
- `/security/monitoring`: Sentinel com DNS watch e forense da maquina host.
- `/metrics`: metricas de runtime.
- `/grakane/dashboard.json`: painel Grakane em JSON.
- `/servers`: lista de subservidores publicados ou planejados.
- `/vps/reality-blueprint`: contrato VPS/VLESS Reality sem instalador externo.
- `/` e `/console`: console unico Mikado para operacao humana.
- `/site`: site publico de teste.

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
- Grakane pode ser protegido por `grakane_admin_gmail`; o console exige o Gmail
  configurado no host para liberar a pagina.
- O arquivo de firmware ou pacote de roteador que existir localmente nao entra
  no manifesto, nao e convertido e nao e empacotado. Michisuji representa a
  interface de configuracao em Rust puro.
- Arquivos `*.npk` ficam ignorados pelo git para evitar que firmware RouterOS
  local entre em PR por acidente.
- O plano VPS/Reality e somente blueprint. Credenciais, chaves, UUIDs,
  politicas de uso e instalacao de Xray ficam fora do repositorio.

## Marco Funcional

Os cortes pendentes do controle de rede foram fechados neste marco:

- arquivo TOML/JSON para configuracao do launcher;
- terminal Mikado com selecao de IP, DNS e Gmail;
- console unico em HTML para navegador;
- plano DNS automatico Rust-native;
- plano de publicacao global com subservidores, scripts RouterOS/Junos e site
  publico de teste;
- guardiao anti-DDoS e Sentinel;
- gerador `xtask` de perfil Michisuji/RB2011;
- smoke test ativo contra servidor local;
- benchmark simples de endpoints HTTP;
- renomeacao das superficies externas para nomes Omoikane.
