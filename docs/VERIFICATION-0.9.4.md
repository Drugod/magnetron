# MAGNETRON 0.9.4 verification

Application/source commit: `e723b813adfb900ae9f9378aad64cfcf4320cf01`.

All listed workflows completed successfully on this exact commit. The source tree is checked again before tagging.

| Gate | Evidence | Result |
| --- | --- | --- |
| windows | [44286738589, attempt 1](https://github.com/Drugod/magnetron/actions/runs/44286738589/attempts/1) | Passed |
| native | [44286745182, attempt 1](https://github.com/Drugod/magnetron/actions/runs/44286745182/attempts/1) | Passed |
| security | [44286761847, attempt 1](https://github.com/Drugod/magnetron/actions/runs/44286761847/attempts/1) | Passed |
| accessibility | [44286761854, attempt 1](https://github.com/Drugod/magnetron/actions/runs/44286761854/attempts/1) | Passed |
| native_updater | [44286761865, attempt 1](https://github.com/Drugod/magnetron/actions/runs/44286761865/attempts/1) | Passed |

## Interface and update consent

Twenty-nine frontend tests cover retained search results, one persistent central spinner, terminal outcomes, replacement queries, stale events and backend failures. Update tests cover listener registration before startup checks, independent Settings placement, immediate signed-manifest notification, Later/Escape, deferred consent while preparing, no duplicate installs/prompts, failed checks/installs, stale version/readiness events and consent bound to the displayed version.

Real Edge checks with a mocked native backend confirm a single 48-pixel ring directly beneath the central message, actual continuous rotation under normal and reduced-motion preferences, completion, both languages/themes, the independent update section and the Update now/Later popup. Zero axe WCAG A/AA violations were reported in all four search variants and the update dialog; screenshots were inspected. This does not claim a live public-network search or installation of a mocked update.

## Signed installation and concurrency

Startup registers update listeners before checking. Availability is emitted only after verifying a signed newer official manifest and rollback constraints. Installation still requires a fully staged package with matching signature and hash. The backend checks the exact version accepted by the user and holds the staging mutex through shutdown; a simultaneous check cannot replace the consented package during engine shutdown. The real signed artifact tests on all five channels now accept matching-version consent and reject mismatched consent, in addition to existing digest-tamper/current-version rejection.

## Distribution and integrity

The Windows and three native package jobs run bundled engine/runtime checks, video and v2 transfer tests, Rust checks, installed/relocated package validation and real signed update-asset verification. Native updater recovery passes on Linux and both Macs. Opt-in public-network tests are not represented as executed. No new 30-minute recovery matrix is claimed for this focused change.

All five collected update envelopes are verified offline against the trusted public key, official versioned URLs and exact package sizes/hashes. Matching source and both Mac DMGs are included. SHA256SUMS.txt covers the final upload set.

## Dependency advisory assessment

The full npm audit retains the documented unpatched GHSA-fc5q-39xc-vjpf finding in picomatch and its exact dev-only build/test dependants. Its narrowly scoped exception expires on 2026-12-02; this is risk acceptance, not a patched dependency or a clean raw npm audit. Other moderate/high/critical advisories remain blocking. Rust and bundled Python audits remain enforced. See docs/SECURITY-NPM-2026-11.md in the matching source archive.

## Antivirus scope

ANTIVIRUS-REPORT.json and ANTIVIRUS-SCAN.txt record point-in-time Microsoft Defender custom scans of the exact Windows installer and its application executable in an ordinary unencrypted ZIP. No analysed application is launched locally, quarantined file restored, exclusion added or global protection setting changed. These scans and signature/provenance checks do not establish universal absence of malware or predict future heuristic/runtime detections. The earlier Microsoft response concerned a separate 0.9.1 sample, not this build.

Linux/macOS remain experimental and require further end-user graphical validation. Windows Authenticode/Apple Developer ID certification is not configured; application-update signatures are separate.
