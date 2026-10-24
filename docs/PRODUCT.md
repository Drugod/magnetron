# Product scope — MAGNETRON

A simple Windows desktop application to find, download and share files over BitTorrent and eD2k in one interface.

## Interaction

One field accepts queries or magnet/eD2k links. Search replaces the current query; Clear resets its display without deleting the index. Content type and order are primary filters. Downloads are integrated, not transfer pop-ups. Sources and advanced settings stay in Settings. Language and appearance switch without restarting. Close continues work in the tray; Quit shuts down.

## Implemented scope

Public-source search, deduplication/local metadata storage, optional DHT, experimental Kad, v1/hybrid/v2/eD2k transfers, persistent queues, destinations, speed controls, safe confirmed deletion, opt-in folder sharing, favourites and export are present in 0.8.0.

The chosen folder is the destination itself; natural multi-file torrent structure is preserved. Existing files must not be silently overwritten. Download-list removal and payload deletion are distinct actions.

## Evidence, resources and privacy

Catalogue counts are not proof of a working transfer. Direct BitTorrent checks and eD2k/Kad reports carry different evidence. No check guarantees future availability, harmless content, content rights or universal coverage.

Keep data local, history optional and accounts/telemetry unnecessary. Sources and peers can observe protocol traffic. Respect access controls. Explicit sharing switches do not disable active-download piece exchange.

Timing/concurrency and metadata budgets bound work, without strict percentage CPU/total SQLite disk caps. Sharing uses validated paths; torrent folder storage is read-only.

## Distribution

Windows x64 packages include engines/runtimes, notices and corresponding engine source. Source commits exclude generated distributions, private data and keys. Successful local builds do not establish public signing/update configuration, clean-machine validation, independent review or other-platform compatibility. README.md, BACKLOG.md and VERIFICATION.md describe current status.
