# Release procedure

Repository: https://github.com/Drugod/magnetron . Preparing a working copy does not publish commits, tags or releases.

## Source and release assets

Review staged files for generated output, local profiles, downloaded payloads, credentials and keys. Commit source, lockfiles, scripts, tests, docs, icons and license texts. Use English public descriptions; keep the app bilingual. Push only with the owner's publication approval.

Publish a versioned tag and GitHub Release with the Windows installer and matching SHA-256 checksums. Binaries belong in Release assets, not source commits. Provide the corresponding app source and bundled-engine source/notices as required by their licenses. Current installers include pinned aMule/Jackett source archives under licenses/.

## Signed application updates

The local build has no public channel configured. Jackett engine updates are separate.

1. Generate Ed25519 keys: `node scripts/sign-update.mjs generate <private-key-path-outside-repository>`. Securely back up the private key outside Git and distributed artifacts. Only the public key is printed.
2. Build with MAGNETRON_UPDATE_URL pointing to a stable HTTPS manifest and MAGNETRON_UPDATE_PUBLIC_KEY containing the 32-byte public key in hex. Plan key rotation before changing it.
3. Prepare engines/licenses, test and run npm run bundle. If available, Authenticode-sign with scripts/sign-windows.ps1 and the publisher certificate; this differs from manifest signing.
4. Sign final installer bytes: `node scripts/sign-update.mjs sign <private-key> <installer> <version> <https-installer-url> <manifest-output>`.
5. Publish the installer before the signed manifest. Clients check Ed25519, version, size and SHA-256 before staging. Installation stops engines before opening the installer.

The stable data identifier preserves local state. There is no automatic executable rollback: keep an older installer and back up data before migrations. Jackett has a separate engine fallback.

## Clean Windows validation (pending)

Use a fresh Windows 10/11 VM without preinstalled Python, Docker, Jackett or aMule. Check WebView2, installation/startup, protocol links, synthetic transfers, pause/resume, restart, tray/Quit, spaces/Unicode paths, existing files, full disk and network loss. Test signed upgrades and tampered manifest/installer rejection. Uninstall without deleting user downloads.

Local compilation does not establish these checks passed. Prolonged Kad tests, independent reviews and signing credentials remain external work. Linux/macOS need native ports/validation.
