# Omoikane Roadmap

Data: 2026-05-06

Este documento e o roteiro tecnico de longo prazo para transformar a Omoikane
mk1 em uma engine/runtime Rust-first capaz de sustentar jogos e projetos reais.
Ele deve servir como memoria operacional para futuras sessoes de trabalho,
incluindo trocas de chat, conta ou agente.

O objetivo nao e listar desejos soltos. O objetivo e manter uma sequencia clara
de evolucao, com criterios de pronto, fronteiras de arquitetura e proximas
fatias pequenas o bastante para serem implementadas e validadas.

## Estado Atual

A Omoikane ja possui uma base forte de runtime:

- `keisan`: matematica deterministica, geometria, vetores, matrizes e cores.
- `jikan`: ticks, timers, game loop e tempo de frame.
- `butsuri`: shapes, broadphase, fixtures, contatos, joints, ray queries e
  transform fisico.
- `sekai`: ECS compartilhado, entidades, componentes, mapas, grids, chunks,
  transforms, serializacao, estado fisico compartilhado e game state.
- `daikoku`: servidor autoritativo, filas de rede, snapshots, PVS, inputs,
  fisica e transform autoritativos.
- `shinobi`: cliente, aplicacao de estado, interpolacao, prediction e sistemas
  locais.
- `hikari`: fundacao CPU-only de renderer, render graph, contratos de
  resources, command lists, pipelines e validacao de submit.
- `omoikane_app`: host headless inicial para orquestrar servidor local, cliente
  local e ticks fixos sem janela.
- `xtask`: verificacoes de layout, mapa arquitetural e regras de dependencia.

O projeto esta mais proximo de um runtime multiplayer/simulacao do que de uma
engine de criacao completa. A prioridade dos proximos ciclos deve ser criar um
corte vertical minimo de jogo sem enfraquecer a arquitetura.

## Norte de Produto

A Omoikane deve mirar uma identidade propria:

- Rust-first, sem dependencia de legado C#/.NET.
- Server-authoritative por padrao.
- Cliente com prediction/interpolation como caso de primeira classe.
- Simulacao deterministica ou pelo menos previsivel em dados essenciais.
- ECS e componentes serializaveis como linguagem central do mundo.
- Renderer desacoplado da simulacao, com extract explicito.
- Ferramentas headless e testes de invariantes antes de UI/editor pesado.
- Caminho 2D primeiro, 3D depois.
- Projetos pequenos devem conseguir criar uma cena, rodar um cliente, conectar
  a um servidor local e renderizar entidades sem tocar nos crates internos.

## Principios de Engenharia

1. Preservar fronteiras de crate.
   `sekai`, `daikoku`, `butsuri`, `jikan` e `keisan` nao devem depender de
   `hikari`. `shinobi -> hikari` so deve entrar quando houver extract
   client-side claro. `hikari -> sekai` so deve entrar em uma camada de extract
   bem definida.

2. Preferir contratos estruturais antes de backend real.
   Antes de adicionar uma dependencia externa, definir handles, descritores,
   erros, validacoes e testes CPU-only.

3. Fazer cortes verticais pequenos.
   Cada fase deve entregar algo observavel: uma validacao nova, um exemplo, um
   mapa carregado, uma entidade renderizada, um snapshot replicado.

4. Testar invariantes, nao apenas caminhos felizes.
   Validar erro conhecido, estado invalido, duplicidade, lifecycle, ordem de
   eventos, serializacao e roundtrip.

5. Documentar decisoes enquanto elas ainda estao pequenas.
   Toda nova fronteira de crate, dependencia externa ou mudanca grande de API
   deve atualizar docs.

6. Nao otimizar APIs publicas cedo demais.
   Primeiro estabilizar comportamento e limites internos. Depois desenhar uma
   superficie agradavel para autores de jogo.

7. Manter licencas explicitas.
   Toda dependencia e todo asset precisam de licenca conhecida. Codigo pode
   seguir permissivo; assets precisam de politica propria.

## Regras de Arquitetura Atuais

Regras que devem continuar bloqueadas por `xtask verify-architecture`:

- `sekai`, `daikoku`, `butsuri`, `jikan` e `keisan` nao dependem de `hikari`.
- `daikoku` nao depende de `shinobi`.
- Crates internos nao formam ciclos.
- `hikari` nao ganha dependencias internas alem das explicitamente permitidas
  antes da etapa de extract.

Regras desejadas para fases futuras:

- `omoikane_app` ou crate equivalente pode depender de `sekai`, `daikoku`,
  `shinobi` e `hikari`, mas nao deve ser dependencia de nenhum deles.
- Asset pipeline deve ficar em crate proprio, por exemplo `kura`, sem puxar
  renderer para simulacao.
- Plataforma/janela deve ficar isolada de `sekai` e `daikoku`.
- Editor/tooling deve consumir APIs publicas de runtime, nao campos internos.

## Fase 0: Higiene, CI e Fundacao

Status: em grande parte concluida.

Objetivo:
manter o workspace limpo, testavel e com regressao arquitetural bloqueada.

Concluido:

- Workspace Rust-first organizado em crates.
- `cargo fmt`, `clippy -D warnings` e testes passando.
- `xtask verify-layout`.
- `xtask architecture-map`.
- `xtask verify-architecture`.
- CI executando layout, arquitetura, fmt, clippy, testes e cargo-deny.
- `deny.toml` com politica permissiva inicial.
- Docs de renderer, pesquisa, mapa e servidor.

Proximos ajustes pequenos:

- Adicionar `xtask architecture-map --dot` para gerar grafo visual simples.
- Adicionar `xtask architecture-map --json` apenas se houver consumidor real.
- Melhorar mensagens de erro de `verify-architecture` com nome da regra,
  crate origem, crate destino e sugestao.
- Adicionar um teste que garanta que novos crates precisam aparecer no mapa
  arquitetural.

Criterios de pronto:

- Todas as validacoes rodam local e no CI.
- Qualquer quebra de fronteira aparece como erro legivel.
- Docs indicam como adicionar um crate novo sem violar regras.

## Fase 1: Hikari CPU-Only Completo

Status: em andamento avancado.

Objetivo:
consolidar o renderer como contrato e pipeline validavel, ainda sem `wgpu`,
`winit`, `naga` ou backend grafico real.

Concluido:

- Crate `hikari`.
- `RenderGraph` CPU-only com passes, recursos, dependencias explicitas,
  ordenacao topologica e deteccao de ciclos.
- Recursos importados, transientes e persistentes.
- Lifetimes em `GraphValidation`.
- Tipos estruturais de graphics: instance, device, surface, frame, buffer,
  texture, sampler, shader, pipelines, bind groups, command lists e submit.
- Validacao de usos declarados de recursos.
- Validacao de bind group layout contra pipeline ativo.
- Validacao de `DrawIndexed` com index buffer.
- Validacao de slots de vertex buffer exigidos pelo pipeline ativo.
- `RenderExtract` CPU-only para cameras 2D, sprites, batches de tiles e linhas
  de debug, sem dependencia de `sekai`. Concluido em 2026-05-06.
- `PreparedFrame` e `QueuedFrame` CPU-only para agrupar primitives 2D e ordenar
  draws de alto nivel antes de existir backend real. Concluido em 2026-05-06.
- `RenderCommandListBuilder`, `RenderPassBuilder` e `ComputePassBuilder` para
  montar command lists validas com begin/end automaticos de pass e erros cedo
  para draw/dispatch sem estado minimo. Concluido em 2026-05-06.
- Diagnosticos de command list enriquecidos para draw sem pipeline,
  `DrawIndexed` sem index buffer, dispatch sem pipeline e chamadas vazias,
  incluindo comando e pass ativo quando aplicavel. Concluido em 2026-05-06.
- Diagnosticos do catalogo de submit enriquecidos para erros de bind group
  slot/layout, incluindo o pipeline render ou compute que declarou o layout
  esperado. Concluido em 2026-05-06.
- Dumps textuais estaveis para `RenderCommandList` e `FrameSubmission`, cobrindo
  ids, labels, handles e parametros principais de comandos submetidos.
  Concluido em 2026-05-06.
- Dumps textuais estaveis para `RenderGraph` e `GraphValidation`, cobrindo
  recursos importados/persistentes, passes, dependencias, usos e lifetimes.
  Concluido em 2026-05-06.
- Exemplo `hikari/examples/cpu_frame_debug.rs` exercitando graph, extract,
  prepare, queue, command list, frame submission, catalogo de recursos e dumps
  em modo CPU-only sem janela. Concluido em 2026-05-06.

Proximas fatias:

1. Melhorar diagnosticos de submission e render graph.
   Erros restantes devem incluir command list, pass, binding ou resource sempre
   que possivel.

2. Evoluir entidades visuais de cena para mais componentes/dados de gameplay
   quando o corte visual exigir.

Criterios de pronto:

- `hikari` continua sem dependencias graficas externas.
- Testes cobrem caminhos validos e invalidos de extract, prepare, queue e
  submit.
- O usuario consegue montar um frame 2D de exemplo em CPU e validar tudo sem
  janela.

## Fase 2: App Host e Corte Vertical Headless

Objetivo:
criar uma camada de aplicacao que orquestre runtime, cliente, servidor e frame
sem ainda exigir janela real.

Crate sugerido:

- `omoikane_app` ou `tenkai`.

Responsabilidades:

- Inicializar servidor local e cliente local.
- Rodar ticks fixos.
- Bombear mensagens client/server in-process.
- Expor lifecycle de projeto: init, update, shutdown.
- Carregar uma configuracao minima de mundo.
- Produzir um `RenderExtract` para `hikari`.

Nao responsabilidades:

- Renderer backend real.
- Editor.
- Asset pipeline definitivo.
- Dependencia circular com crates internos.

Fatias:

1. Criar crate app host vazio. Concluido em 2026-05-06 com `omoikane_app`.
2. Definir `App`, `AppOptions`, `AppState` e `GamePlugin` minimo.
   Parcialmente concluido em 2026-05-06 com `HeadlessApp` e
   `HeadlessAppOptions`; `App`, `AppOptions` e `AppState` existem como
   superficie inicial do host headless.
3. Criar exemplo headless `examples/headless_sandbox.rs`. Concluido em
   2026-05-06; o exemplo roda servidor e cliente locais por 5 ticks e valida
   que o cliente aplicou estado autoritativo.
4. Subir servidor e cliente local dentro do app. Concluido em 2026-05-06 com
   bombeamento in-process entre `BaseClient` e `DaikokuServer`.
5. Criar uma entidade com transform e appearance simples. Concluido em
   2026-05-06; `HeadlessApp::spawn_sandbox_entity` cria uma entidade no
   servidor com transform, appearance textual e opcionalmente prende o player
   local nela.
6. Rodar N ticks e verificar que estado final e deterministico. Parcialmente
   concluido em 2026-05-06; testes do `omoikane_app` verificam que a entidade
   de sandbox replica para o cliente com posicao e appearance esperadas.
7. Extrair frame visual CPU-only para `hikari`. Concluido em 2026-05-06;
   `HeadlessApp::build_render_extract` cria camera e sprite a partir da
   entidade de sandbox replicada no cliente, e o exemplo prepara/queueia o
   frame em CPU.
8. Montar submissao `hikari` CPU-only validada pelo catalogo. Concluido em
   2026-05-06; `HeadlessApp::build_cpu_frame` cria recursos estruturais,
   command list, `FrameSubmission` e valida a submissao contra
   `GraphicsResourceCatalog`.
9. Substituir handles fixos do host headless por alocacao simples de handles de
   frame. Concluido em 2026-05-06; `FrameHandleAllocator` aloca ids de extract
   e graphics, e `HeadlessApp::build_allocated_cpu_frame` usa ids distintos por
   frame sem exigir backend grafico.
10. Manter registro persistente de recursos CPU do app. Concluido em
    2026-05-06; `CpuFrameResources` guarda device, surface, target, vertex
    buffer, shaders, pipeline e `GraphicsResourceCatalog`, e
    `HeadlessApp::build_registered_cpu_frame` reutiliza esses recursos entre
    frames enquanto aloca ids transientes de frame e command list.
11. Suportar mais de uma textura e pipeline no registro CPU do app. Concluido
    em 2026-05-06; `CpuFrameResources` agora registra e consulta texturas e
    pipelines extras, mantendo o alvo/pipeline padrao como caminho ergonomico e
    sincronizando tudo com o `GraphicsResourceCatalog`.
12. Alimentar o registro CPU do app por descriptors vindos de configuracao de
    projeto. Concluido em 2026-05-06; `CpuFrameResourceConfig`,
    `CpuTextureResourceConfig` e `CpuRenderPipelineResourceConfig` permitem
    criar `CpuFrameResources` a partir de descriptors estruturais, validando
    recursos extras antes do submit e com atalho em
    `HeadlessApp::build_registered_cpu_frame_from_config`.
13. Definir o primeiro formato serializavel de configuracao de projeto.
    Concluido em 2026-05-06; `OmoikaneProjectConfig` serializa nome do projeto
    e texturas CPU em JSON via `serde`, converte para `CpuFrameResourceConfig`
    e rejeita ids duplicados antes de criar recursos.
