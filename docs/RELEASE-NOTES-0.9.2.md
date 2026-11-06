# MAGNETRON 0.9.2

## Fixed

Search results no longer disappear as their availability checks become older than five minutes. Once admitted to the current search, results remain while new batches arrive, after the search finishes, and when you leave it idle. They are reset when you clear the search or start another one; the selected content filter still controls which rows are visible.

Weaker duplicate updates preserve the last admitted title and availability evidence. Exports include the same retained results as the filtered list. Availability labels can still age: keeping a result does not guarantee a currently reachable peer or eventual download completion. New unchecked or expired cached candidates are not automatically admitted.

## Packages

- **Windows x64:** `MAGNETRON_0.9.2_x64-setup.exe` (complete installer).
- **Linux x64, experimental:** Debian package or AppImage.
- **macOS, experimental, macOS 15+:** Apple Silicon (`aarch64.dmg`) or Intel (`x64.dmg`).

The existing five signed update channels remain supported. Update signatures are separate from Windows Authenticode / Apple Developer ID publisher certification, which is not configured.

## Verification

The release upload set includes `VERIFICATION.md`, `BUILD-INFO.json` and `SHA256SUMS.txt` with the exact build, checks and limitations. Antivirus results are point-in-time results for the specified samples; they are not a universal malware-free guarantee. The historical Defender report for 0.9.1 must not be represented as a Microsoft certification of this new version.
