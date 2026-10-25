# Contributing

Follow README.md and docs/ARCHITECTURE.md. Windows x64 is the owner-validated target; Linux x64 and macOS Intel/Apple Silicon packages are experimental. Use the issue forms and docs/TESTING.md for feedback. Keep changes focused and describe the problem, final behaviour and relevant validation in English.

Use synthetic or authorised test payloads. Never commit downloaded content, profiles, history, credentials, signing keys, engines or build output. Redact personal paths, filenames, links and addresses from public diagnostics.

Run npm test and npm run build for interface changes, plus relevant Rust/worker checks for native work. Public-network tests are opt-in and network-dependent. Preserve GPL-3.0-only licensing of project code and third-party notices. Do not add access-control bypass or unsupported completion guarantees.
