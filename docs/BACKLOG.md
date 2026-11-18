# MAGNETRON release checklist

Updated November 18, 2026. Published baseline: [0.9.3](https://github.com/Drugod/magnetron/releases/tag/v0.9.3). The authorised release has been published and its public download/update links verified.

## Delivered baseline

- Unified search/link input, local torrent-file drops, torrent v1/hybrid/v2, eD2k and individual YouTube transfers.
- Direct destinations, transfer limits, explicit folder sharing, safe removal, integrated downloads and close-to-tray.
- Spanish/English, appearance controls, source automation and optional DHT/Kad discovery.
- Queue recovery, owned-engine crash containment, installed version display and five signed application-update channels.
- Version 0.9.0 and 0.9.1 package, runtime, security and accessibility checks passed; prolonged recovery evidence is documented for 0.9.0. Do not infer that future builds inherit those results automatically.

## 0.9.3 delivery

- Completed: animated search progress, accessible status and reduced-motion support in both languages and themes.
- Completed: fresh installations default to the actual OS Downloads/magnetron folder. Existing, cleared and legacy destination preferences are preserved, with a chooser fallback when creation/resolution is unavailable.
- Completed: official SQLite/rustix security fixes and replacement of the yanked async-executor version. The unpatched dev-only picomatch advisory has a documented, narrowly scoped exception expiring 2026-12-02; this is risk acceptance, not a clean raw npm audit.
- Completed: signed Windows/Linux/Mac package/runtime/update checks, security/accessibility checks, 19 published assets and public-link/hash verification. The first Windows loopback resume test timed out; the same-code rerun and the full local Windows v2 suite passed. See [verification](VERIFICATION-0.9.3.md).
- Completed: point-in-time Defender scans of the exact Windows installer and application sample reported no threats with security intelligence 1.463.355.0. See the attached release antivirus report for scope and limitations.

## 0.9.2 delivery

- Implemented and locally tested: retain admitted search rows after availability evidence expires, including incoming batches, idle/completed searches, weak duplicates, filters and exports.
- Completed: fresh signed Windows/Linux/Mac packages, runtime and updater checks, security/accessibility checks and point-in-time Defender scans of the exact Windows installer and application sample. No threats were reported with security intelligence 1.463.153.0.
- Completed: 19 release assets, corresponding source, five signed update manifests, antivirus evidence and full SHA-256 inventory published and verified. See [verification](VERIFICATION-0.9.2.md).

## Follow-up work

- Reassess/remove the temporary picomatch build-tool exception by 2026-12-02; the weekly security workflow enforces its expiry.

- SourceForge profile/upload preparation follows the GitHub release.
- End-user graphical native-update, polkit, Gatekeeper and screen-reader reports remain useful external validation.
- Windows Authenticode and Apple Developer ID/notarisation require publisher credentials. They are independent of update manifest signing.
- Independent security/privacy review is not represented by the maintainer's automated tests or antivirus scans.

Private indexer accounts, Windows/Linux ARM packages and hard CPU/total-disk quotas remain future scope.
