# Omoikane Renderer Architecture

Data: 2026-05-06

Este documento define o contrato inicial do futuro crate `hikari`. O objetivo
nao e adicionar renderer antes da hora, e sim fixar as fronteiras para que a
Omoikane continue Rust-first, deterministica no estado de simulacao e explicita
no caminho entre mundo, cliente e GPU.

## Papel do `hikari`

`hikari` deve ser a camada grafica da Omoikane. Ele nao deve possuir autoridade
de jogo, nao deve modificar `sekai::EntityManager` diretamente e nao deve
introduzir dependencias circulares com `sekai`, `daikoku` ou `shinobi`.

Responsabilidades esperadas:

- abstrair instancia grafica, device, surface e frames;
- compilar e validar shaders;
- gerenciar buffers, texturas, samplers e pipelines;
- executar render graphs tipados;
- receber dados extraidos do mundo por estruturas proprias de renderizacao;
- fornecer caminhos headless para testes de graph e, depois, testes de imagem.

Responsabilidades fora do escopo:

- regras de simulacao;
- autoridade de rede;
- mutacao de componentes ECS durante submit;
- loading completo de mapas;
- asset pipeline definitivo.

## Fronteiras de Crate

Dependencias permitidas no alvo inicial:

- `hikari -> keisan` para tipos matematicos estaveis;
- `hikari -> sekai` apenas quando houver uma camada de extract bem definida;
- `shinobi -> hikari` para apresentacao client-side;
- nenhum caminho de `sekai`, `daikoku` ou `butsuri` para `hikari`.

A primeira implementacao pode evitar depender de `sekai` e testar somente o
render graph em CPU. Isso mantem o crate pequeno enquanto o contrato amadurece.

## Tipos Minimos

O primeiro corte de API deve conter tipos com responsabilidade estreita:

- `GraphicsInstance`: escolha de backend e capacidades globais;
- `GraphicsDevice`: device logico, queues e limites;
- `SurfaceTarget`: janela, canvas ou alvo headless;
- `FrameContext`: recursos transientes de um frame;
- `GpuBuffer`: dados estruturados, vertices, indices e uniforms;
- `GpuTexture`: imagens, render targets e depth targets;
- `ShaderModule`: modulo validado e identificavel;
- `RenderGraph`: declaracao de passes e recursos.

Esses tipos devem favorecer handles explicitos e validacao antecipada em vez de
acesso global mutavel.

Em 2026-05-05, `hikari` passou a expor esses tipos como contratos estruturais
sem backend real. Eles carregam somente ids, labels, tamanhos e estagios basicos
de shader. Isso permite estabilizar ownership e fronteiras de API antes de
adicionar `wgpu`, `winit`, `naga` ou qualquer integracao de janela.

`GpuBuffer` e `GpuTexture` tambem possuem descritores validaveis em CPU. Buffers
declaram tamanho e usos minimos (`Vertex`, `Index`, `Uniform`, `Storage`,
transferencias), enquanto texturas declaram extent, formato inicial e usos
(`Sampled`, `RenderTarget`, `DepthStencil`, `Storage`, transferencias). Esses
descritores ainda nao sao mapeamentos de API grafica; eles sao contratos para
evitar recursos vazios ou sem finalidade antes de existir backend real.

`ShaderModule` segue a mesma regra: o descriptor registra stage, tipo de fonte
(`Wgsl`, `SpirV`, `RustGpu`), entry point e texto/binario representado como
payload estrutural. A validacao atual exige entry point e fonte nao vazios, mas
nao tenta compilar ou interpretar shader sem uma dependencia propria para isso.

`RenderPipeline` tambem existe como contrato CPU-only. O descriptor liga shaders
por handle e stage, layouts de vertex buffer, formatos de color target e depth
target opcional. A validacao atual confere stage de vertex/fragment, pelo menos
um alvo de saida, formatos compativeis para color/depth targets e layouts de
vertex buffer com stride nao-zero. Ele ainda nao representa estado completo de
rasterizacao, blend, depth ou bind groups.

