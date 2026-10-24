# Design decisions

This consolidates the development history. BACKLOG.md records current delivery status.

Tauri 2/Rust controls engines; a plain TypeScript/Vite UI stays local. Windows x64/NSIS is the first platform. MAGNET became MAGNETRON in 0.7.0 while retaining its data identifier.

Initial Ubuntu/Debian adapters were extended with Internet Archive and bundled, privately managed Jackett to improve coverage without separate user installations. Search is progressive, bounded and cancellable at the coordinator; remote servers may finish already-received requests. No universal coverage is promised.

Optional DHT discovers metadata without payload. Live admission was separated from catalogue seed counts and cached metadata. eD2k network reports remain distinct from direct BitTorrent evidence.

Integrated v1/hybrid transfers use librqbit; eD2k uses aMuled. Completion releases files, cancellation retains partials and user destinations do not receive application/hash wrapper folders. Pure v2 in 0.8.0 uses isolated libtorrent workers. Explicit folder sharing is opt-in by network; BitTorrent folder storage is read-only.

The interface has one search/link field, integrated downloads and Sources inside Settings. Spanish/English and system/light/dark are remembered. Close hides to tray; Quit stops transfers.

SQLite/FTS5 keeps metadata local. The original 5,000-record limit became configurable (50,000 records/256 MiB text default). Favourites are independent of cache. Timing/concurrency regulate discovery, without hard CPU/total-disk quotas.

Own code is GPL-3.0-only. Bundled dependencies retain licenses and required corresponding source. Generated engines, binaries, profiles, caches and private keys stay outside source commits. Jackett updates use official HTTPS/checksums with smoke tests and fallback. Application updates additionally need a signed manifest and trusted build-time channel/key; public configuration, publisher signing and independent validation remain pending.
