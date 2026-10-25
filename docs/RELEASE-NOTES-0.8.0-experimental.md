# MAGNETRON 0.8.0 — experimental native packages

MAGNETRON brings unified search and pasted magnet/eD2k links, integrated downloads, optional folder sharing, Spanish/English and light/dark appearance to experimental Linux and macOS packages alongside Windows x64.

## Choose your package

- Windows x64: NSIS setup executable. The standalone executable requires its companion engine, amule, v2 and licenses directories.
- Linux x64: DEB for compatible Debian/Ubuntu systems, or AppImage. AppImage files may need executable permission before launch.
- macOS Apple Silicon: ARM64 DMG or app archive; macOS 15 or newer.
- macOS Intel: x64 DMG or app archive; macOS 15 or newer.

Native CI compilation, automated transfer tests and packaged-runtime checks pass for Linux and both Mac architectures. End-user desktop testing remains pending. macOS packages are not signed/notarized; the Windows installer is not Authenticode-signed. Do not disable OS security protections to test the application.

Application automatic updates are not enabled. Install future application packages manually. Engine-source availability and transfer completion cannot be guaranteed.

Please [report bugs](https://github.com/Drugod/magnetron/issues/new?template=bug_report.yml) with your version, OS, architecture, package type and reproduction steps. Redact personal information. See [testing guidance](https://github.com/Drugod/magnetron/blob/main/docs/TESTING.md).

Installers are built from commit `e36074cca1f694454be9dc19161bfbce91b38580`. The release tag adds documentation and issue forms without changing application code. Check downloads against `SHA256SUMS.txt`. Matching project source is attached; third-party notices and corresponding engine sources are included in the packages.
