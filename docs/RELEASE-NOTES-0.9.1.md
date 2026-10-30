# MAGNETRON 0.9.1

## New

- Drop one or more local `.torrent` files onto the Explore search bar to add them directly to Downloads. The bar highlights while a file is over it. No extra import dialog or button is added.
- Downloads use the configured destination. If none is set, the first import asks for a folder and remembers it; cancelling starts no transfers.
- BitTorrent v1, hybrid and pure v2 metadata use the existing download managers. Private copies of imported metadata allow restart/resume even when the original `.torrent` is moved or deleted. Existing queue entries are reused by torrent identity.

## Validation and limits

Local imports are limited to 100 files per drop and 4 MiB of metadata per file. Non-torrent files are ignored in mixed drops; malformed torrents are reported individually and do not prevent the other files being queued. Existing payloads keep the same collision and path protections.

Dragging a torrent requests its download; it does not guarantee reachable peers or eventual completion. Linux/macOS remain experimental. This change does not associate `.torrent` files with double-click opening.
