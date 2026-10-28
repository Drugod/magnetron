# Security and reliability review — next version

This is a maintainer code review and automated testing effort, not an independent security audit. End-user reports and graphical update testing remain with the owner. Linux/macOS signing and SourceForge publication are deferred until the reliability work is complete.

## Trust boundaries

- Search/indexer results, peer metadata and pasted links are untrusted. They must not become shell commands, unrestricted filesystem paths or HTML.
- Download destinations and explicitly shared folders are user selections. Cleanup must remain confined to payload files and preserve unrelated files.
- Engine control endpoints must bind to loopback with fresh credentials. Profiles and stored queues contain private paths/links and need private permissions.
- Bundled executables are pinned build dependencies. Application updates require an embedded public key, a signature and exact size/hash checks; the private key stays outside source.
- Crash containment must terminate only owned process groups and descendants, never unrelated processes.

## Findings being addressed

- REL-01: Unix Job attachment was a no-op, so killing the parent could leave engines alive. A private pipe-driven guard and isolated process groups are implemented. POSIX crash/group-isolation and Rust integration tests passed native CI at 1d47bd5.
- REL-02: v1/eD2k persistence used fixed temporary names; eD2k snapshots could race after releasing the record lock. Unique atomic temporary files, serialized snapshots and fsync are implemented. Concurrent-reader regression tests passed on all four runners; Windows transient sharing failures use bounded atomic-rename retries.
- PRIV-01: engine configuration and queues could inherit permissive Unix umasks. Application-owned state directories now use 0700, and atomic private files use 0600. Download destinations are not chmodded. Permission and symlink-target tests passed native CI.
- REL-03: Jackett retained a cached connection after its process died; eD2k treated any stored child as running. Liveness checks and serialized reinitialisation are implemented; real engine crash/restart regression tests passed on all four runners.

## Dependency evidence

The local npm audit on October 26, 2026 reported zero known vulnerabilities. This is a point-in-time advisory check and does not establish absence of vulnerabilities. Dependabot alerts are currently disabled on the repository; no account scopes/settings were changed. A scheduled read-only security workflow adds npm and RustSec checks. Rust advisory checks passed with the narrowly scoped, integrity-checked GLib backport exception described below.

## Remaining acceptance work

Run the native process-guard and persistence tests, exercise real engine restart, run prolonged synthetic transfer/interruption/recovery scenarios, review IPC/path/network handling and keyboard/screen-reader semantics, and verify package contents and relocated runtimes. Record actual results before closing findings. Do not mark these complete based only on source preparation.

## Dependency remediation evidence

The first RustSec run reported RUSTSEC-2026-0293 (ringbuf) and RUSTSEC-2026-0367/0368/0369 (zip), plus informational maintenance/GLib unsoundness notices. ZIP is upgraded to 9.1.2. The reviewed librqbit-utp manifest patch upgrades ringbuf to 0.5.3. GTK3-compatible GLib carries the exact upstream pointer-mutability correction for RUSTSEC-2024-0429. `check-vendor-patches.py` checks every retained upstream file and reconstructs the allowed modifications; an optimized Linux iterator test additionally gates builds. The single audit exception is for that backported GLib fix, not the unmodified registry package.

Security workflow run 40415928690 passed at commit 8fd5127. Informational unmaintained-package notices remain visible and tracked: fxhash, proc-macro-error, zbus_names, zvariant and zvariant_derive. They are upstream dependency-maintenance risks, not proof of exploitation; they are not silently suppressed. Known vulnerabilities and unpatched unsoundness fail the gate. Subsequent dependency changes must rerun the same checks.

The first real-browser WCAG scan found insufficient contrast in light-theme muted text and accent buttons; the palette and accent foregrounds are corrected. The scan checks both languages, themes and main views, and honours reduced-motion so it measures settled colours. The browser scan passed both themes, both languages and all three main views (run 40621188349).

## Verified reliability baseline (October 28, 2026)

Commit `1d47bd5` passed Windows build/install/upgrade/package checks (40621188225), all native build/package/relocation checks (40621188201), security (40621188146) and accessibility (40621188349). Prolonged run 40621188478 passed on all four runners:

| Runner | Elapsed seconds | Cycles | Completed scenarios |
| --- | ---: | ---: | ---: |
| Windows x64 | 1804.149 | 42 | 290 |
| Linux x64 | 1810.563 | 41 | 321 |
| macOS ARM64 | 1810.076 | 28 | 217 |
| macOS Intel | 1814.666 | 28 | 217 |

These are controlled transfers with real bundled engines, crash/restart and persistence checks. They do not establish public network availability or certify an absence of memory leaks.

## New 0.9.0 boundaries requiring fresh acceptance

YouTube accepts only canonical individual HTTPS video identities. Downloaded titles are text, never HTML or commands. Credentials, browser cookies, external plugins and remote EJS updates are not imported. Private pinned Python packages, Node and a local-file-only FFmpeg are included with their notices and corresponding FFmpeg source. FFmpeg source signature was verified against `FCF986EA15E6E293A5644F10B4322F04D67658D8`; its SHA-256 remains build-pinned. Final output uses exclusive creation and preserves pre-existing files.

Native updates use the existing Ed25519 key. Both the app and detached installer verify the platform-specific official URL, version, size and digest. AppImage and Mac replacements preserve the previous installation until startup acknowledgement, restoring it on failure. Debian installation delegates privilege approval to polkit/dpkg; declining approval preserves the existing package. Desktop publisher certification (Apple Developer ID/Authenticode) is separate from update authenticity and is not configured.

The video/updater changes passed the full candidate matrix and installer checks at 05295dd; see [0.9.0 verification](VERIFICATION-0.9.0.md). No production update has been published for them. Native GUI/polkit prompts and Apple Gatekeeper behaviour still need end-user feedback; they are not claimed as automated GUI validation.

## Video dependency advisory check

On October 28, 2026 the OSV PyPI query returned no known advisories for the pinned yt-dlp 2026.10.26, yt-dlp-ejs 0.8.0 and certifi 2026.10.23 wheels. The read-only audit is included in scheduled CI and preserves its response artifact. This is a point-in-time check, not an assertion that every native dependency is vulnerability-free.

Node 24.22.1 is the upstream security release documented at https://nodejs.org/en/blog/release/v24.22.1 ; target binary hashes match its official release checksums. EJS invokes Node with the permission model enabled. The worker removes NODE_OPTIONS/NODE_PATH and permits only the YouTube extractor. FFmpeg has no network protocols enabled and only the required local demux/mux/parser paths are built.

## Public video smoke test

On October 28, 2026 the Windows packaged video tools successfully fetched metadata for a public short video (25 formats), downloaded it at the selected 360p maximum, probed its video stream and published one 474,561-byte file directly in the selected test directory. No cookies/account were used and the temporary payload was deleted after verification. The previous upstream test-video URL had become unavailable; the opt-in script accepts a replacement URL. This demonstrates one public-service transfer, not universal YouTube availability.

## Final candidate acceptance

The 0.9.0 application build at 05295dd passed all Windows/native package, signature, security, accessibility and updater-recovery gates. The final four-runner prolonged check passed 886 controlled scenarios over at least 30 minutes per runner. The controlled eD2k server now advertises only its supported framing and waits for the actual connection state, not the periodic publication timer. All delivered update assets were independently rehashed and verified with the embedded public key. See VERIFICATION-0.9.0.md for exact run links, counts and limitations. The acceptance closure changes documentation only.
