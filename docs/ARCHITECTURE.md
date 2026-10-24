# Architecture — 0.8.0

The local TypeScript/Vite interface invokes Tauri 2 commands in Rust. Events carry search identifiers so obsolete responses cannot replace the current query. The WebView uses local assets and a restrictive CSP; untrusted titles and paths are rendered as text. Native commands validate filesystem paths and links. Windows x64 is the supported target, retaining the stable local.magnetforge.desktop data identifier.

## Search and storage

- sources.rs defines the source interface and Ubuntu, Debian and Internet Archive adapters.
- engine.rs manages authenticated loopback Jackett, its public catalogue, Torznab parsing and connector health/backoff.
- engine_updates.rs stages bounded official releases, checks published SHA-256, confines extraction and smoke-tests startup/catalogue. It retains an engine fallback. Trust comes from the official HTTPS release, not an independent signature.
- availability.rs separates direct BitTorrent evidence from eD2k reports. Cached candidates are checked before normal display.
- dht.rs implements optional mainline DHT discovery, BEP 51 sampling and BEP 9/10 metadata retrieval, with bounded queues/concurrency and no payload requests.
- torrent.rs handles bounded bencode and original-byte v1 info hashing for direct catalogue torrents.
- storage.rs stores SQLite/FTS5 metadata, settings, history and a separate favourites table. New settings have compatible defaults.

Default budgets: 50,000 records and 256 MiB of metadata text. FTS, SQLite pages/WAL and temporary data add overhead. Discovery uses one recovery at a time with timing and local-hour controls, not hard CPU-percentage or total-disk quotas.

## Transfers

Downloads use librqbit 9.0.1 for v1/hybrid and isolated Python/libtorrent workers for pure v2. v2.rs uses private JSON stdio IPC, Windows Jobs and persisted state. Its worker validates paths before publication from private staging, protects existing files, rechecks resumed data and exits before completion. At most eight v2 workers transfer simultaneously. V2 search uses public IPv4 DHT without indexer-supplied tracker/peer endpoints; selected download links may retain those endpoints.

ed2k.rs searches public servers; kad.rs bootstraps contacts and converts experimental Kad results. ec.rs implements bounded authenticated loopback EC. ed2k_downloads.rs manages aMuled, private partial storage and verified, non-overwriting finalisation into the user destination.

Transfer limits are shared among active engines and control content traffic; protocol overhead remains. Cancellation retains partials. Completed transfers release files. Quit stops engines and persists paused transfers; window close only hides the UI.

## Sharing, export and lifecycle

sharing.rs creates folder torrents through read-only storage and sends selected eD2k directories through EC. Paths must not overlap or contain symlinks/reparse points. Folder changes require refresh. Download-piece exchange is independent of explicit sharing.

exports.rs bounds links/JSON/CSV output and protects CSV formulas/quotes. Favourites survive cache clearing and old evidence is not labelled fresh.

Windows Job Objects bind child engines to the application. Management APIs are authenticated loopback; P2P endpoints remain network-facing. No firewall rules or source access controls are changed.

app_updates.rs verifies Ed25519 manifests, HTTPS installers, size/hash and versions. The endpoint/key are fixed at build time; the current public channel is unconfigured. Installation stops engines first. Automatic executable rollback is not implemented.
