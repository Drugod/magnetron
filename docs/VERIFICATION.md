# Verification — MAGNETRON 0.8.0

Recorded October 24–25, 2026. The owner reports the requested functional checks passed on their Windows machine; this is not clean-machine or independent validation.

## Recorded evidence

- TypeScript/Vite production build succeeded; 17 Vitest/DOM tests passed for classification, identity/deduplication, availability, ranking, links/downloads, destination persistence, confirmed deletion, one input field, Settings layout, language and appearance.
- 34 Rust tests passed: 33 in the default suite plus a targeted eD2k folder test. Coverage includes SQLite/FTS/settings, favourites/cache separation, CSV formulas/quotes, link/path/protocol validation, complete-source checks, local v1/eD2k transfers and file release/deletion.
- Selected-folder eD2k testing published a synthetic file, preserved content, allowed deletion and withdrew sharing.
- Native v2 testing covered metadata, pause, restart/persistence, resume, byte-identical output and payload deletion preserving an unrelated file.
- A separate loopback libtorrent test admitted a complete seed without payload requests, rejected an incomplete peer, transferred bytes, protected existing files, repaired a corrupt partial and released the completed file.
- An opt-in public Kad test bootstrapped, connected and completed a query with zero complete-source results. This does not show broad coverage or long-term stability.
- App update tests checked Ed25519 and tamper/HTTP/version rejection. A real public upgrade was not installed; the channel is unconfigured.

Seven public-service tests are normally excluded. Kad was run explicitly; the others were not all rerun for this delivery. Official Jackett update staging/fallback was exercised in 0.7.0. Test-only aMule copies isolate instance mutexes; the distributed engine remains official.

## Boundaries and reproduction

After relocation, all 17 interface tests and the TypeScript/Vite production build passed again. Documentation/ignore/remote preparation did not change application code. A clean GitHub CI run is still needed after publication. Another clean Windows installation/upgrade/uninstall, prolonged public-network tests, screen-reader tests and independent review remain pending. Availability does not guarantee future completion or content safety.

Follow README.md for preparation, npm test, npm run build, npm run test:rust, the Python v2 test and npm run bundle. For opt-in public Kad: `cargo test --manifest-path src-tauri/Cargo.toml live_kad -- --ignored --nocapture`. Public UDP/NAT restrictions can cause failures.

## Existing artifacts

- MAGNETRON-0.8.0.exe: SHA-256 `3038ace156e9c07d2d65891c4de6a6479194f52ce1798723cc3a3028de1aeb7c`.
- MAGNETRON_0.8.0_x64-setup.exe: 105,295,975 bytes; SHA-256 `5d3ba5b80c65140b5b024a92b8f87e4c0a2353a8e12c4228fcab606f55ed30bb`.

The installer includes Python/libtorrent and notices. Portable runtime/worker imports were checked. Rebuilding or signing changes hashes: regenerate checksums for published assets.