`BindGroupLayout` descreve os recursos visiveis para shaders sem criar recursos
GPU reais. Cada entrada declara binding, tipo de recurso (`UniformBuffer`,
`StorageBuffer`, `SampledTexture`, `StorageTexture`, `Sampler`) e stages
visiveis. A validacao rejeita layouts vazios, bindings duplicados e entradas sem
visibilidade de shader. `RenderPipelineDescriptor` pode referenciar esses
layouts por handle e rejeita o mesmo layout repetido no pipeline.

`BindGroup` associa um layout a handles de recursos (`GpuBufferId`,
`GpuTextureId`, `GpuSamplerId`) por binding. A validacao estrutural rejeita
grupos vazios e bindings duplicados; a validacao contra `BindGroupLayout`
tambem rejeita recursos faltando, bindings desconhecidos e tipo de recurso
incompativel com o layout.

`RenderCommandList` cobre o primeiro contrato CPU-only para a etapa `queue`.
Ele descreve uma sequencia de comandos estruturais, como iniciar render pass,
definir pipeline, associar bind groups e buffers, desenhar e encerrar o pass.
A validacao ainda nao tenta executar nada em GPU, mas rejeita command lists
vazias, render passes aninhados ou deixados abertos, comandos de draw fora de
pass, draws sem pipeline, draws vazios, `DrawIndexed` sem index buffer e alvos
de render pass ambiguos.

`ComputePipeline` e comandos de compute seguem o mesmo padrao estrutural.
O descriptor exige shader em stage `Compute` e rejeita layouts de bind group
duplicados. A command list tambem pode declarar compute pass, pipeline,
bind groups e dispatches; a validacao rejeita dispatch fora de compute pass,
dispatch sem pipeline, dispatch vazio e compute pass deixado aberto.

`FrameSubmission` fixa a primeira fronteira estrutural para `submit`. Ele
associa um `FrameContext` a uma ou mais `RenderCommandList`, revalida as listas
e rejeita submissao vazia ou command lists repetidas no mesmo frame. Isso ainda
nao representa queue real de GPU; e apenas o contrato de entrada para um futuro
backend.

Em 2026-05-06, `RenderCommandList` e `FrameSubmission` ganharam dumps textuais
estaveis via `debug_dump()`. O dump de command list lista id, label, quantidade
de comandos e cada comando com handles e parametros principais. O dump de
submission lista frame, device, target e command lists submetidas. O formato e
intencionalmente simples para alimentar logs, testes e ferramentas headless
antes de existir backend real.

Em 2026-05-06, `RenderCommandListBuilder`, `RenderPassBuilder` e
`ComputePassBuilder` foram adicionados como camada ergonomica sobre a command
list validada. Eles criam passes por closure, emitem comandos de begin/end
automaticamente, nao deixam comandos parciais quando a closure falha e retornam
erro cedo para casos comuns como draw sem pipeline, `DrawIndexed` sem index
buffer, dispatch sem pipeline e chamadas vazias. A validacao final de
`RenderCommandList` continua sendo a fonte de seguranca para listas montadas
manualmente ou vindas de outro caminho.

No mesmo dia, os diagnosticos de command list passaram a carregar mais contexto
nos erros de draw e dispatch. Falhas como draw sem pipeline, `DrawIndexed` sem
index buffer, dispatch sem pipeline, draw vazio e dispatch vazio agora informam
o comando afetado e, quando aplicavel, o pass ativo. Isso prepara a API para
ferramentas e dumps de debug sem alterar o modelo CPU-only.

