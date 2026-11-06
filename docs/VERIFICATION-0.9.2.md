# MAGNETRON 0.9.2 verification

Application/source commit: `b88e6bfa014da31842c394c44b8af93e22e71700`.

All listed workflows completed successfully on this exact commit. The source tree is checked again before tagging.

| Gate | Evidence | Result |
| --- | --- | --- |
| windows | [42181812161](https://github.com/Drugod/magnetron/actions/runs/42181812161) | Passed |
| native | [42181815835](https://github.com/Drugod/magnetron/actions/runs/42181815835) | Passed |
| security | [42181812324](https://github.com/Drugod/magnetron/actions/runs/42181812324) | Passed |
| accessibility | [42181812304](https://github.com/Drugod/magnetron/actions/runs/42181812304) | Passed |
| native_updater | [42181812325](https://github.com/Drugod/magnetron/actions/runs/42181812325) | Passed |

## Search-result regression

Twenty-three frontend tests cover the admitted-list lifecycle, including six-minute and day-long simulated delays, incoming/empty batches, v1/v2/eD2k evidence, weaker duplicates, search completion, filtering/sorting/language changes, exporting retained rows, clearing, new queries and stale-event rejection. Fresh evidence is still required for initial admission; retained evidence is not a claim of continued peer availability.

## Distribution and integrity

The Windows and three native workflows run real bundled engine/runtime checks, video and v2 transfer tests, Rust checks, installed/relocated distribution validation and actual signed update-asset verification. Default opt-in public-network tests are not represented as executed. No new 30-minute recovery matrix is claimed for this UI correction.

All five collected update envelopes are verified offline against the trusted public key, official versioned URLs and exact package sizes/hashes. Matching source and both Mac DMGs are included. SHA256SUMS.txt covers the final upload set.

## Antivirus scope

ANTIVIRUS-REPORT.json and ANTIVIRUS-SCAN.txt record point-in-time Microsoft Defender custom scans of the exact Windows installer and its application executable in an ordinary unencrypted ZIP. No analysed application is launched locally, quarantined file restored, exclusion added or global protection setting changed. These scans and signature/provenance checks do not establish universal absence of malware or predict future heuristic/runtime detections. The earlier Microsoft response concerned a separate 0.9.1 sample, not this build.

Linux/macOS remain experimental and require further end-user graphical validation. Windows Authenticode/Apple Developer ID certification is not configured; application-update signatures are separate.

Release and evidence files: https://github.com/Drugod/magnetron/releases/tag/v0.9.2

The Windows installer and application sample reported no threats using Defender security intelligence 1.463.153.0 on November 6, 2026. All 19 uploaded files matched their local SHA-256 hashes; public package endpoints and all five latest-update manifests were verified after publication.