14. Expandir o formato serializavel de projeto para render pipelines.
    Concluido em 2026-05-06; `ProjectRenderPipelineConfig` descreve pipelines
    por ids de shader, layouts de vertex buffer, targets e bind group layouts,
    converte para `CpuRenderPipelineResourceConfig` e rejeita ids duplicados no
    nivel de projeto antes da validacao estrutural do `hikari`.
15. Expandir o formato serializavel de projeto para cenas headless. Concluido
    em 2026-05-06; `ProjectSceneConfig` descreve camera, textura de sandbox,
    viewport, world view, tamanho/tint/depth do sprite e converte para
    `RenderFrameOptions`, com deteccao de cena ausente ou nome duplicado.
16. Ligar cenas serializadas a sprites estaticos no extract headless.
    Concluido em 2026-05-06; `ProjectSpriteConfig` descreve textura, posicao,
    tamanho, rotacao, tint e depth, e
    `HeadlessApp::build_project_scene_render_extract` combina o sprite de
    sandbox replicado com sprites declarados pela cena antes de validar o
    `RenderExtract`.
17. Ligar cenas serializadas ao `CpuFrame` registrado completo. Concluido em
    2026-05-06; `HeadlessApp::build_project_scene_registered_cpu_frame`
    reaproveita o extract de cena com sprites estaticos, prepara/queueia o
    frame e valida a submissao usando os recursos CPU persistentes, preservando
    os ids de camera/textura da cena e alocando apenas handles transientes de
    frame e command list.
18. Permitir que cenas serializadas controlem mais de uma entidade visual
    dinamica. Concluido em 2026-05-06; `ProjectSceneEntityConfig` descreve
    entidades visuais com prototype, transform inicial, controle local opcional,
    appearance, textura, tamanho, tint e depth, e
    `HeadlessApp::spawn_project_scene_entities` cria essas entidades no servidor
    para serem replicadas e extraidas no frame de cena.
19. Ligar entidade dinamica de cena ao input local autoritativo. Concluido em
    2026-05-06; `ProjectSceneEntityConfig::attach_local_player` permite que uma
    entidade da cena seja anexada ao jogador local no servidor, enquanto
    `HeadlessApp::handle_local_input` encaminha comandos pelo cliente local para
    o servidor antes da replicacao voltar ao extract visual.
20. Permitir physics inicial em entidades dinamicas de cena. Concluido em
    2026-05-06; `ProjectScenePhysicsConfig` descreve body type, velocidade
    linear/angular, awake, can_collide e predict, e
    `HeadlessApp::spawn_project_scene_entities` aplica esses dados no servidor
    autoritativo antes da simulacao e da replicacao para o extract visual.
21. Permitir fixtures declarativas em entidades dinamicas de cena. Concluido em
    2026-05-06; `ProjectSceneFixtureConfig` descreve AABB/circulo, material e
    bits de colisao dentro de `ProjectScenePhysicsConfig`, e o app host converte
    esses dados em `butsuri::Fixture` no servidor autoritativo durante o spawn.
22. Replicar rotacao inicial de entidades dinamicas ate o extract visual.
    Concluido em 2026-05-06; `ProjectSceneEntityConfig::rotation` alimenta o
    transform criado no servidor, `BaseClient::entity_local_rotation` expoe o
    estado aplicado no cliente, e `HeadlessApp::build_project_scene_render_extract`
    emite sprites dinamicos com a rotacao replicada.
23. Dar identidade autoral estavel para entidades dinamicas de cena. Concluido
    em 2026-05-06; `ProjectSceneEntityConfig::id` identifica a entidade no
    projeto, `HeadlessApp::spawn_project_scene_entities` rejeita ids duplicados
    dentro da mesma cena antes de criar entidades runtime, e
    `HeadlessApp::project_scene_entity` permite consultar o `EntityUid`
    resultante por `(scene, id)`.
24. Permitir que a cena escolha a entidade controlada pelo jogador local por id
    autoral. Concluido em 2026-05-06; `ProjectSceneConfig::controlled_entity`
    referencia um `ProjectSceneEntityConfig::id`, o spawn valida que o id existe
    antes de criar entidades runtime e anexa o jogador local a essa entidade no
    servidor autoritativo.
25. Declarar bindings simples de input por cena. Concluido em 2026-05-06;
    `ProjectSceneInputBindingConfig` mapeia uma acao autoral para uma
    `BoundKeyFunction`, `HeadlessApp::handle_project_scene_input` resolve esse
    binding e envia o input pelo cliente local, e a configuracao rejeita acoes
    ausentes ou duplicadas antes de alcançar o servidor.