`GraphicsResourceCatalog` e um registro CPU-only de handles conhecidos. Ele pode
validar uma `FrameSubmission` contra devices, surfaces, buffers, texturas,
samplers, pipelines e bind groups registrados, rejeitando referencias
desconhecidas antes de qualquer backend grafico existir. Esse catalogo nao e um
resource manager definitivo; por enquanto serve como contrato de sanidade para
submit e testes. Ele tambem preserva usos declarados de buffers/texturas para
rejeitar, por exemplo, texture sem `RenderTarget` usada como alvo de render ou
buffer sem `Vertex` usado como vertex buffer. Quando um pipeline ativo declara
layouts de bind group, o catalogo tambem confere se o bind group associado ao
slot usa o layout esperado. Em 2026-05-06, esses erros de slot/layout tambem
passaram a informar o pipeline render ou compute que declarou a exigencia. Para
draws, o catalogo tambem confere se os slots de vertex buffer exigidos pelo
pipeline ativo foram associados antes do comando.

No app host headless, `CpuFrameResources` usa esse catalogo como registro local
de recursos CPU. Em 2026-05-06, ele passou a aceitar mais de uma textura e mais
de um render pipeline registrados, preservando o alvo/pipeline padrao para o
caminho simples e mantendo as consultas extras sincronizadas com a validacao de
submit.

Ainda em 2026-05-06, o app host ganhou `CpuFrameResourceConfig` e configs
menores para texturas e render pipelines. Essa camada ainda nao e um formato de
arquivo nem asset pipeline, mas ja separa "descriptor vindo do projeto" de
"recurso registrado em CPU", que e a fronteira esperada para loaders futuros.
Em 2026-05-15, o registro persistente tambem passou a aceitar configuracoes
posteriores: texturas e pipelines novos sao incorporados ao catalogo CPU mesmo
depois do primeiro frame registrado.

O primeiro formato serializavel acima dessa camada tambem foi introduzido em
2026-05-06 no app host. `OmoikaneProjectConfig` cobre nome de projeto e
texturas CPU em JSON, converte para `CpuFrameResourceConfig` e valida
duplicidade de ids antes de construir os descriptors de `hikari`. Pipelines e
cenas serializadas permanecem fora desse primeiro corte.
Em 2026-05-15, `ProjectTextureConfig` tambem passou a validar dimensoes
nao-zero antes de criar descriptors CPU, reportando o erro como dado de projeto
em vez de deixar a falha chegar apenas no catalogo grafico.

No mesmo dia, o formato foi expandido para render pipelines. O projeto pode
declarar `ProjectRenderPipelineConfig` com ids de shader, layouts de vertex
buffer, targets e bind group layouts; o app host converte isso para
`CpuRenderPipelineResourceConfig` e deixa a validacao detalhada do descriptor a
cargo de `hikari`.
Em 2026-05-15, esse formato passou a rejeitar vertex buffers com stride zero
como erro de projeto antes de construir o descriptor CPU-only.
O mesmo preflight tambem rejeita pipelines sem target de cor nem depth,
mantendo a falha no dominio do projeto em vez de delegar tudo ao descriptor
grafico.
Ele tambem confere a compatibilidade dos formatos de target: targets de cor
precisam usar formatos de cor, e o target de depth precisa usar formato de
depth.
Bind group layouts declarados em pipelines de projeto tambem passam por
preflight de duplicidade antes de virar descriptor `hikari`.
Slots de vertex buffer duplicados seguem a mesma regra, evitando contratos
ambiguos entre layout de pipeline e buffers submetidos.

O corte seguinte adicionou cenas headless serializaveis. `ProjectSceneConfig`
descreve camera, textura de sandbox, viewport, world view e parametros do
sprite principal, e converte diretamente para `RenderFrameOptions`. Isso ainda
nao substitui um formato completo de cena ou mapa; por enquanto e a ponte
minima entre projeto JSON e o frame CPU-only do sandbox.

