# Status do Servidor Omoikane

O crate autoritativo de servidor e `daikoku`. O estado exposto para rede nasce
de sessoes explicitas, filas de mensagens, deltas de estado por jogador,
filtros de visibilidade e ticks reconhecidos.

## Estado de Runtime

O estado de servidor fica dividido em sistemas pequenos:

- `DaikokuServer`: orquestracao, ciclo de tick e bombeamento de mensagens.
- `ServerNetManager`: filas de entrada e saida do protocolo.
- `PlayerManager`: conexoes e sessoes.
- `ServerGameStateManager`: snapshots completos e incrementais.
- `PvsSystem`: conjuntos de visibilidade por jogador.
- `PhysicsSystem`, `MapSystem` e `TransformSystem`: simulacao autoritativa.

## Contrato de Status

Um endpoint de status em producao e uma projecao fina do estado autoritativo,
nao uma segunda fonte de verdade. `DaikokuServer::status_snapshot` le o estado
do runtime e `HttpStatusService` transforma esse snapshot em HTTP/1 pequeno sem
levar runtime async ou framework web para dentro de `daikoku`.

`omoikane_web` monta o mesmo contrato na borda Hayate. Isso preserva a
independencia do servidor autoritativo e, ao mesmo tempo, entrega uma superficie
HTTP operacional para testes de rack, proxies reversos, observabilidade e
ferramentas futuras.

Rotas de status:

- `GET /status` e `HEAD /status`: JSON completo.
- `GET /status.json` e `HEAD /status.json`: mesmo payload, com sufixo
  explicito.
- `GET /health` e `GET /healthz`: JSON minimo de saude.
- `GET /metrics`: metricas de runtime.
- `GET /database/status`: estado do pool SQL quando configurado.

Forma estavel inicial:

```json
{
  "name": "Omoikane",
  "state": "running",
  "players": 0,
  "max_players": 32,
  "tick": 0,
  "tick_rate": 60,
  "queues": {
    "sessions": 0,
    "outbound_messages": 0,
    "queued_inputs": 0,
    "queued_entities": 0,
    "queued_player_list_requests": 0
  }
}
```

O transporte continua deliberadamente fino. Ele pode ser exposto por TCP, por
metadados QUIC, por API in-process ou por Hayate, desde que leia o mesmo estado
autoritativo e nao altere dados de simulacao por uma rota observacional.
