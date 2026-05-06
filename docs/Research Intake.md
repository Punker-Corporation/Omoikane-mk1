# Omoikane Research Intake

Data: 2026-05-03

Este documento registra a varredura inicial de artigos, TCCs, teses, talks tecnicos e repositorios Rust que podem orientar a evolucao da Omoikane. A regra de aproveitamento e simples: codigo so entra se a licenca permitir e se a integracao mantiver o projeto 100% Rust; ideias de artigos entram por reimplementacao propria; textos, figuras e assets de trabalhos academicos entram apenas como referencia citada, salvo licenca explicita permitindo reutilizacao.

## Regras de aproveitamento

- Dependencia direta: aceitar preferencialmente `MIT`, `Apache-2.0`, `MIT OR Apache-2.0`, `BSD`, `Zlib` ou `CC0`, com registro em `legal.md` quando virar dependencia real.
- Reimplementacao propria: usar papers, TCCs e talks para derivar arquitetura, algoritmos e testes, sem copiar texto, diagramas ou codigo.
- Somente referencia: materiais com copyright reservado, licencas `NC`, `ND`, paginas pagas ou assets com licenca separada.
- Antes de copiar qualquer trecho de codigo: confirmar licenca do arquivo especifico, preservar copyright quando exigido, preferir dependencia publicada em crate, e manter a API da Omoikane idiomatica.
- Antes de trazer assets: exigir licenca de asset separada; nao assumir que a licenca do repositorio cobre texturas, fontes, cenas, modelos ou screenshots.

## Literatura e material tecnico