Em seguida, cenas passaram a poder declarar sprites estaticos por
`ProjectSpriteConfig`. O app host combina esses sprites com o sprite de sandbox
replicado em `HeadlessApp::build_project_scene_render_extract`, mantendo a
validacao final em `RenderExtract`.

Esse proximo corte tambem foi fechado em 2026-05-06:
`HeadlessApp::build_project_scene_registered_cpu_frame` monta o extract de
cena, preserva os ids serializados de camera/textura, prepara, queueia e valida
a submissao contra `CpuFrameResources`. O metodo so aloca handles transientes de
frame e command list, mantendo recursos persistentes registrados no app host.

Cenas serializadas tambem podem declarar entidades visuais dinamicas com
`ProjectSceneEntityConfig`. O app host cria essas entidades no servidor por
`HeadlessApp::spawn_project_scene_entities`; depois do bombeamento local de
ticks, o extract de cena le posicoes replicadas no cliente e emite sprites com
os parametros visuais declarados no projeto. Isso ainda nao e um sistema de
prefabs completo, mas fecha o caminho minimo projeto -> entidade autoritativa
-> cliente -> render extract.

Esse caminho tambem cobre o primeiro controle local vindo de configuracao de
cena. `ProjectSceneEntityConfig::attach_local_player` pode anexar uma entidade
dinamica ao jogador local no servidor, e `HeadlessApp::handle_local_input`
encaminha comandos pelo cliente local para o loop autoritativo antes que a
posicao replicada volte ao `RenderExtract`. Assim o renderer continua recebendo
apenas dados extraidos, sem assumir autoridade de gameplay.

Entidades dinamicas de cena tambem podem carregar um primeiro bloco de physics
por `ProjectScenePhysicsConfig`. O app host converte esse bloco em configuracao
de corpo e velocidades no servidor antes dos ticks, permitindo que movimento
autoritativo inicial apareca no cliente e no `RenderExtract` sem acoplar
`hikari` aos componentes de simulacao.

O mesmo bloco de physics agora aceita fixtures declarativas por
`ProjectSceneFixtureConfig`. Cenas podem descrever AABBs e circulos simples com
material e bits de colisao; o app host converte esses descriptors em
`butsuri::Fixture` no servidor durante o spawn. O renderer continua vendo apenas
a posicao replicada e os dados visuais extraidos.
Em 2026-05-15, o app host tambem passou a validar ids de fixtures declarativas
antes do spawn, rejeitando ids vazios ou duplicados dentro da mesma entidade
para evitar substituicao silenciosa no componente de fixtures do servidor.
O mesmo caminho valida a geometria das fixtures: AABBs precisam de bounds
finitos e dimensoes positivas, raios de AABB nao podem ser negativos e circulos
precisam de centro finito e raio positivo.
Os valores fisicos declarativos tambem passam por uma barreira antes do spawn:
velocidades linear/angular precisam ser finitas, e propriedades de fixture como
friction, restitution e mass precisam ser finitas e nao negativas antes de
chegar ao servidor autoritativo.

As entidades dinamicas de cena tambem preservam rotacao inicial pelo mesmo
caminho autoritativo. A configuracao serializada alimenta o transform do
servidor, o cliente aplica a rotacao replicada e o app host le esse valor antes
de criar o `SpriteExtract`. Isso mantem o renderer dependente apenas do extract,
mesmo quando os dados visuais derivam de transform replicado.