26. Validar bindings de input vazios no projeto. Concluido em 2026-05-06;
    acoes autorais vazias e funcoes runtime vazias agora geram
    `ProjectConfigError` no app host antes que comandos invalidos possam ser
    enviados ao cliente local.
27. Atualizar o exemplo headless para usar cena declarativa de projeto.
    Concluido em 2026-05-15; `examples/headless_sandbox.rs` agora monta um
    `OmoikaneProjectConfig`, cria a entidade dinamica controlada no servidor,
    envia input por `ProjectSceneInputBindingConfig` e valida um frame CPU da
    cena registrada em vez de depender apenas do sandbox manual.
28. Fazer o exemplo headless passar pelo formato JSON de projeto. Concluido em
    2026-05-15; `examples/headless_sandbox.rs` serializa o
    `OmoikaneProjectConfig`, recarrega por `OmoikaneProjectConfig::from_json_str`
    e usa a configuracao reidratada para spawn, input autoritativo e frame CPU.
29. Validar todos os bindings de input de cena antes do lookup. Concluido em
    2026-05-15; `ProjectSceneConfig::validate_input_bindings` rejeita acoes
    vazias, funcoes vazias e acoes duplicadas em qualquer binding da cena antes
    de resolver a acao pedida pelo runtime headless.
30. Validar bindings de input antes de spawnar entidades de cena. Concluido em
    2026-05-15; `HeadlessApp::spawn_project_scene_entities` chama
    `ProjectSceneConfig::validate_input_bindings` antes de criar entidades no
    servidor, evitando spawns parciais para cenas com input invalido.
31. Validar ids de fixtures declarativas antes do spawn. Concluido em
    2026-05-15; `ProjectScenePhysicsConfig::validate_fixtures` rejeita ids
    vazios ou duplicados dentro da mesma entidade de cena antes que fixtures
    possam substituir umas as outras no servidor.
32. Validar geometria de fixtures declarativas antes do spawn. Concluido em
    2026-05-15; AABBs precisam de bounds finitos e dimensoes positivas, raios
    de AABB precisam ser finitos e nao negativos, e circulos precisam de centro
    finito e raio positivo.
33. Validar valores fisicos declarativos antes do spawn. Concluido em
    2026-05-15; velocidades linear/angular precisam ser finitas, e propriedades
    de fixture como friction, restitution e mass precisam ser finitas e nao
    negativas antes de tocar o servidor autoritativo.
34. Validar ids autorais vazios de entidades de cena. Concluido em 2026-05-15;
    `ProjectSceneConfig::validate_dynamic_entities` rejeita entidade dinamica
    sem id e `controlled_entity` vazio antes de qualquer spawn no servidor.
35. Validar dados visuais de entidades dinamicas antes do spawn. Concluido em
    2026-05-15; posicao, rotacao, tamanho, tint e depth passam por validacao
    finita/positiva antes de alimentar servidor autoritativo ou render extract.
36. Validar dados de renderizacao de cena antes do extract. Concluido em
    2026-05-15; world view, viewport, sprite base e sprites estaticos de
    `ProjectSceneConfig` sao validados antes de montar o `RenderExtract`.
37. Validar metadata autoral de entidades dinamicas antes do spawn. Concluido
    em 2026-05-15; `appearance_name` vazio e `prototype` vazio quando presente
    sao rejeitados antes de criar metadata ECS no servidor.
38. Validar recursos de textura usados pela cena antes do frame CPU registrado.
    Concluido em 2026-05-15; texturas referenciadas pelo `RenderExtract` da
    cena precisam existir em `CpuFrameResources` antes da submissao CPU.
39. Reportar frames CPU sem draws como erro de app. Concluido em 2026-05-15;
    `CpuFrameError::EmptyQueuedFrame` evita transformar uma cena vazia em
    `DrawCall` com zero instancias.

Criterios de pronto:

- `cargo run -p omoikane_app --example headless_sandbox` ou equivalente roda
  sem janela.
- O exemplo cria mundo, roda ticks e valida uma submissao `hikari`.
  Concluido em 2026-05-06 para o sandbox CPU-only.
- Teste de integracao cobre o loop local.

## Fase 3: Renderer 2D Inicial

Objetivo:
abrir caminho visual real com menor superficie possivel.

Dependencias possiveis, apos revisao de licenca:

- `wgpu`
- `winit`
- `raw-window-handle`
- `pollster` ou runtime async minimo
- `bytemuck`
- `naga`, se necessario diretamente

