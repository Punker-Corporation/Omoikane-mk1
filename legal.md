# Legal Info

## Copyright

The Authors retain all copyright to their respective work here submitted.

## License

All images, models, and rigging files in this repository are licensed under the
Creative Commons Attribution-ShareAlike 3.0 United States license, unless
otherwise stated. See `LICENSE-ASSETS.TXT`.

## Code license

The Omoikane Rust crates are licensed under the MIT license unless otherwise
stated. See `LICENSE-MIT.TXT`.

Historical license notices are preserved for inherited assets and repository
history. See `LICENSE-GPLv3.TXT` where it applies to older inherited source
materials in history.

## Dependency policy

Rust dependencies are checked with `cargo-deny` using `deny.toml`. New direct
dependencies should prefer permissive licenses such as MIT, Apache-2.0, BSD,
Zlib or CC0, and must be reviewed before adding graphics backends, platform
crates or asset pipeline tooling.

The `omoikane_web` crate integrates Actix Web as a Rust dependency. The upstream
Actix Web workspace is licensed `MIT OR Apache-2.0`, which fits the current
dependency policy.

RouterOS support in this repository is limited to generated configuration text
and integration guidance for MikroTik devices. RouterOS itself is proprietary
software licensed and distributed by MikroTik with RouterBOARD devices; no
RouterOS binaries, packages or firmware images are vendored into this
repository.

The network-control layer is implemented as original Rust source in
`omoikane_control`. It generates rack automation manifests, NETCONF RPC text and
WireGuard-style overlay profiles, but it does not vendor Python automation
source, network operating system images, proprietary VPN clients, activators,
cracked installers, short-link downloads or third-party binaries. Operators must
provide legitimate credentials, keys, endpoints and device software.

The SQL layer integrates SQLx as a Rust dependency. SQLx is licensed
`MIT OR Apache-2.0`, matching the current dependency policy. This repository
enables PostgreSQL support for the Omoikane server launcher and does not enable
SQLite or MySQL/MariaDB in this cut.

Grafana support is limited to generating metrics text and a dashboard JSON
shape that can be imported into a Grafana deployment. Grafana itself is not
vendored, embedded or redistributed by this repository.

## Warranty

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