Cada entidade dinamica de cena possui tambem um id autoral estavel. Esse id nao
substitui o `EntityUid` runtime; ele serve para a camada de projeto consultar a
entidade criada pelo servidor depois do spawn. O app host rejeita ids duplicados
dentro da mesma cena antes de criar entidades, evitando que sistemas futuros de
gameplay apontem para uma entidade ambigua.
Em 2026-05-15, essa barreira passou a rejeitar tambem ids autorais vazios e
referencias `controlled_entity` vazias antes do spawn, mantendo a identidade de
projeto explicita antes de criar entidades runtime.
O mesmo pre-spawn tambem valida a metadata autoral da entidade dinamica:
`appearance_name` vazio e `prototype` vazio quando presente sao rejeitados antes
de escrever metadata ECS no servidor.
O mesmo caminho valida os dados visuais das entidades dinamicas: posicao,
rotacao, tamanho, tint e depth precisam ser finitos, e o tamanho precisa ser
positivo antes de alimentar o servidor autoritativo ou o `RenderExtract`.
O extract de cena tambem ganhou uma barreira propria para dados renderizaveis:
world view, viewport, sprite base da cena e sprites estaticos sao validados
como configuracao de projeto antes que o app host construa o `RenderExtract`.
Quando a cena e transformada em frame CPU registrado, as texturas referenciadas
pelo extract precisam existir em `CpuFrameResources`, antecipando erros de
recurso ausente antes de qualquer backend grafico real.
Essa barreira tambem cobre texturas declaradas por sprites estaticos e
entidades dinamicas da cena, mesmo quando uma entidade dinamica ainda nao foi
replicada para o cliente e, portanto, ainda nao apareceu no `RenderExtract`.
Em 2026-05-15, a mesma checagem passou a cobrir tambem a textura base
`ProjectSceneConfig::sandbox_texture`, mantendo todas as referencias de textura
declaradas pela cena no mesmo preflight de recursos.
Frames CPU sem draws tambem sao reportados pelo app host como
`CpuFrameError::EmptyQueuedFrame`, antes de gerar uma command list com draw de
zero instancias.

A cena tambem pode declarar qual entidade autoral deve ser controlada pelo
jogador local. `ProjectSceneConfig::controlled_entity` referencia um id de
`ProjectSceneEntityConfig`; o app host valida a referencia antes do spawn e
anexa o jogador local no servidor autoritativo. O extract visual continua lendo
apenas o estado replicado do cliente.

No mesmo nivel de autoria, cenas agora podem declarar bindings simples de
input. `ProjectSceneInputBindingConfig` mapeia uma acao de projeto para a
`BoundKeyFunction` runtime usada pelos sistemas de cliente e servidor, e
`HeadlessApp::handle_project_scene_input` resolve esse binding antes de
encaminhar o comando pelo cliente local. Isso mantem o caminho de controle
server-authoritative enquanto evita que o projeto dependa diretamente dos nomes
hardcoded usados pelo runtime atual.

Esses bindings tambem sao validados como dados de projeto: acoes vazias,
funcoes runtime vazias, acoes ausentes e acoes duplicadas sao reportadas como
`ProjectConfigError` no app host. O cliente e o servidor continuam recebendo
apenas comandos resolvidos para uma funcao runtime explicita.
Em 2026-05-15, essa validacao passou a cobrir todos os bindings da cena antes
do lookup da acao pedida, impedindo que uma duplicata nao usada no momento
fique escondida em uma cena aparentemente valida.
O spawn de entidades de cena tambem executa essa validacao antes de criar
entidades no servidor, mantendo cenas com input invalido fora do runtime
autoritativo.

Em 2026-05-15, o exemplo `omoikane_app/examples/headless_sandbox.rs` passou a
usar esse caminho de projeto de ponta a ponta. Ele declara uma cena com
entidade dinamica controlada, resolve input por acao autoral, bombeia o loop
cliente/servidor local e constroi o frame CPU registrado a partir do extract da
cena.

O mesmo exemplo tambem passa pelo formato serializado antes de tocar o runtime:
o `OmoikaneProjectConfig` e emitido como JSON, recarregado e so entao usado para
spawn, input e frame. Isso aproxima o corte headless do fluxo esperado para
projetos authored sem introduzir asset pipeline ou backend grafico real.

## Render Graph

O render graph deve ser testavel sem abrir janela. Passes declaram:

