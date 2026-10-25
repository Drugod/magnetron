# MAGNETRON

A minimalist app to search, download, and share files over BitTorrent and eD2k.

**Windows x64 · experimental Linux/macOS · GPL-3.0-only**

One field for search queries and pasted links, built-in transfer engines, automatic public-source management and a local metadata index. The packaged app does not require separate Jackett, aMule, Python or Docker installations.

## Linux and macOS development

Linux x64, macOS Apple Silicon and macOS Intel packages have passed native CI compilation, automated transfer tests and packaged-runtime checks. They are experimental and have not been validated on end-user desktops. See [platform builds](docs/PLATFORMS.md) for status, target requirements and native build instructions. Windows 0.8.0 was functionally validated by the project owner; 0.8.1 also passes automated installer-upgrade checks.

## Download

Get the packages from [Releases](https://github.com/Drugod/magnetron/releases/latest). Linux and macOS packages are experimental. On Windows, choose the file ending in `_x64-setup.exe`: it includes the required engines and libraries.

The application executable alone is **not** a complete portable package. Keep `engine/`, `amule/`, `v2/` and `licenses/` beside it. Generated distributions are excluded from source commits.

The current installer is not Authenticode-signed. Windows may show an unknown-publisher warning. Publisher signing and the public application-update channel are not configured yet.

## Feedback on experimental builds

Report installation, search, transfer or background-operation problems using the [bug report form](https://github.com/Drugod/magnetron/issues/new?template=bug_report.yml). Include the application version, OS version, architecture, package format and steps to reproduce. Review screenshots and log excerpts for personal information before posting. See [testing and feedback](docs/TESTING.md).

## Application updates

Windows x64 builds starting with 0.8.1 check the signed update channel after startup and every six hours while running. A newer installer downloads in the background, is verified, and appears as **Update available**. Installation requires a click and stops transfers first. GitHub receives update/download requests, including the connection's network address.

**Already using 0.8.0? Install 0.8.1 manually once.** The distributed 0.8.0 executable has no configured channel and cannot activate it remotely. Linux/macOS application updates remain manual. Manifest signing does not provide an Authenticode publisher certificate.

## Features

- Unified search and magnet/eD2k input; paste up to 100 links, one per line.
- Integrated BitTorrent v1/hybrid/v2 and eD2k downloads with progress, rates, pause, resume and cancellation.
- A remembered destination and global content transfer limits. Files go directly to the chosen directory; multi-file torrents retain their natural structure.
- Optional sharing of selected folders, independently for BitTorrent and eD2k.
- Automatic public Jackett sources, failure backoff and official engine update staging with checksum validation and fallback.
- Local SQLite index, optional DHT metadata discovery, index budgets and discovery scheduling.
- Links/CSV/JSON export.
- Spanish/English, light/dark/system appearance and background downloads in the tray.
- Experimental Kad search, separate from BitTorrent DHT.

## Quick start

1. Choose language and appearance using the controls at the top right. Choose a download directory in **Settings**; with no saved destination, the first download asks for one.
2. In **Explore**, enter a query or paste links and press Enter. Another search replaces the current query.
3. Filter results by content type and choose **Download**.
4. In **Downloads**, right-click a row or choose **…** to open its folder, remove the entry while keeping files, or confirm deletion of its payload.
5. Closing the window hides it in the tray and keeps downloads running. **Quit** stops engines; unfinished transfers return paused on the next launch.

Clearing a search keeps the index. Clearing the cache keeps downloads. Existing files are not silently overwritten; deletion preserves the destination and unrelated files.

## Availability and coverage

Availability is evidence, not a guarantee of completion, speed, content safety or rights.

- V1 search checks matching metadata and a reachable peer advertising all pieces and accepting requests, without requesting payload pieces.
- V2 checks SHA-256 metadata and a connected complete source, without the same unchoke check as v1.
- eD2k/Kad uses recent network-reported complete sources, not a direct connectivity or queue-position test.

Peers can disconnect and downloads may still stall. Cache entries must be checked again before normal search display. Sources can fail or require configuration; access controls and CAPTCHAs are not bypassed. There is no universal-source coverage promise.

## Sharing and privacy

No MAGNETRON account, telemetry service or central backend is required. Sources, servers and peers can observe relevant queries and network addresses. Availability checks contact DHT/peers even when optional index discovery is paused.

Only folders explicitly enabled in Settings are shared. BitTorrent generates a folder magnet using read-only storage; it does not publish that link to a website. eD2k publishes the selected files. Refresh shared files after folder changes. Active downloads still exchange available pieces independently of folder-sharing switches.

Completed transfers release files; cancellation keeps partials. eD2k uses private temporary storage and verifies files before publishing to the destination, which can require additional disk space.

Local data uses `%LOCALAPPDATA%/local.magnetforge.desktop/`, preserving compatibility with earlier MAGNET versions. History is optional and initially off. Engine logs may contain filenames and links: redact them before sharing.

## Build on Windows

Requirements: Node.js 22.12+, stable Rust MSVC, Visual Studio C++ build tools, Windows SDK and WebView2. Engine/dependency preparation needs network access.

~~~powershell
npm ci
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-engine.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-ed2k.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-v2.ps1
npm run desktop
~~~

~~~powershell
npm test
npm run build
npm run test:rust
./vendor/v2-runtime/python.exe -I scripts/test-v2.py
npm run bundle
~~~

The desktop helper uses an optional local Rust toolchain under `.toolchain/`, otherwise Cargo from PATH. `npm run dev` previews the interface only. Public-network tests are opt-in and network-dependent.

## Documentation

[Architecture](docs/ARCHITECTURE.md) · [Sources](docs/SOURCES.md) · [Roadmap](docs/BACKLOG.md) · [Verification](docs/VERIFICATION.md) · [Release procedure](docs/RELEASE.md) · [Third-party notices](docs/THIRD_PARTY.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## License

MAGNETRON's own code is **GPL-3.0-only**; see [LICENSE](LICENSE). Dependencies retain their licenses. Jackett and aMule run as separate bundled processes; distributed packages include their notices and corresponding pinned source archives, plus Python/libtorrent and Rust dependency notices.

Use MAGNETRON to access and share material you have the right to use.
