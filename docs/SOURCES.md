# Sources and discovery

## Direct adapters

| Adapter | Catalogue | Documentation |
|---|---|---|
| Ubuntu | releases.ubuntu.com, bounded recent numeric releases | https://ubuntu.com/download/alternative-downloads |
| Debian | cdimage.debian.org installation CD/DVD and Live amd64 torrents | https://www.debian.org/CD/torrent-cd/ |
| Internet Archive | Public advanced search and item metadata, up to 20 candidates | https://archive.org/developers/md-read.html |

These adapters retrieve metadata, not payload content. Restricted items and items without a published torrent are skipped. Domain restrictions, timeouts and response bounds apply. Public entries do not establish content rights or future availability.

## Jackett

Pinned base: https://github.com/Jackett/Jackett/releases/tag/v0.24.2900 . The observed catalogue exposed 94 public connectors, not 94 guaranteed working sources. Automatic mode includes public adapters, prioritises responsive ones and backs off failures. Manual selection remains in Settings. Sources needing credentials/CAPTCHA configuration are not bypassed; no CAPTCHA-solving service is bundled.

Official stable updates are bounded, checked against published SHA-256 and smoke-tested before a subsequent engine start, retaining the bundled fallback and matching upstream source. This updates upstream adapters, not arbitrary websites discovered across the Internet.

## BitTorrent discovery and availability

DHT uses BEP 5/51 with BEP 9/10 metadata retrieval. It is partial discovery, not keyword search. Passive indexing follows v1/hybrid identities; pure v2 from Torznab or pasted links uses the libtorrent worker. Protocols: https://www.bittorrent.org/beps/bep_0005.html and corresponding BEP 9, 10, 51 and 52 documents.

Availability checks run independently of optional discovery. V1 requires matching metadata, a complete bitfield and unchoked peer. V2 requires verified metadata and a connected complete source, without the same unchoke test. Search checks do not request payload. Public IPv4 filtering/time bounds can omit valid torrents.

## eD2k and Kad

Server search uses a public list from https://github.com/Telemacore/ed2k-server-list and fixed fallbacks, with bounded concurrency/pages, compression and 64-bit size support. Kad uses aMule with bounded nodes.dat bootstrap and remains experimental. It is not BitTorrent DHT.

Recent reported complete sources are required; reports do not verify connectivity, queue position or future completion. NAT/firewalls and aMule's Windows global instance lock can affect operation.
