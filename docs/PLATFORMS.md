# Native platform builds (experimental)

Windows 0.8.0 is the existing owner-validated distribution. Linux/macOS source ports and CI are prepared locally; they are not validated downloadable releases until the native workflow succeeds and packages have been tested on actual desktops.

| Target | Builder | Planned packages |
|---|---|---|
| Linux x64 | Ubuntu 22.04 | .deb and AppImage |
| macOS Apple Silicon | macOS 15 ARM64 | .app archive and .dmg |
| macOS Intel | macOS 15 Intel | .app archive and .dmg |

The current macOS packaging target requires macOS 15 or newer, including the minimum imposed by the pinned Intel libtorrent wheel. Linux ARM and Windows ARM are not part of this first matrix. AppImage portability is not a promise of compatibility with every distribution/desktop.

## Included engines

Native builds use official Jackett v0.24.2900, official aMule 3.1.0 Linux static/macOS universal distributions, a private CPython 3.13.15 runtime from python-build-standalone 20260901 and libtorrent 2.1.1 wheels. URLs and SHA-256 are fixed in scripts/native-assets.json. Target users do not install Python or separate P2P clients. Builder Python/Rust/Node and OS libraries are development dependencies, not user configuration screens.

scripts/prepare-native.py verifies downloads, confines archives, preserves runtime layout, includes original notices and corresponding engine source archives, and smoke-tests the runtime/daemon before packaging. The macOS aMule application directory is kept intact so its bundled dylib paths remain valid. Linux system WebKit/GTK requirements still apply; .deb packages and desktop environments need normal OS dependencies. Tray support depends on the desktop's indicator implementation.

## Build locally on the target OS

Install Tauri's documented build prerequisites: https://v2.tauri.app/start/prerequisites/ . Use Python 3.12+ for the preparation helper, Node.js and stable Rust. Then:

~~~sh
npm ci
python3 scripts/prepare-native.py
npm test
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
vendor/native/v2/bin/python3 -I scripts/test-v2.py
npm run bundle:native
python3 scripts/package-native.py
~~~

The native GitHub workflow builds and uploads experimental artifacts only; it does not publish Releases. Test real searches, protocol links, transfers, file permissions, selected shares, folder pickers, tray/Quit and clean installation before marking a target supported.

## Lifecycle, signing and updates

Normal Quit stops engines on all targets. Windows additionally uses kernel Job Objects for abnormal parent exit. Equivalent crash containment is not yet provided for all Unix child engines; Python transfer workers observe stdin closure, but abrupt crashes must be tested for remaining engines.

Native Jackett updates use target-specific tar.gz assets, bounded extraction and executable permissions; Windows retains ZIP updates. Signed application-installer updates remain Windows-only infrastructure and are disabled on Unix. Linux/macOS currently require manual application package upgrades.

Apple Developer signing/notarization and clean-machine Gatekeeper testing are not configured. Do not present unsigned CI artifacts as notarized or production-validated packages.

## Windows-side checks for this port

The local Windows compiler check passed, all 35 default Rust tests passed (seven public-network tests skipped), and four offline native-preparation/archive tests passed. These checks do not validate the Unix-only compiler branches or native installers; the experimental workflow must pass separately on all three native runners. macOS Dock reopen and system Quit are routed to window restoration and graceful engine shutdown.
