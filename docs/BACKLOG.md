# Roadmap — MAGNETRON 0.8.0

Updated October 25, 2026. The owner has functionally validated 0.8.0 on their Windows machine. This is not independent or clean-machine validation.

## Implemented

- Unified search/link input, content filters, sorting, search replacement and clearing.
- Integrated v1/hybrid/v2 and eD2k transfers, persistent queues, pause/resume, cancellation and verified completion/file release.
- Direct destinations, content transfer limits and confirmed payload deletion preserving unrelated files.
- Automatic public Jackett sources, health/backoff, official engine update staging and fallback.
- Local index, optional DHT metadata discovery, record/text budgets, recovery timing and local-hour scheduling.
- Experimental Kad bootstrap/search: one public test connected and completed with zero complete-source results.
- Explicit per-network arbitrary-folder sharing, read-only folder torrent storage and copyable magnets.
- Persistent favourites, links/CSV/JSON export, ranking improvements and keyboard download actions.
- Spanish/English, light/dark/system appearance, close-to-tray and Quit.
- Windows portable/NSIS packaging, pinned preparation scripts and license/source notices.
- Signed app-update infrastructure; public channel still disabled.

## Release work remaining

- Publish reviewed source, a versioned installer, checksums and matching source on GitHub.
- Configure/safeguard update signing keys and HTTPS channel; test an actual upgrade.
- Obtain/configure Authenticode signing if recognised publisher identity is required.
- Validate installation, upgrade and uninstall on a clean Windows system, including disk/network failures.
- Run prolonged Kad/transfer tests and broader relevance/coverage measurements.
- Obtain independent security/privacy and screen-reader accessibility reviews.

## Native platform work in progress

Linux x64 and macOS Intel/Apple Silicon resource paths, bundled-engine preparation and native CI are now prepared. Native compile/transfer/package validation is still pending; see PLATFORMS.md. No Linux/macOS binaries have been certified by the Windows-local checks.

## Future scope

Linux/macOS ports, private-source accounts/external Torznab configuration, hard CPU/total-disk limits and automatic executable rollback need additional development and validation. Current resources are Windows x64 only.

Separate history navigation, the source-status wall, decorative sidebar text, Cancel Search, More Filters, separate DHT navigation and a separate pasted-link form were deliberately removed for simplicity.