Ordem recomendada:

1. Atualizar `legal.md` com dependencias graficas planejadas.
2. Rodar/validar `cargo deny` via CI e, se possivel, local.
3. Adicionar feature `hikari/wgpu`.
4. Criar backend `WgpuGraphicsBackend` atras de trait ou modulo isolado.
5. Criar janela minima em crate de app/platform, nao dentro de `sekai`.
6. Renderizar clear color por frame.
7. Renderizar linhas de debug.
8. Renderizar retangulos/quads.
9. Renderizar sprites com textura simples.
10. Renderizar tile grid.
11. Integrar camera 2D.
12. Integrar ordenacao por `DrawDepth`/layer.

Criterios de pronto:

- Exemplo `local_2d_sandbox` abre janela.
- Uma entidade do mundo aparece como sprite/quad.
- Um grid de tiles aparece na tela.
- A simulacao continua independente do renderer.
- Existe fallback/teste headless para CI.

## Fase 4: Asset Pipeline Minimo

Objetivo:
permitir que projetos carreguem conteudo sem hardcode em Rust.

Crate sugerido:

- `kura` para assets, manifests e cache.

Tipos iniciais:

- `AssetId`
- `AssetPath`
- `AssetManifest`
- `TextureAsset`
- `SpriteAsset`
- `TileSetAsset`
- `MapAsset`

Fatias:

1. Manifest TOML/JSON simples com ids estaveis.
2. Loader de textura PNG permissivo, apos licenca/dependencia.
3. Loader de spritesheet.
4. Loader/saver de mapa usando formato documentado.
5. Cache content-addressed para assets processados.
6. Erros com path, asset id e causa.
7. Testes de roundtrip de manifest e mapa.

Criterios de pronto:

- Exemplo 2D carrega sprites e tiles por manifest.
- Mapas roundtripam sem perda de dados essenciais.
- Assets de exemplo tem licenca clara.

## Fase 5: API de Projeto e Ergonomia

Objetivo:
fazer a engine ser usavel por um autor de jogo, nao apenas por quem conhece os
crates internos.

Superficie desejada:

- `GamePlugin`
- `WorldCommands`
- `EntityCommands`
- Registro de componentes customizados.
- Registro de sistemas por stage.
- Eventos typed.
- Configuracao de input.
- Configuracao de camera.
- Spawn de entidades com bundles simples.

Fatias:

1. Definir stages publicos: startup, fixed_update, update, extract, render.
2. Criar comandos de spawn/despawn/add_component.
3. Criar bundles para sprite, physics body, map/grid e camera.
4. Criar exemplo `pong` ou `top_down_mover`.
5. Documentar "primeiro projeto Omoikane".

Criterios de pronto:

- Um jogo pequeno nao precisa importar diretamente managers internos.
- O exemplo tem menos boilerplate que montar `EntityManager` manualmente.
- A API nao quebra server-authority nem serializacao.

## Fase 6: Networking Jogavel

Objetivo:
transformar a base client/server em demo multiplayer real.

Fatias:

1. Definir transporte inicial.
   Pode comecar in-process/local loopback e depois UDP/QUIC/WebSocket.

2. Criar servidor local executavel.
   Deve aceitar conexoes, criar sessoes e publicar status simples.

3. Criar cliente executavel.
   Deve conectar, enviar input, receber snapshots e renderizar estado.

4. Criar protocolo de debug.
   Logs de tick, ack, snapshot size, lost/late packets e prediction errors.

5. Adicionar reconciliation visivel.
   Ferramenta/debug overlay para divergencia entre predicted e authoritative.

Criterios de pronto:

- Dois clientes locais conectam em um servidor.
- Movimento autoritativo replica entre clientes.
- Prediction local e correcao funcionam em demo simples.
- Testes cobrem ordem de ticks, ack e deltas.

## Fase 7: Tooling e Debug Views

Objetivo:
dar visibilidade ao runtime antes de construir um editor completo.

Ferramentas iniciais:

- Overlay de FPS/tick.
- Debug draw de colliders.
- Visualizacao de broadphase.
- Visualizacao de PVS por jogador.
- Dump de entity/component state.
- Dump de render graph.
- Inspector headless via CLI.

Fatias:

1. Adicionar debug line/shape extraction.
2. Adicionar comandos `xtask inspect-map`, `xtask inspect-save` ou similares.
3. Adicionar modo de debug no app host.
4. Exportar graph/submission como texto.
5. Criar screenshots de teste quando renderer real existir.

