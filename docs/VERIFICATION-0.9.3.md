# MAGNETRON 0.9.3 verification

Application/source commit: `a07708bad585bfadd7b75d10511917d2c3525d7a`.

All listed workflows completed successfully on this exact commit. The source tree is checked again before tagging.

| Gate | Evidence | Result |
| --- | --- | --- |
| windows | [44274514444, attempt 2](https://github.com/Drugod/magnetron/actions/runs/44274514444/attempts/2) | Passed |
| native | [44274521676, attempt 1](https://github.com/Drugod/magnetron/actions/runs/44274521676/attempts/1) | Passed |
| security | [44274521541, attempt 1](https://github.com/Drugod/magnetron/actions/runs/44274521541/attempts/1) | Passed |
| accessibility | [44274521610, attempt 1](https://github.com/Drugod/magnetron/actions/runs/44274521610/attempts/1) | Passed |
| native_updater | [44274521608, attempt 1](https://github.com/Drugod/magnetron/actions/runs/44274521608/attempts/1) | Passed |

## Search feedback and default destinations

Twenty-three frontend tests cover retained search results and animated progress lifecycle: batches, terminal outcomes, replacement queries, stale events and command failure. An Edge browser check with a mocked native backend confirms actual rotation, reduced-motion handling, completion and zero axe violations in Spanish/English and light/dark themes; screenshots were inspected. These browser checks do not claim a live public-network search. The local Windows v2 suite also passed complete-source admission/rejection, payload checks, collisions, corrupt-file resume, retained torrent metadata and file release.

Three new Rust tests run on all four platforms: default folder creation and persistence (including redirected Unicode paths), existing/custom/cleared/legacy preference preservation and unavailable-folder fallback. Startup resolves the operating system Downloads directory and seeds only a fresh preferences row. No existing payload is moved.

## Distribution and integrity

Windows and three native package jobs run bundled engine/runtime checks, video and v2 transfer tests, Rust checks, installed/relocated package validation and real signed update-asset verification. Default opt-in public-network tests are not represented as executed. No new 30-minute recovery matrix is claimed for this release. The first Windows CI attempt exhausted the 45-second loopback corrupt-resume wait; the same code was rerun without relaxing assertions. The latest attempt recorded in BUILD-INFO.json passed every required step; the full local Windows v2 suite also passed. This timing-dependent test result is not a guarantee for public peers.

All five collected update envelopes are verified offline against the trusted public key, official versioned URLs and exact package sizes/hashes. Matching source and both Mac DMGs are included. SHA256SUMS.txt covers the final upload set.

## Dependency advisory assessment

Bundled SQLite (libsqlite3-sys 0.39.1 / SQLite 3.54.1 through rusqlite 0.41.1) and rustix 1.1.6 resolve RUSTSEC-2026-0459 and RUSTSEC-2026-0486. async-executor 1.14.1 replaces its yanked predecessor. The complete npm audit still reports GHSA-fc5q-39xc-vjpf in picomatch 4.0.7 and its dev-only build/test dependants. No official patched version is available on the review date. A documented, exact-version/advisory exception expires on 2026-12-02; this is risk acceptance, not a patched dependency or a clean raw audit. It does not cover production use or other moderate/high/critical advisories. Its fail-closed tests pass. See docs/SECURITY-NPM-2026-11.md in the matching source archive. Rust and bundled Python audits remain enforced.

## Antivirus scope

ANTIVIRUS-REPORT.json and ANTIVIRUS-SCAN.txt record point-in-time Microsoft Defender custom scans of the exact Windows installer and its application executable in an ordinary unencrypted ZIP. No analysed application is launched locally, quarantined file restored, exclusion added or global protection setting changed. These scans and signature/provenance checks do not establish universal absence of malware or predict future heuristic/runtime detections. The earlier Microsoft response concerned a separate 0.9.1 sample, not this build.

Linux/macOS remain experimental and require further end-user graphical validation. Windows Authenticode/Apple Developer ID certification is not configured; application-update signatures are separate.