- recursos lidos;
- recursos escritos;
- recursos transientes criados;
- recursos persistentes declarados ou criados;
- recursos preservados entre frames;
- recursos descartados no fim do frame.
- dependencias explicitas de ordenacao entre passes, quando a relacao nao for
  expressa apenas por recursos.

Validacoes obrigatorias:

- recurso lido sem escritor anterior;
- recurso criado duas vezes enquanto ainda esta vivo;
- escrita duplicada sem regra explicita;
- ciclo entre passes;
- pass sem efeito observavel;
- uso conflitante do mesmo recurso em categorias incompativeis no mesmo pass;
- aliasing transiente incompatavel quando o mesmo nome tenta representar
  lifetime transiente e persistente;
- uso depois de descarte.

O primeiro backend do graph pode ser uma representacao em memoria que apenas
ordena passes e valida dependencias. A integracao com `wgpu` deve vir depois.

O resultado de validacao tambem deve servir como material de diagnostico. O
`GraphValidation` atual expoe a ordem de execucao e os lifetimes observados dos
recursos, incluindo tipo (`Imported`, `Transient`, `Persistent`), pass de
criacao, pass de descarte e ultimo pass de uso. Essa informacao deve continuar
independente de backend e pode alimentar ferramentas futuras de debug, dumps ou
visualizacoes do graph.

Em 2026-05-06, `RenderGraph` e `GraphValidation` ganharam dumps textuais
estaveis via `debug_dump()`. O dump do graph lista recursos importados,
persistentes, passes, dependencias e usos por pass. O dump da validacao lista a
ordem de execucao e lifetimes com tipo, criacao, descarte e ultimo uso. Esses
dumps completam o primeiro caminho headless de diagnostico para graph,
submission e command lists.

No mesmo corte, `hikari/examples/cpu_frame_debug.rs` passou a exercitar o fluxo
CPU-only sem janela: graph, extract, prepare, queue, command list, submission,
catalogo de recursos e dumps. O exemplo serve como trilha minima para autores e
para futuras ferramentas validarem o caminho renderer headless antes de existir
backend grafico real.

## Estagios de Frame

O renderer deve preservar uma separacao clara:

- `extract`: copia dados visuais estaveis do mundo/client state;
- `prepare`: resolve buffers, materiais, pipelines e batches;
- `queue`: transforma dados preparados em comandos de render;
- `submit`: envia comandos para GPU ou backend headless.

`extract` pode observar estado de cliente, mas `prepare`, `queue` e `submit`
devem operar em dados proprios do renderer.

Em 2026-05-06, `hikari` passou a expor `RenderExtract` como contrato CPU-only
para o primeiro corte 2D. Ele ainda nao depende de `sekai`: recebe cameras 2D,
sprites, batches de tiles e linhas de debug como dados ja extraidos. A
validacao rejeita camera ausente, camera duplicada, viewport/world view
invalidos, primitivos apontando para camera inexistente, batches de tile vazios
e valores nao finitos ou nao positivos onde isso tornaria a preparacao
ambigua.

No mesmo corte, `PreparedFrame` e `QueuedFrame` passaram a representar os
estagios `prepare` e `queue` ainda sem GPU real. `PreparedFrame` valida o
extract, agrupa sprites por camera/textura/depth, preserva batches de tiles e
agrupa linhas de debug por camera/depth/cor/espessura. `QueuedFrame` transforma
esses dados em draws de alto nivel ordenados por camera, depth, tipo de
primitivo e textura. Essa fila ainda nao cria `RenderCommandList`; ela fixa a
ordem e os lotes que um backend futuro deve consumir.

## Caminho 2D Inicial

O primeiro renderer util deve cobrir:

- sprites;
- tiles;
- linhas de debug;
- retangulos e formas simples para tooling;
- cameras 2D;
- ordenacao por depth/layer.

Esse caminho deve servir para validar o fluxo completo antes de PBR, glTF ou
features GPU-driven.