| Fonte | Tipo | Ideia aproveitavel | Acao para a Omoikane |
| --- | --- | --- | --- |
| [SyDRA: An Approach to Understand Game Engine Architecture](https://arxiv.org/abs/2406.05487) | Artigo | Recuperacao de dependencias entre subsistemas, deteccao de acoplamento excessivo e nesting de pastas. | Criar um `xtask architecture-map` para gerar grafo de crates, modulos e dependencias internas. |
| [Visualising Game Engine Subsystem Coupling](https://arxiv.org/abs/2309.06329) | Artigo | Padroes recorrentes de acoplamento entre core, renderer, plataforma e recursos. | Manter renderer, asset pipeline, plataforma e simulacao como fronteiras explicitas; bloquear dependencia circular via CI. |
| [Are Game Engines Software Frameworks?](https://arxiv.org/abs/2004.05705) | Artigo | Engines tem perfil diferente de frameworks comuns: maior complexidade, releases curtas, motivacao por controle fino. | Priorizar modularidade e contratos internos mais estaveis que APIs externas prematuras. |
| [The Data-Oriented Design Process for Game Development](https://doi.org/10.1109/MC.2022.3155108) | Artigo | Data-oriented design como processo: entender dados, fluxo, hardware, cache e compilador antes de abstrair. | Converter ECS e renderer para pipelines medidos por layout de dados, nao por hierarquia de objetos. |
| [Implementation and Analysis of the Entity Component System Architecture](https://digitalcommons.calpoly.edu/theses/2389/) | Dissertacao | Benchmarks de ECS contra arquiteturas orientadas a objeto, com foco em cache e processamento massivo. | Criar benches de archetype/storage para `sekai`, comparando `Vec`, `HashMap`, sparse set e slab. |
| [Advantages and Implementation of Entity-Component-Systems](https://trepo.tuni.fi/handle/123456789/27593) | Tese | Separacao de entidades, componentes e sistemas como caminho para evitar encapsulamento pesado e cache ruim. | Documentar invariantes do ECS da Omoikane e evoluir armazenamento para consultas por assinatura. |
| [Ganhos de performance com a utilizacao da arquitetura de Entity Component Systems](https://repositorio.ufms.br/handle/123456789/9780) | TCC | Discussao em portugues sobre ECS, hardware e simulacoes em tempo real. | Usar como referencia didatica interna; nao copiar texto ou conteudo por copyright reservado. |
| [Game engine architecture: A comprehensive view](https://digitalcommons.njit.edu/theses/312/) | Tese | Visao ampla de subsistemas de engine generica. | Usar como checklist de cobertura: renderer, audio, input, recursos, fisica, scripting, tooling. |
| [3D Game Builder: Uma Game Engine para a criacao de jogos 3D](https://edirlei.com/papers/3d_game_builder_tcc.pdf) | TCC | Divisao em sub-engines: grafica, fisica, script e demais subsistemas. | Aproveitar a taxonomia de subsistemas como referencia historica, com implementacao Rust propria. |
| [Estudo comparativo de Motores de jogos no desenvolvimento de jogo 2D para web](http://repositorio.ufc.br/handle/riufc/70844) | TCC | Comparacao de motores por custo-beneficio, plataformas e facilidade para web. | Criar criterio de qualidade da Omoikane: web, desktop, ergonomia, build, documentacao e runtime previsivel. |
| [Parallelizing the Naughty Dog Engine Using Fibers](https://media.gdcvault.com/gdc2015/presentations/Gyrling_Christian_Parallelizing_The_Naughty.pdf) | Talk tecnico | Job system com filas por prioridade, jobs que podem esperar, pipeline frame-centric e multiplos frames em voo. | Implementar um job graph Rust com stages `simulate`, `extract`, `prepare`, `submit`; usar futures/tasks em vez de fibers nativas. |
| [FrameGraph: Extensible Rendering Architecture in Frostbite](https://www.gdcvault.com/play/1024612/FrameGraph-Extensible-RenderingArc) | Talk tecnico | Grafo de passes e recursos para renderizacao modular sem perder eficiencia. | Criar crate de render graph tipado com recursos versionados, aliasing transiente e validacao de dependencias. |
| [GPU-Driven Rendering Pipelines](https://www.advances.realtimerendering.com/s2015/index.html) | Talk tecnico | Pipeline GPU-driven para cenas complexas, draw indirect, culling e reducao de trabalho CPU. | Projetar renderer com comandos indiretos, buffers persistentes e extracao minimizada do mundo. |
| [A GPU-friendly hybrid occlusion culling algorithm for large scenes](https://doi.org/10.1016/j.displa.2023.102533) | Artigo | Culling hibrido: hierarchical Z em compute, rasterizacao fina, indirect multidraw sem readback CPU. | Adicionar backlog de HZB compute e visibilidade por BVH para cenas grandes. |
| [Integrating Occlusion Culling into LOD on GPU](https://doi.org/10.2312/pgs.20141246) | Artigo | Integracao de occlusion culling e LOD na GPU para reduzir triangulos e memoria. | Unificar `VisibilitySet`, LOD e chunk streaming no futuro renderer. |
| [Virtual Texturing](https://arxiv.org/abs/1005.3163) | Tese | Sistema de virtual texturing e toolchain para texturas massivas. | Planejar asset pipeline com paginas de textura, cache e manifest content-addressed. |
| [Real-Time Cloth Simulation Using WebGPU](https://arxiv.org/abs/2507.11794) | Artigo | WebGPU compute para simulacao mass-spring, colisao e escala alta de nos. | Criar prova de conceito de compute physics opcional sobre `wgpu`. |
| [RTGPU: Real-Time Computing with Graphics Processing Units](https://arxiv.org/abs/2507.06069) | Survey | Variabilidade temporal, contencao e desafios de previsibilidade em GPU. | Adicionar telemetria de deadline e budgets por pass de GPU; evitar depender de latencias nao medidas. |
| [DarthShader: Fuzzing WebGPU Shader Translators & Compilers](https://arxiv.org/abs/2409.01824) | Artigo | Fuzzing em pipeline de shader WebGPU, mutacao por AST/IR e cobertura. | Criar fuzz tests para shader preprocessing, WGSL/Rust-GPU linkage e conversoes de recursos. |

## Varredura de repositorios Rust

| Repositorio | Licenca observada | Estado | O que podemos aproveitar | Acao recomendada |
| --- | --- | --- | --- | --- |
| [gfx-rs/wgpu](https://github.com/gfx-rs/wgpu) | `MIT OR Apache-2.0` | Ativo | Backend grafico Rust multiplataforma, WebGPU, Vulkan, Metal, DX12, GLES e WASM. | Melhor candidato para dependencia grafica base da Omoikane. |
| [bevyengine/bevy](https://github.com/bevyengine/bevy) | `MIT OR Apache-2.0`, com avisos por crate/asset | Ativo | ECS, app schedule, render graph, asset model, exemplos e padroes de modularidade. | Estudar arquitetura; evitar importar a engine inteira para nao perder identidade e controle. |
| [linebender/vello](https://github.com/linebender/vello) | `MIT OR Apache-2.0`, shaders tambem com opcao Unlicense | Ativo/alpha | Renderer 2D compute-centric, path rendering, cenas vetoriais e integracao `wgpu`. | Candidato para UI/debug draw vetorial ou referencia para renderer 2D proprio. |
| [Rust-GPU/rust-gpu](https://github.com/Rust-GPU/rust-gpu) | `MIT OR Apache-2.0` | Experimental | Shaders em Rust para SPIR-V, `spirv-builder`, possibilidade de compartilhar tipos CPU/GPU. | Manter como trilha experimental; comecar com WGSL e adicionar crate de shaders Rust quando estabilizar. |
| [schell/renderling](https://github.com/schell/renderling) | `MIT OR Apache-2.0` | Alpha | Renderer GPU-driven, cena em buffers GPU, glTF, Forward+, PBR, rust-gpu e `wgpu`. | Forte referencia para `hikari`: cena GPU-resident, slab allocator, forward+ e testes headless. |
| [aclysma/rafx](https://github.com/aclysma/rafx) | `MIT OR Apache-2.0`; assets com licencas separadas | Pre-0.1/legado util | Render graph, asset pipeline, shader processor, render jobs, visibilidade e plugin renderer. | Usar como blueprint arquitetural; nao copiar assets; avaliar trechos especificos antes de qualquer codigo. |
| [BVE-Reborn/rend3](https://github.com/BVE-Reborn/rend3) | `MIT OR Apache-2.0 OR Zlib` | Arquivado/manutencao | Renderer 3D em `wgpu`, render graph, glTF, PBR, rotinas reutilizaveis. | Referencia historica; evitar dependencia central por estar arquivado. |
| [dimforge/rapier](https://github.com/dimforge/rapier) | `Apache-2.0` | Ativo | Fisica 2D/3D, colisao, integracao WASM, abordagem robusta para simulacao. | Comparar com `butsuri`; usar como benchmark e possivel dependencia opcional para 3D. |
| [Are We Game Yet](https://arewegameyet.rs/) | `CC BY 4.0` para a pagina | Catalogo vivo | Lista de crates por engines, rendering, ECS, physics, shaders, networking, UI e tools. | Usar como fonte recorrente para auditoria de ecossistema e descoberta de crates. |

## Backlog tecnico derivado

### P0: base grafica Rust pura

- Criar crate `hikari` para renderer e abstracao grafica.
- Adicionar `wgpu`, `winit`, `raw-window-handle`, `bytemuck`, `pollster` e `naga` somente depois de checagem de licencas via `cargo-deny`.
- Implementar camada minima: `GraphicsInstance`, `GraphicsDevice`, `SurfaceTarget`, `FrameContext`, `FrameSubmission`, `GraphicsResourceCatalog`, `GpuBuffer`, `GpuTexture`, `ShaderModule`, descritores de buffers/texturas/shaders/pipelines/bind groups/compute pipelines, command lists e usos/formato iniciais.
- Criar render graph tipado: passes declaram `read`, `write`, `create_transient`, `create_persistent`, `preserve`, `discard`.
- Validar grafo em runtime e teste: recursos sem escritor, ciclos, leitura antes de escrita, criacao duplicada, pass morto, aliasing indevido e lifetimes diagnosticaveis.

### P1: pipeline de renderizacao escalavel

- Separar etapas `extract`, `prepare`, `queue`, `submit`.
- Usar buffers persistentes e handles geracionais para recursos de GPU.
- Adicionar renderer 2D inicial: sprites, debug lines, tiles e UI basica.
- Adicionar renderer 3D minimo: mesh, camera, material PBR simples, luz direcional e glTF subset.
- Adicionar testes headless com hash de framebuffer ou snapshot PNG opcional.

### P2: recursos experimentais

- GPU-driven culling: frustum CPU primeiro, HZB compute depois.
- Forward+ ou clustered lighting para muitas luzes.
- Virtual texturing com manifest content-addressed.
- Rust-GPU como trilha opcional para shaders escritos em Rust.
- Fuzzing de shader pipeline e asset manifests.
- Telemetria de frame budget: CPU simulation, CPU render extraction, GPU pass timings, upload bandwidth.

## Primeiros alvos concretos

1. Implementar `xtask architecture-map` para medir acoplamento real da Omoikane.
2. Criar `docs/Renderer Architecture.md` com o contrato de `hikari`.
3. Adicionar `cargo-deny` e uma politica de licencas permissivas. Concluido em 2026-05-05.
4. Criar crate `hikari` vazio com testes de render graph puro em CPU, sem abrir janela. Concluido em 2026-05-05.
5. Fortalecer `hikari` com lifetimes explicitos de recursos e tipos estruturais minimos sem backend real. Concluido em 2026-05-05.
6. Expor lifetimes de recursos em `GraphValidation` para diagnostico de graph. Concluido em 2026-05-05.
7. Adicionar descritores validaveis de buffers e texturas sem backend real. Concluido em 2026-05-05.
8. Adicionar descriptor validavel de shader sem compilador real. Concluido em 2026-05-05.
9. Adicionar descriptor validavel de render pipeline sem backend real. Concluido em 2026-05-05.
10. Adicionar descriptor validavel de bind group layout sem backend real. Concluido em 2026-05-05.
11. Adicionar descriptor validavel de bind group sem backend real. Concluido em 2026-05-05.
12. Adicionar command list validavel para `queue`/`submit` sem backend real. Concluido em 2026-05-05.
13. Adicionar compute pipeline e dispatch validaveis sem backend real. Concluido em 2026-05-05.
14. Adicionar submissao de frame validavel sem backend real. Concluido em 2026-05-05.
15. Adicionar catalogo CPU-only de recursos para validar handles submetidos. Concluido em 2026-05-05.
16. Validar usos declarados de recursos no catalogo CPU-only. Concluido em 2026-05-05.
17. Validar compatibilidade de bind group layout contra pipeline ativo. Concluido em 2026-05-05.
18. Validar que `DrawIndexed` tenha index buffer associado. Concluido em 2026-05-05.
19. Validar slots de vertex buffer exigidos pelo pipeline ativo. Concluido em 2026-05-06.
20. Depois integrar `wgpu` e um teste headless que cria device em backend disponivel ou `noop`.

## Decisoes de seguranca juridica

- Nao copiar codigo de papers, TCCs ou repositorios sem auditoria de licenca por arquivo.
- Nao trazer imagens, modelos, fontes, skyboxes ou cenas de demo sem licenca de asset separada.
- Repositorios permissivos podem virar dependencia; repositorios arquivados ou alpha entram primeiro como referencia.
- Trabalhos academicos brasileiros encontrados em repositorios institucionais devem ser tratados como referencia, nao como material copiavel.
- O dossie deve ser atualizado a cada integracao real, indicando commit, crate, licenca e motivo tecnico.
