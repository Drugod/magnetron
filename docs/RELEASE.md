# Release procedure

Repository: https://github.com/Drugod/magnetron . Preparing a working copy does not publish commits, tags or releases.

## Source and release assets

Review staged files for generated output, local profiles, downloaded payloads, credentials and keys. Commit source, lockfiles, scripts, tests, docs, icons and license texts. Use English public descriptions; keep the app bilingual. Push only with the owner's publication approval.

Publish a versioned tag and GitHub Release with the Windows installer and matching SHA-256 checksums. Binaries belong in Release assets, not source commits. Provide the corresponding app source and bundled-engine source/notices as required by their licenses. Current installers include pinned aMule/Jackett source archives under licenses/.

## Signed application updates

Windows 0.8.1 embeds `src-tauri/update-public-key.txt` and the fixed HTTPS endpoint `https://github.com/Drugod/magnetron/releases/latest/download/windows-update.json`. Published 0.8.0 binaries cannot receive this configuration remotely: users install 0.8.1 manually once. Linux/macOS remain manual-update targets.

The private Ed25519 key is backed up outside the repository in the maintainer's local application data `MAGNETRON/release-signing/update.private.pem` and stored as the encrypted repository Actions secret `MAGNETRON_UPDATE_PRIVATE_KEY`. Never commit, print, distribute or rotate this key without a migration plan. Losing it prevents existing clients trusting future signatures. Anyone possessing it can sign updates.

1. Review the source, bump all version files consistently and create a reviewed `vMAJOR.MINOR.PATCH` tag.
2. Manually dispatch **Signed Windows update release** with that tag. This workflow never runs with secrets on pull requests. It tests, builds and signs the exact installer bytes, checks the private/public key match, hashes the artifacts and creates a draft. It refuses to overwrite an existing release. The signing secret is scoped to the signing step.
3. Review the draft's installer, `windows-update.json`, source and checksums. Attach other platform packages with matching source when available. Publish only after the required validation. The latest non-prerelease must contain `windows-update.json`; publishing a newer stable release without it interrupts update checks.
4. Clients check Ed25519, official versioned Windows URL, version, size and SHA-256 before staging, then revalidate staged bytes before installation. Background failures are retried at the next six-hour check; the user can also check in Settings. Installation is explicit and stops engines first.

For a local release: `node scripts/sign-update.mjs sign <private-key-outside-repo> <installer> <version> <official-versioned-installer-url> <manifest-output>`. Authenticode signing, if configured later, must happen before manifest generation because it changes installer bytes.

The stable data identifier preserves local state. There is no automatic executable rollback: keep an older installer and back up data before migrations. Jackett has a separate engine fallback.

## Clean Windows validation (pending)

Use a fresh Windows 10/11 VM without preinstalled Python, Docker, Jackett or aMule. Check WebView2, installation/startup, protocol links, synthetic transfers, pause/resume, restart, tray/Quit, spaces/Unicode paths, existing files, full disk and network loss. Test signed upgrades and tampered manifest/installer rejection. Uninstall without deleting user downloads.

Local compilation does not establish these checks passed. Prolonged Kad tests, independent reviews and signing credentials remain external work. Linux/macOS native CI passes, but end-user desktop validation and their application updater integration remain pending.
