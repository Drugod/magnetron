# MAGNETRON 0.9.0 candidate verification

Application build commit: `05295dd2b569ca4f72608ab72c3757dacc9110ae`.

All application acceptance gates passed on October 28, 2026. This is maintainer testing, not an independent audit or a promise of universal download availability. Linux/macOS remain experimental.

| Gate | Evidence | Result |
| --- | --- | --- |
| Windows build, installed runtime, upgrade and real signed installer | [40711844489](https://github.com/Drugod/magnetron/actions/runs/40711844489) | Passed; 41 Rust tests, six offline video tests before/after installation, signed-byte/tamper/rollback checks |
| Linux and both Mac packages, relocated runtime and signed native assets | [40711837923](https://github.com/Drugod/magnetron/actions/runs/40711837923) | Passed; 46 Linux / 45 Mac Rust tests, six video tests, optimized GLib regression, real signed package verification |
| Four-platform prolonged recovery | [40711771116](https://github.com/Drugod/magnetron/actions/runs/40711771116) | Passed on all four runners |
| Native updater startup and recovery | [40711770857](https://github.com/Drugod/magnetron/actions/runs/40711770857) | Passed; platform/signature binding, failed commit preservation, startup acknowledgement, rollback and Mac LaunchServices fixture |
| Dependency security | [40711770881](https://github.com/Drugod/magnetron/actions/runs/40711770881) | Passed; npm, RustSec/backport integrity and OSV video-wheel checks |
| Browser accessibility | [40711770923](https://github.com/Drugod/magnetron/actions/runs/40711770923) | Passed; both languages/themes, three main views, keyboard reachability |

## Prolonged recovery details

| Runner | Elapsed seconds | Cycles | Successful scenarios |
| --- | ---: | ---: | ---: |
| macos-15 | 1809.991 | 27 | 210 |
| macos-15-intel | 1801.293 | 26 | 202 |
| ubuntu-22.04 | 1801.171 | 41 | 324 |
| windows-latest | 1804.969 | 22 | 150 |

There were 886 successful scenarios in total. They use real bundled engines and controlled peers, including interrupted/corrupted partials, restart/resume, verified file completion/release, checkpoint recovery and owned-process shutdown. These repeated lifecycle scenarios do not constitute a memory-leak audit or a public-network availability guarantee.

## Public YouTube smoke test

The packaged Windows tools fetched public metadata, downloaded a short public video at a 360p maximum, probed the result and published one 474,561-byte file directly in the selected test directory. No cookies/account were used; the temporary test payload was deleted. Six offline video tests additionally cover path/link rejection, collision preservation, cancellation, owned-partial recovery, an interruptible limiter, actual mux/probe tools and title/quality handling.

## Artifact integrity and source

All five update envelopes were rechecked offline with the application public key. Their official versioned URLs, versions, sizes and SHA-256 hashes match the delivered Windows installer, Debian/AppImage and both Mac application archives. Both DMGs and matching source are also included. The final SHA256SUMS.txt covers the complete upload set.

The acceptance closure updates documentation only; application sources, pinned assets, package manifests and preparation scripts remain identical to the tested build commit. BUILD-INFO.json records build and source commits separately.

No Release, tag or SourceForge project has been published for 0.9.0. Apple Developer ID/notarisation, Windows Authenticode, independent reviews and end-user graphical update/polkit/Gatekeeper reports remain external certification/validation work, not completed claims.
