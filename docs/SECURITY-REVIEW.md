# Security and reliability review — next version

This is a maintainer code review and automated testing effort, not an independent security audit. End-user reports and graphical update testing remain with the owner. Linux/macOS signing and SourceForge publication are deferred until the reliability work is complete.

## Trust boundaries

- Search/indexer results, peer metadata and pasted links are untrusted. They must not become shell commands, unrestricted filesystem paths or HTML.
- Download destinations and explicitly shared folders are user selections. Cleanup must remain confined to payload files and preserve unrelated files.
- Engine control endpoints must bind to loopback with fresh credentials. Profiles and stored queues contain private paths/links and need private permissions.
- Bundled executables are pinned build dependencies. Application updates require an embedded public key, a signature and exact size/hash checks; the private key stays outside source.
- Crash containment must terminate only owned process groups and descendants, never unrelated processes.

## Findings being addressed

- REL-01: Unix Job attachment was a no-op, so killing the parent could leave engines alive. A private pipe-driven guard and isolated process groups are implemented. POSIX crash/group-isolation and Rust integration tests are pending native CI.
- REL-02: v1/eD2k persistence used fixed temporary names; eD2k snapshots could race after releasing the record lock. Unique atomic temporary files, serialized snapshots and fsync are implemented. Concurrent-reader regression tests are pending Rust CI.
- PRIV-01: engine configuration and queues could inherit permissive Unix umasks. Application-owned state directories now use 0700, and atomic private files use 0600. Download destinations are not chmodded. Permission and symlink-target tests are pending native CI.
- REL-03: Jackett retained a cached connection after its process died; eD2k treated any stored child as running. Liveness checks and serialized reinitialisation are implemented; engine restart regression coverage is in progress.

## Dependency evidence

The local npm audit on October 26, 2026 reported zero known vulnerabilities. This is a point-in-time advisory check and does not establish absence of vulnerabilities. Dependabot alerts are currently disabled on the repository; no account scopes/settings were changed. A scheduled read-only security workflow adds npm and RustSec checks. Rust advisory results are pending its first CI run.

## Remaining acceptance work

Run the native process-guard and persistence tests, exercise real engine restart, run prolonged synthetic transfer/interruption/recovery scenarios, review IPC/path/network handling and keyboard/screen-reader semantics, and verify package contents and relocated runtimes. Record actual results before closing findings. Do not mark these complete based only on source preparation.

## Dependency remediation evidence

The first RustSec run reported RUSTSEC-2026-0293 (ringbuf) and RUSTSEC-2026-0367/0368/0369 (zip), plus informational maintenance/GLib unsoundness notices. ZIP is upgraded to 9.1.2. The reviewed librqbit-utp manifest patch upgrades ringbuf to 0.5.3. GTK3-compatible GLib carries the exact upstream pointer-mutability correction for RUSTSEC-2024-0429. `check-vendor-patches.py` checks every retained upstream file and reconstructs the allowed modifications; an optimized Linux iterator test additionally gates builds. The single audit exception is for that backported GLib fix, not the unmodified registry package.

Security workflow run 40415928690 passed at commit 8fd5127. Informational unmaintained-package notices remain visible and tracked: fxhash, proc-macro-error, zbus_names, zvariant and zvariant_derive. They are upstream dependency-maintenance risks, not proof of exploitation; they are not silently suppressed. Known vulnerabilities and unpatched unsoundness fail the gate. Subsequent dependency changes must rerun the same checks.

The first real-browser WCAG scan found insufficient contrast in light-theme muted text and accent buttons; the palette and accent foregrounds are corrected. The scan checks both languages, themes and main views, and honours reduced-motion so it measures settled colours. Verification of those changes remains pending the next accessibility run.
