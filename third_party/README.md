# Reviewed dependency patches

These directories retain upstream code and licences. Exact original crate checksums and per-file SHA-256 digests are recorded in PATCHES.json; Cargo.toml.orig remains upstream material. Build scripts must include these path dependencies in licence notices and corresponding application source.

- librqbit-utp 0.7.0: only Cargo.toml changes, replacing ringbuf 0.4 with patched ringbuf >=0.5.2 for RUSTSEC-2026-0293. This avoids maintaining a private memory-safety backport in ringbuf itself. Compatibility and uTP regression tests must pass.
- glib 0.18.5: backport the upstream RUSTSEC-2024-0429 fix to VariantStrIter::impl_get, making the output pointer mutable and passing &mut p to g_variant_get_child. GTK3/Tauri requires the 0.18 ABI, so replacing it with glib 0.20 alone is incompatible. Optimized string-iterator regression tests must pass. The audit exception for this one ID is conditional on patch-integrity and runtime tests; it is not an assertion that unmodified registry glib 0.18.5 is fixed.

Upstream advisories:
https://rustsec.org/advisories/RUSTSEC-2026-0293.html
https://rustsec.org/advisories/RUSTSEC-2024-0429.html
