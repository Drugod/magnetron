# MAGNETRON 0.8.1 — signed Windows update channel

Windows checks for application updates shortly after startup and every six hours. New installers download in the background, are verified against an embedded Ed25519 public key and exact SHA-256/size, and appear as an integrated Update available button. Installation requires a click and stops transfers and engines before starting the installer. Transfers return paused on restart.

## One-time migration from 0.8.0

The published 0.8.0 binary has no update channel configured and cannot enable it remotely. Install 0.8.1 manually once using MAGNETRON_0.8.1_x64-setup.exe. Keep the same installation and data identifier to preserve settings and queues. Future signed Windows releases use this channel.

Linux/macOS application updates remain manual. Those platforms stay experimental. Update-manifest signing is separate from Authenticode: the Windows installer still has no recognised publisher certificate.

The application does not install updates silently. Download/source availability and completion are not guaranteed. Report reproducible problems through the repository issue forms, after redacting personal information.

Language and appearance controls now sit at the top right of the main workspace. Favourites navigation and result-star actions have been removed.
