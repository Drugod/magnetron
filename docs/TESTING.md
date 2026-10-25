# Testing and feedback

Linux and macOS builds are experimental. Native CI checks pass, but installation and desktop behaviour have not been independently validated. macOS requires version 15 or newer; Apple signing/notarization is not configured. Linux x64 packages are built on Ubuntu 22.04; desktop libraries and tray support vary.

## A short first test

1. Install or extract the package for your OS and architecture, then launch it.
2. Choose a download directory. Search for a freely distributable test file or paste an authorised magnet/eD2k link.
3. Start a download, pause it, quit MAGNETRON, reopen and resume it. Check the final file and destination.
4. Open the destination from Downloads. Remove an entry while keeping its file. If testing deletion, use a disposable test download and check that unrelated files remain.
5. Close the window and check background operation, then use Quit and check that engines stop. Tray behaviour depends on the desktop.
6. Check the language/theme controls and opening browser protocol links if configured.

Source availability changes over time: a stalled public download alone does not establish a client defect. Include whether another client can retrieve the same authorised test file and when you tested, if possible.

## Report a problem

Use the [bug report form](https://github.com/Drugod/magnetron/issues/new?template=bug_report.yml). Include the exact MAGNETRON version or commit, OS version, architecture, package type, reproduction steps, expected behaviour and actual behaviour. A short redacted error excerpt is more useful than an entire profile.

Do not upload downloaded content, full profiles, private paths, filenames, links, addresses, credentials or signing keys. Review screenshots before posting. Follow SECURITY.md for vulnerabilities rather than opening a public bug report.
