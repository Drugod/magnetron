# Native platform builds (experimental)

Windows 0.8.0 is the existing owner-validated distribution. Linux x64 and both macOS architectures have passed native CI compilation, automated transfers and relocated-package runtime checks. Experimental packages are available as workflow artifacts; end-user desktop validation is still pending.

| Target | Builder | Generated packages |
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

Normal Quit stops engines on all targets. Windows additionally uses kernel Job Objects for abnormal parent exit. The 0.9.0 candidate uses private pipe-driven guards and isolated process groups for abnormal parent exit on Unix; guard/crash isolation passed the native recovery matrix.

Native Jackett updates use target-specific tar.gz assets, bounded extraction and executable permissions; Windows retains ZIP updates. Published 0.8.1 Unix packages still require one manual upgrade. The 0.9.0 candidate implements platform-specific signed update channels for Debian/AppImage and both Mac architectures. AppImage/Mac require a writable installation location; Debian uses polkit/dpkg and normal OS authorisation. The updater preserves the previous application until startup acknowledgement. Native candidate acceptance is recorded separately from the older published baseline.

Apple Developer signing/notarization and clean-machine Gatekeeper testing are not configured. Do not present unsigned CI artifacts as notarized or production-validated packages.

## Windows-side checks for this port

The local Windows compiler check passed, all 35 default Rust tests passed (seven public-network tests skipped), and five offline native-preparation/archive tests passed. These checks do not validate the Unix-only compiler branches or native installers; the experimental workflow must pass separately on all three native runners. macOS Dock reopen and system Quit are routed to window restoration and graceful engine shutdown.

## Verified CI baseline

Commit `e36074cca1f694454be9dc19161bfbce91b38580` passed all three native jobs in [workflow run 40147474521](https://github.com/Drugod/magnetron/actions/runs/40147474521), including 36 Rust tests per native target (seven public-network tests skipped), the v2 transfer harness, package generation and relocated-runtime checks. The Windows workflow also succeeded. This does not certify installer interaction, desktop integration or operation on every OS version.

The disposable macOS runner uses a resolvable localhost hostname because aMuled performs synchronous hostname resolution during startup. No end-user hostname or DNS settings are changed by MAGNETRON. Linux packaging omits the unused optional .NET LTTng tracing provider.

The 0.8.1 application code at `f1c594111e59acf570a8a3c845153e9bd7749c18` also passed [native CI run 40199085041](https://github.com/Drugod/magnetron/actions/runs/40199085041). [Signed Windows run 40199080933](https://github.com/Drugod/magnetron/actions/runs/40199080933) verified installer upgrade from 0.8.0 and checked the actual signed installer through the client verifier, including same-version and one-byte-tamper rejection. These are automated checks, not independent desktop validation.