Criterios de pronto:

- Uma falha de simulacao ou renderizacao pode ser investigada sem debugger
  pesado.
- Debug views nao mutam estado de jogo.

## Fase 8: Editor Leve

Objetivo:
criar ferramentas de autoria sem travar a engine em uma arquitetura de editor
prematura.

Abordagem:

- Primeiro editor externo/leitor de mapa.
- Depois inspector runtime.
- Por ultimo edicao live com undo/redo.

Fatias:

1. Abrir mapa e listar grids/chunks/entities.
2. Visualizar tile grid.
3. Selecionar entidade.
4. Editar transform e appearance.
5. Salvar mapa com roundtrip.
6. Undo/redo de comandos de edicao.
7. Integrar asset manifest.

Criterios de pronto:

- Criar um mapa simples sem escrever Rust.
- Salvar e carregar preserva entidades, tiles e componentes.
- Editor usa APIs publicas, nao campos internos.

## Fase 9: 3D Minimo

Objetivo:
adicionar 3D apenas depois do 2D estar provado.

Fatias:

1. Mesh com vertex/index buffers.
2. Camera perspectiva.
3. Material basico.
4. Luz direcional.
5. Depth buffer.
6. Loader glTF subset.
7. PBR simples.
8. Debug draw 3D.

Criterios de pronto:

- Exemplo abre cena 3D minima.
- Teste headless ou snapshot cobre pipeline basico.
- 3D nao quebra caminho 2D.

## Fase 10: Performance, Escala e Medicao

Objetivo:
transformar corretude em desempenho medido.

Areas:

- ECS storage e query performance.
- Serialization bandwidth.
- Snapshot delta size.
- Broadphase e physics step.
- Render extraction cost.
- GPU upload bandwidth.
- Frame time budgets.

Fatias:

1. Criar benches com `criterion` ou alternativa permitida.
2. Medir spawn/despawn massivo.
3. Medir query de componentes.
4. Medir serializacao de snapshots.
5. Medir broadphase queries.
6. Medir render extract de muitos sprites.
7. Adicionar relatorio de benchmark em CI opcional.

Criterios de pronto:

- Otimizacoes sao guiadas por numeros.
- Regressao critica tem benchmark ou teste.

## Backlog por Subsistema

### ECS e Mundo

- Definir invariantes formais de lifecycle de entidade/componente.
- Melhorar storage para consultas por assinatura quando necessario.
- Separar API publica de autoria dos managers internos.
- Criar comandos transacionais de mundo.
- Melhorar snapshots e deltas com diagnostico de tamanho.
- Adicionar fuzz/roundtrip para serializacao de componentes.

### Mapas

- Fechar formato de mapa v1.
- Loader/saver deterministicos.
- Content-addressed chunks.
- Streaming de chunks.
- Ferramentas de inspeccao.
- Edicao basica de tiles.

### Fisica

- Medir broadphase.
- Documentar determinismo esperado.
- Melhorar joints e contato conforme demos exigirem.
- Debug draw de colliders e contatos.
- Comparar comportamento com Rapier apenas como referencia/benchmark.

### Networking

- Escolher transporte inicial.
- Formalizar protocolo e versao.
- Medir tamanho de snapshot.
- Simular latencia/perda em teste.
- Expor telemetria de prediction/reconciliation.

### Renderer

- Finalizar extract/prepare/queue/submit CPU-only.
- Backend `wgpu` isolado.
- Renderer 2D.
- Debug draw.
- Headless tests.
- Renderer 3D minimo.

### Assets

- Manifest.
- Loader de textura/sprite/tilemap.
- Licenca de assets de exemplo.
- Cache processado.
- Hot reload futuro.

### Tooling

- `xtask` para mapas, arquitetura, auditoria e dumps.
- Debug overlays.
- Inspector.
- Editor leve.

## Sequencia Recomendada das Proximas 10 Rodadas

1. Criar crate app host headless com loop local server/client.
2. Criar exemplo headless que spawna entidade, roda ticks e extrai frame.
3. Definir API minima de projeto/plugin.
4. Adicionar renderer backend feature-gated com `wgpu` apenas apos revisar
   licencas e CI.
5. Abrir janela e renderizar clear color.
6. Renderizar debug lines/quads.
7. Renderizar sprite/tile de uma entidade real do mundo.
8. Adicionar overlay de diagnostico para extract/prepare/queue.
9. Criar app host headless com loop server/client local e um frame extraido.
10. Melhorar diagnosticos de command list, submission e render graph.

