# MAGNETRON 0.9.0 — candidate release notes

This release is being prepared and is not published yet.

## New

- Paste an individual YouTube video link into Explore and manage its download in the existing Downloads view. Choose the maximum quality in Settings. Video tools are bundled; no manual Python, Node or FFmpeg installation is required.
- See the installed version in Settings / MAGNETRON updates.
- Signed background update checks/downloads on Linux and macOS as well as Windows. Installation remains an explicit action. AppImage/Mac replacements keep the previous application until the new version confirms startup; Debian uses the system's privilege approval.

## Reliability and safety

- Recover corrupt queues from valid private checkpoints without deleting downloaded payloads.
- Stop owned Unix engines after abnormal parent exit and recover failed engines with fresh control credentials.
- Preserve unrelated files during download removal and video publication.
- Update vulnerable dependencies and backport the GTK-compatible GLib fix with source-integrity and optimized regression checks.
- Improve contrast and run browser accessibility checks in both languages/themes.

## Packages and limitations

Windows x64 is the primary target. Linux x64 (Debian/AppImage) and macOS Intel/Apple Silicon remain experimental; Mac requires macOS 15 or later. Desktop publisher certification (Authenticode/Apple Developer ID) is not configured; update manifests use a separate Ed25519 signature.

Existing Linux/Mac 0.8.1 installations need one manual installation of 0.9.0 before they can use its new updater. Windows 0.8.0 also needs a manual update once. Updates stop engines first; unfinished transfers remain recoverable and paused.

YouTube support covers individual publicly available videos, not playlists, live streams, DRM or login/cookie import. Available formats, service-side restrictions and network connectivity can prevent a download. Quality is a maximum, not a promise that a particular resolution exists. Video downloads contact YouTube/Google; there is no MAGNETRON telemetry service.

Download the complete installer/package. A Windows executable by itself does not include its engines. Preserve the engines, runtime and licences when using a portable directory.

Use the issue template to report the exact app version, OS/architecture, package type and reproduction steps. Redact personal paths and links before sharing logs.
