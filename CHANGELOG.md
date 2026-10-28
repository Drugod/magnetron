# Changelog

## 0.9.0 (candidate, not yet released)

- Add individual YouTube video downloads in the existing input and integrated list, with maximum quality in Settings.
- Bundle pinned video tools, licences and FFmpeg source; preserve existing destination files and remove completed private duplicates.
- Add signed native update channels, startup acknowledgement and application recovery for Linux/Mac.
- Show installed version in Settings / MAGNETRON updates.
- Recover private queues from valid checkpoints and preserve damaged state for inspection.
- Contain Unix engines after parent crashes and restart failed owned engines with fresh credentials.
- Fix dependency advisories and light/dark contrast; add four-platform prolonged recovery and package gates.

## 0.8.1

- Enable the signed Windows update channel and validate installer upgrades.
- Remove favourites and move language/appearance controls to the top right.
- Publish experimental Linux/macOS packages.


## 0.8.0

- Add pure BitTorrent v2 metadata/source handling and integrated downloads through bundled libtorrent.
- Add explicit per-network folder sharing with read-only BitTorrent storage.
- Add favourites and links/CSV/JSON export.
- Improve ranking, index budgets, discovery timing and local-hour scheduling.
- Improve keyboard actions, focus visibility and reduced motion.
- Prepare signed application updates; public channel remains disabled.
- Exercise public Kad bootstrap/search; Kad remains experimental.

## 0.7.0

- Rename MAGNET to MAGNETRON while retaining the data identifier.
- Unify query/link input and move Sources into Settings.
- Add Spanish/English, system/light/dark appearance and close-to-tray downloading.
- Include automatic public sources and official Jackett engine update staging/fallback.

Earlier versions introduced v1/hybrid/eD2k transfers, destination/speed controls, safe file/list management, local indexing and optional DHT metadata discovery.
