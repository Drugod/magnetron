# Temporary build-tool advisory assessment

Reviewed: 2026-11-18. Exception expires **2026-12-02** (UTC); CI fails again if still reported on that date. This is a scoped risk acceptance, not a dependency fix or a clean raw npm audit.

## GHSA-fc5q-39xc-vjpf / CVE-2026-112366

The [reviewed advisory](https://github.com/advisories/GHSA-fc5q-39xc-vjpf) describes denial of service when attacker-controlled repeated extglob patterns are compiled and matched by picomatch. The npm registry's latest release remains 4.0.7 with no official patched release on the review date. The [upstream discussion](https://github.com/micromatch/picomatch/pull/235) disputes the advisory and recommends keeping glob patterns trusted; the application assessment does not depend on withdrawing the advisory.

MAGNETRON uses picomatch only through Vite/Vitest and their file-discovery helpers during development, tests and builds. The exact locked graph is picomatch 4.0.7, fdir 6.5.0, tinyglobby 0.2.17, Vite 7.3.7 and Vitest 5.0.5. All five lock entries are dev-only. Build/test glob patterns come from reviewed repository configuration and tool defaults. Search queries, imported links and remote search results are not passed to these Node tools; the distributed application contains compiled frontend assets and the Rust backend, not the Node build server or dependency tree. Vite listens only on loopback. Untrusted source/configuration must not be built with secrets.

The release accepts this limited build-tool denial-of-service risk until a patched release, revised advisory or expiry requires review. No protection against deliberately malicious repository changes is claimed. Reassess immediately if dynamic/user-controlled globs, a deployed Vite server, production dependencies or the dependency graph change.

## Enforced boundaries

`scripts/audit-npm.mjs` still runs the complete npm audit, including development dependencies. It records the unfiltered report and evaluation in `artifacts/security/npm-audit.json`. Only this exact advisory identity and exact versions/nodes marked dev-only can be accepted, including dependants whose entire advisory chain resolves solely to it. Changed versions, additional copies, production use, new advisories, malformed responses, audit execution failures and expiry fail closed for moderate/high/critical findings. Tests exercise these failure paths. Other npm vulnerabilities retain the previous moderate threshold; Rust and bundled Python audits are unchanged.

The weekly security workflow reevaluates current advisory data and retains the complete report. Maintainers must remove this exception once it is no longer needed or replace it with a documented, reviewed remedy; do not extend it automatically or label the package patched.
