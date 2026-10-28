# MAGNETRON 0.9.0 release checklist

Updated October 28, 2026. Published baseline: 0.8.1. Version 0.9.0 is a working candidate, not a published release.

## Completed baseline

- Unified search and links, torrent v1/hybrid/v2 and eD2k transfers, direct destinations and safe payload deletion.
- Source automation, optional DHT/Kad discovery, explicit folder sharing and transfer limits.
- Spanish/English, appearance controls, integrated downloads, close-to-tray and Windows signed updates.
- Crash containment, atomic private queues with checkpoint recovery, real engine restart, dependency fixes and browser accessibility corrections.
- Four-platform 30-minute recovery matrix and package checks passed at 1d47bd5. See SECURITY-REVIEW.md for exact evidence and limits.

## Verified for the 0.9.0 candidate

- Installed version visible in Settings / MAGNETRON updates.
- Individual YouTube links through the existing input and download list; default maximum quality in Settings.
- Private pinned video tools, local-only FFmpeg build, no user-installed dependencies.
- Signed native update manifests and installers for Linux AppImage/Debian and both Mac architectures, with startup acknowledgement and application recovery.

## Delivery status and publication checkpoint

- Completed: fresh video, updater, security, accessibility, native package and four-platform 30-minute recovery checks passed at 05295dd.
- Completed: actual signed package bytes were verified for all five update channels; package/install/runtime checks passed.
- Completed: versioned installers, matching source, manifests and release notes are prepared; final upload-set checksums are generated alongside the kit. See [verification](VERIFICATION-0.9.0.md).
- Obtain owner approval for the concrete new GitHub release; publish only after acceptance.
- Prepare SourceForge profile and upload set after the GitHub release.

## External validation and optional certification

- End-user graphical update tests, polkit authorisation and Mac Gatekeeper reports.
- Apple Developer ID/notarisation and Windows Authenticode require external publisher credentials. They do not replace the application's update signatures.
- Independent security/privacy and screen-reader reviews are not represented by the maintainer's automated checks.

Private indexer accounts, Windows/Linux ARM packages and hard CPU/total-disk quotas are future scope, not part of this candidate.
