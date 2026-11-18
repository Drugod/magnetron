# MAGNETRON 0.9.3

## New

- An animated progress ring appears in the search button and beside the search status while sources are being searched and checked. It stops when the search ends, is cleared, fails or is cancelled. Starting a replacement query keeps the progress indication active. The button remains usable to start another search.
- New installations use `Downloads/magnetron` by default for all download networks. The application resolves the operating system's actual Downloads folder, including redirected or localized locations, and creates the subfolder automatically. The destination remains configurable in Settings.

## Existing installations and accessibility

Existing download-folder preferences are preserved, including an explicitly cleared choice. If the operating system's Downloads folder cannot be resolved or created, the existing first-download folder chooser remains available. No files are moved and sharing preferences are not enabled.

Search status is exposed to assistive technology. With reduced motion enabled, the progress ring remains visible without rotation. Light/dark appearance and both languages remain supported.

## Distribution

Windows x64 is the primary package. Linux x64 (Debian/AppImage) and macOS Intel/Apple Silicon (macOS 15+) remain experimental. All five signed update channels remain supported. The upload set includes source, verification evidence, SHA-256 checksums and a point-in-time Windows Defender report. Update signatures are separate from publisher Authenticode/Apple Developer ID certification, which is not configured.

## Security review

The full npm audit reports an unpatched build-only picomatch denial-of-service advisory. A narrowly scoped, version-checked exception expires on 2026-12-02; it does not apply to production dependencies or other advisories. See [the documented assessment](SECURITY-NPM-2026-11.md). Rust and bundled Python checks remain enforced.
