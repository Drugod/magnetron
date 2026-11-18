# MAGNETRON 0.9.4

- Updates are now a separate section in Settings, outside Shared folders. The installed version remains visible.
- Startup explicitly checks the signed update channel after registering listeners. A newer verified manifest immediately opens an Update now / Later dialog, before background package preparation finishes. Update now waits for the verified package; Later and Escape postpone installation. Each version is announced only once per session. Installation always requires explicit consent; signature/hash and recovery checks remain enforced.
- Search progress uses one large, continuous ring directly below the central searching message. The search button keeps its arrow, and the small status ring is removed. Results arriving during a search do not remove or recreate the central indicator. Reduced-motion settings slow this essential fixed-location feedback rather than freezing it.

Both languages and themes are supported. Windows is the primary package; Linux/macOS remain experimental. Publisher Authenticode/Apple Developer ID certification is not configured; signed application updates are separate. The existing documented dev-only picomatch advisory exception expires on 2026-12-02; it is not a patched dependency or a clean raw audit.
