# Estagio Funcional da Omoikane

Data: 2026-05-07

Este documento marca a transicao do estagio inicial para o estagio funcional.
O estagio inicial terminou quando a Omoikane deixou de ser somente fundacao de
crates e passou a operar como servidor local com observabilidade, config,
automacao de rack e validacao de endpoints.

## O Que Foi Fechado

- A raiz continua sendo workspace Cargo virtual.
- `omoikane.exe` existe como binario raiz para Windows.
- `omoikane-server` continua sendo o binario Rust-fonte dentro de
  `omoikane_web`.
- Hayate sobe a borda HTTP do servidor.
- Grakane gera painel JSON em `/grakane/dashboard.json`.
- Kaminari expoe catalogo NETCONF read-only por padrao.
- Mamori expoe plano de checks e rollback.
- Michisuji gera perfil RB2011 auditavel.
- SQLx/PostgreSQL pode ser habilitado por `--database-url`.
- Config TOML/JSON pode ser usada por `--config`.
- `xtask` valida layout, arquitetura, smoke test, bench e perfis de rack.

## Contrato do Estagio Funcional

O estagio funcional exige que qualquer mudanca mantenha:

- `daikoku` independente da borda web;
- `omoikane_control` independente de runtime HTTP e de firmware;
- credenciais mascaradas em terminal e endpoints;
- documentacao em portugues quando a mudanca alterar comportamento;
- validacoes locais antes de abrir PR;
- rotas observacionais sem mutacao acidental de simulacao.

## Checklist de Operacao

```powershell
.\omoikane.exe --name Omoikane --bind 127.0.0.1 --port 8080 --overlay-seed local
```

Em outro terminal:

```bash
cargo run -p xtask -- web-smoke --host 127.0.0.1 --port 8080
cargo run -p xtask -- web-bench --host 127.0.0.1 --port 8080 --path /status --requests 128
```

## Proximo Nivel

O proximo nivel nao e mais provar que a base existe. Agora o foco e transformar
o runtime em experiencia jogavel:

- transporte jogavel;
- cliente com janela;
- renderer 2D real;
- mapas carregados por conteudo;
- debug overlay de tick, FPS e rede;
- pacote minimo de exemplo que qualquer pessoa consiga rodar.
