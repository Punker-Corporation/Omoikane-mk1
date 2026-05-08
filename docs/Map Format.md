# Formato de Mapa da Omoikane

Os mapas da Omoikane sao snapshots deterministicos de topologia, tiles e estado
de entidades. O runtime Rust modela essa camada por `sekai::MapManager`,
`sekai::MapGrid`, `sekai::MapChunk`, `sekai::Tile`, componentes de transform e
payloads serializados.

A representacao atual prioriza correcao de simulacao. Mapas possuem grids,
grids possuem chunks esparsos, chunks possuem arrays compactos de tiles, e
entidades referenciam espaco de mapa ou grid por `MapId`, `GridId` e
`EntityUid`.

## Secoes

### `meta`

Guarda dados de formato e proveniencia.

- `format`: versao inteira do formato.
- `name`: nome humano opcional do mapa.
- `author`: autor ou gerador opcional.
- `postmapinit`: indica se a geracao e a inicializacao ja foram executadas.

### `tilemap`

Mapeia nomes estaveis de definicoes de tile para ids numericos compactos usados
nos arrays internos de chunks. Codigo de runtime nao deve assumir que ids
numericos permanecem estaveis entre pacotes de conteudo.

### `grids`

Guarda um ou mais registros de grid. Cada registro contem:

- `settings`: tamanho de tile, tamanho de chunk e snap size;
- `chunks`: registros esparsos indexados por coordenadas de chunk;
- estado da entidade de grid quando ela participa do ECS.

### `entities`

Guarda entidades serializadas e payloads de componentes. Referencias de entidade
usam `EntityUid`, e referencias de grid usam `GridId`. Referencias ausentes ou
externas devem ser representadas explicitamente, nunca remapeadas de forma
silenciosa.

## Dados Binarios de Tile

Dados de tile em chunk sao empacotados em ordem row-major. Cada tile carrega um
id compacto e flags de renderizacao/metadados. Codigo de runtime deve preferir
a API tipada `Tile` em vez de interpretar bytes crus.

## Direcao

O formato antigo baseado em YAML ainda pode servir como intercambio, mas o
caminho interno da engine deve preferir codificacao binaria ou hibrida quando a
fronteira de serializacao estiver completa. O alvo e carregar mapas de forma
deterministica, fazer roundtrip sem perda, enderecar chunks por conteudo e
replicar incrementos com baixo custo.
