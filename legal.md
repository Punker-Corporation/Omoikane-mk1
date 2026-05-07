# Informacoes Legais

## Copyright

Os autores preservam o copyright de seus respectivos trabalhos submetidos a este
repositorio.

## Licenca de Codigo

Os crates Rust da Omoikane sao licenciados sob MIT, salvo indicacao explicita
em contrario. Consulte `LICENSE-MIT.TXT`.

## Licenca de Assets

Imagens, modelos e arquivos de rigging neste repositorio usam Creative Commons
Attribution-ShareAlike 3.0 United States, salvo indicacao especifica. Consulte
`LICENSE-ASSETS.TXT`.

Avisos historicos foram preservados para materiais herdados e para o historico
do repositorio. Consulte `LICENSE-GPLv3.TXT` quando aplicavel a materiais
antigos mantidos por continuidade juridica.

## Politica de Dependencias

Dependencias Rust sao auditadas por `cargo-deny` usando `deny.toml`.
Dependencias diretas devem preferir licencas permissivas como MIT,
Apache-2.0, BSD, Zlib ou CC0. Qualquer nova dependencia de backend grafico,
plataforma, banco, rede ou asset pipeline deve atualizar esta pagina.

A borda HTTP Hayate usa uma dependencia Rust versionada para servir rotas. A
licenca dessa dependencia e `MIT OR Apache-2.0`, compativel com a politica
atual.

A camada SQL usa SQLx como dependencia Rust. SQLx e licenciado como
`MIT OR Apache-2.0`. Esta linha habilita PostgreSQL no launcher Omoikane e nao
habilita SQLite, MySQL ou MariaDB neste corte.

## Rede e Rack

Michisuji gera texto de configuracao para equipamentos RB2011, mas nao
vendoriza firmware, pacotes proprietarios ou imagens de sistema de rede. O
operador deve fornecer software, credenciais, chaves e endpoints legitimos.

Kaminari gera catalogo e RPC NETCONF em XML, com perfil read-only por padrao.
Mamori descreve plano de aceitacao, checks e rollback como dados Rust
estruturados. Nenhuma dessas camadas copia scripts externos, clientes
proprietarios, ativadores, instaladores quebrados ou downloads por short-link.

## Grakane

Grakane e o nome do painel Omoikane para interpretar metricas e produzir um
dashboard JSON. O repositorio nao vendoriza, embute ou redistribui codigo de
ferramentas externas de observabilidade. A Omoikane emite metricas e JSON
proprios para que o operador conecte a ferramenta que escolher.

## Garantia

O SOFTWARE E FORNECIDO "COMO ESTA", SEM GARANTIA DE QUALQUER TIPO, EXPRESSA OU
IMPLICITA, INCLUINDO, MAS NAO SE LIMITANDO A, GARANTIAS DE COMERCIALIZACAO,
ADEQUACAO A UM FIM ESPECIFICO E NAO VIOLACAO. EM NENHUM CASO OS AUTORES OU
DETENTORES DE COPYRIGHT SERAO RESPONSAVEIS POR QUALQUER RECLAMACAO, DANO OU
OUTRA RESPONSABILIDADE DECORRENTE DO USO DO SOFTWARE.
