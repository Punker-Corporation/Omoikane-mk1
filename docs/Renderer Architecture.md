# Omoikane Renderer Architecture

Data: 2026-05-05

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

`GraphicsResourceCatalog` e um registro CPU-only de handles conhecidos. Ele pode
validar uma `FrameSubmission` contra devices, surfaces, buffers, texturas,
samplers, pipelines e bind groups registrados, rejeitando referencias
desconhecidas antes de qualquer backend grafico existir. Esse catalogo nao e um
resource manager definitivo; por enquanto serve como contrato de sanidade para
submit e testes. Ele tambem preserva usos declarados de buffers/texturas para
rejeitar, por exemplo, texture sem `RenderTarget` usada como alvo de render ou
buffer sem `Vertex` usado como vertex buffer. Quando um pipeline ativo declara
layouts de bind group, o catalogo tambem confere se o bind group associado ao
slot usa o layout esperado. Para draws, o catalogo tambem confere se os slots
de vertex buffer exigidos pelo pipeline ativo foram associados antes do comando.

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

## Estagios de Frame

O renderer deve preservar uma separacao clara:

- `extract`: copia dados visuais estaveis do mundo/client state;
- `prepare`: resolve buffers, materiais, pipelines e batches;
- `queue`: transforma dados preparados em comandos de render;
- `submit`: envia comandos para GPU ou backend headless.

`extract` pode observar estado de cliente, mas `prepare`, `queue` e `submit`
devem operar em dados proprios do renderer.

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
20. Adicionar politica de licencas antes de qualquer backend real.
21. Integrar `wgpu` somente depois do graph minimo estar coberto por testes.