## Caminho 3D Futuro

O 3D minimo deve vir depois do 2D e do graph:

- mesh com vertex/index buffers;
- camera perspectiva;
- material PBR simples;
- luz direcional;
- subset de glTF;
- teste headless com snapshot ou hash de framebuffer quando possivel.

## Licencas e Dependencias

Antes de adicionar `wgpu`, `winit`, `raw-window-handle`, `bytemuck`, `pollster`
ou `naga`, o projeto deve ter politica de licencas via `cargo-deny` ou revisao
equivalente registrada em `legal.md`.

Assets de exemplo, screenshots, modelos, fontes e texturas devem ter licenca
separada. Licenca de codigo nao deve ser assumida como licenca de asset.

## Marcos Recomendados

1. Criar crate `hikari` sem dependencia grafica externa. Concluido em 2026-05-05.
2. Implementar `RenderGraph` CPU-only com testes de validacao. Concluido em 2026-05-05.
3. Adicionar `xtask architecture-map` e `xtask verify-architecture` ao CI para observar e bloquear acoplamento indevido. Concluido em 2026-05-05.
4. Diferenciar recursos importados, transientes e persistentes no graph. Concluido em 2026-05-05.
5. Adicionar tipos estruturais minimos de graphics sem backend real. Concluido em 2026-05-05.
6. Expor lifetimes de recursos em `GraphValidation` para diagnostico CPU-only. Concluido em 2026-05-05.
7. Adicionar descritores validaveis para buffers e texturas sem backend real. Concluido em 2026-05-05.
8. Adicionar descriptor validavel para shaders sem compilador real. Concluido em 2026-05-05.
9. Adicionar descriptor validavel para render pipeline sem backend real. Concluido em 2026-05-05.
10. Adicionar descriptor validavel para bind group layout sem backend real. Concluido em 2026-05-05.
11. Adicionar descriptor validavel para bind group sem backend real. Concluido em 2026-05-05.
12. Adicionar command list validavel para `queue`/`submit` sem backend real. Concluido em 2026-05-05.
13. Adicionar compute pipeline e dispatch validaveis sem backend real. Concluido em 2026-05-05.
14. Adicionar submissao de frame validavel sem backend real. Concluido em 2026-05-05.
15. Adicionar catalogo CPU-only de recursos para validar handles submetidos. Concluido em 2026-05-05.
16. Validar usos declarados de recursos no catalogo CPU-only. Concluido em 2026-05-05.
17. Validar compatibilidade de bind group layout contra pipeline ativo. Concluido em 2026-05-05.
18. Validar que `DrawIndexed` tenha index buffer associado. Concluido em 2026-05-05.
19. Validar slots de vertex buffer exigidos pelo pipeline ativo. Concluido em 2026-05-06.
20. Adicionar `RenderExtract` CPU-only para camera 2D, sprites, tiles e debug lines. Concluido em 2026-05-06.
21. Adicionar `PreparedFrame` e `QueuedFrame` CPU-only para batches 2D de alto nivel. Concluido em 2026-05-06.
22. Adicionar builders ergonomicos para command list, render pass e compute pass. Concluido em 2026-05-06.
23. Melhorar diagnosticos de command list para draw, index buffer e dispatch. Concluido em 2026-05-06.
24. Melhorar diagnosticos de bind group layout no catalogo com pipeline ativo. Concluido em 2026-05-06.
25. Adicionar dumps textuais de debug para command list e frame submission. Concluido em 2026-05-06.
26. Adicionar dumps textuais de debug para render graph e lifetimes validados. Concluido em 2026-05-06.
27. Adicionar exemplo CPU-only `cpu_frame_debug` para extract/prepare/queue/submit/dumps. Concluido em 2026-05-06.
28. Adicionar politica de licencas antes de qualquer backend real.
29. Integrar `wgpu` somente depois do graph minimo estar coberto por testes.