## Criterios de Qualidade Para Cada PR

Toda PR deve responder:

- Qual subsistema mudou?
- Qual fronteira de crate foi afetada?
- Quais invariantes foram adicionadas ou preservadas?
- Que testes cobrem o novo comportamento?
- Que comando valida a mudanca?
- Alguma dependencia/licenca/asset novo entrou?
- A documentacao relevante foi atualizada?

Checks padrao:

```powershell
cargo fmt --all -- --check
$env:CARGO_TARGET_DIR='target-pr-check'
cargo run -p xtask -- verify-layout
cargo run -p xtask -- verify-architecture
cargo run -p xtask -- architecture-map
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets -j1
```

Depois, limpar o target temporario com checagem de caminho:

```powershell
$repo = (Resolve-Path .).Path
$target = (Resolve-Path target-pr-check).Path
if ((Split-Path -Leaf $target) -ne 'target-pr-check') { throw "unexpected target leaf: $target" }
if (-not $target.StartsWith($repo, [System.StringComparison]::OrdinalIgnoreCase)) { throw "target outside repo: $target" }
Remove-Item -LiteralPath $target -Recurse -Force
```

## Politica Para Futuras Sessoes Codex

Quando uma nova sessao assumir o trabalho, ela deve:

1. Ler este documento.
2. Ler `README.md`.
3. Ler `docs/Renderer Architecture.md`.
4. Rodar `git status -sb`.
5. Identificar a fase ativa.
6. Escolher uma fatia pequena da secao "Sequencia Recomendada".
7. Implementar com testes.
8. Rodar validacao apropriada.
9. Atualizar docs e este roadmap quando concluir um marco.

Evitar:

- Adicionar backend grafico antes do contrato CPU-only estar confortavel.
- Fazer refactors grandes sem teste de comportamento.
- Misturar editor, renderer, assets e networking na mesma PR.
- Introduzir dependencia externa sem atualizar `legal.md`/`deny.toml`.
- Fazer `git reset --hard` ou reverter mudancas locais sem pedido explicito.

## Prompt de Continuidade

Use este bloco quando precisar continuar em outro chat ou conta:

```text
Voce esta trabalhando no repositorio Omoikane-mk1-fresh, uma engine/runtime
experimental Rust-first. Leia primeiro:

- README.md
- docs/Omoikane Roadmap.md
- docs/Renderer Architecture.md
- docs/Research Intake.md

Estado arquitetural:

- keisan: matematica deterministica.
- jikan: ticks/timers/game loop.
- butsuri: fisica/collision/broadphase/raycast.
- sekai: ECS compartilhado, mapas, transforms, serialization e game state.
- daikoku: servidor autoritativo.
- shinobi: cliente, prediction/interpolation.
- hikari: renderer CPU-only, render graph, resources, command lists, pipelines,
  frame submissions e catalogo de validacao.
- xtask: verify-layout, architecture-map e verify-architecture.

Regras:

- Nao adicionar dependencias graficas externas sem atualizar licencas e docs.
- Preservar fronteiras de crate; sekai/daikoku/butsuri/jikan/keisan nao devem
  depender de hikari.
- Trabalhar em fatias pequenas, com testes e docs.
- Usar target temporario para validacao no Windows e limpar depois com checagem
  de caminho.

Proxima direcao recomendada:

1. Finalizar a fase `hikari` CPU-only com RenderExtract, PreparedFrame,
   QueuedFrame e builders ergonomicos.
2. Depois criar um app host headless com loop server/client local.
3. Depois integrar renderer 2D real com backend isolado e feature-gated.

Validacao padrao:

cargo fmt --all -- --check
cargo run -p xtask -- verify-layout
cargo run -p xtask -- verify-architecture
cargo run -p xtask -- architecture-map
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets -j1
```

## Definicao de Sucesso

A Omoikane mk1 chega a um primeiro marco publico quando for possivel:

1. Criar um projeto minimo.
2. Rodar servidor local.
3. Abrir cliente com janela.
4. Carregar mapa simples.
5. Renderizar tiles/sprites.
6. Controlar uma entidade com input.
7. Replicar movimento via servidor autoritativo.
8. Ver debug overlay de tick/FPS/network.
9. Salvar/carregar mapa sem perda.
10. Explicar a arquitetura em poucos documentos atualizados.

Esse e o ponto em que a engine deixa de ser apenas uma base excelente e passa a
ser uma ferramenta real de criacao.
