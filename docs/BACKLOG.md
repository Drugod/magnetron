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
- Links/CSV/JSON export, ranking improvements and keyboard download actions. Favourites navigation and actions were removed for simplicity in 0.8.1.
- Spanish/English and light/dark/system appearance controls at the top right, close-to-tray and Quit.
- Windows portable/NSIS packaging, pinned preparation scripts and license/source notices.
- Windows 0.8.1 signed update channel implementation, periodic checks and integrated update notice; release/upgrade validation pending.

## Release work remaining

- Publish reviewed source, a versioned installer, checksums and matching source on GitHub.
- Validate and publish the 0.8.1 bootstrap release; existing 0.8.0 clients require one manual installation. Signing key is backed up locally and stored as an encrypted Actions secret.
- Obtain/configure Authenticode signing if recognised publisher identity is required.
- Validate installation, upgrade and uninstall on a clean Windows system, including disk/network failures.
- Run prolonged Kad/transfer tests and broader relevance/coverage measurements.
- Obtain independent security/privacy and screen-reader accessibility reviews.

## Experimental native packages

Linux x64 (DEB/AppImage), macOS Apple Silicon and macOS Intel (DMG/app archive) passed native compilation, transfer tests and packaged-runtime checks. See PLATFORMS.md for evidence and limitations. End-user installation and desktop feedback remain pending; publishing these packages as experimental does not require claiming desktop validation.

## Next release priorities

- Make experimental packages and matching source available through a reviewed GitHub Release.
- Collect reproducible community reports using the issue forms and TESTING.md.
- Triage reports by platform and fix installation, transfer and file-management failures first.
- Complete Apple signing/notarization when developer credentials become available.
- Validate Windows installer upgrade and signed-channel behaviour before publishing 0.8.1.

## Future scope

Private-source accounts/external Torznab configuration, hard CPU/total-disk limits, Unix abnormal-exit engine containment and automatic executable rollback need additional development and validation. Linux ARM and Windows ARM are outside the current build matrix.

Separate history navigation, the source-status wall, decorative sidebar text, Cancel Search, More Filters, separate DHT navigation and a separate pasted-link form were deliberately removed for simplicity.
