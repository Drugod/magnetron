import { describe, it, expect } from "vitest";
import {
  mergeResults,
  isCurrentlyAvailable,
  bytes,
  rank,
  contentTypes,
  matchesContent,
  type Torrent,
} from "../src/model";
const torrent = (hash: string, sources: string[] = ["ubuntu"]): Torrent => ({
  id: hash,
  infohash_v1: hash,
  title: "ubuntu desktop.iso",
  magnet_uri: "magnet:",
  sources,
  size_bytes: null,
  published_at: null,
  seeds_reported: null,
  availability_status: "unknown",
  availability_checked_at: null,
  files: [],
  discovered_at: "",
});

describe("Filtros por contenido", () => {
  it("reconoce ISO incluso si el título lleva .torrent", () => {
    expect(
      contentTypes({ ...torrent("iso"), title: "Ubuntu.ISO.torrent" }),
    ).toContain("iso");
  });
  it("usa los archivos internos y permite paquetes mixtos", () => {
    const item = {
      ...torrent("mixed"),
      title: "Colección",
      files: ["video/Film.MKV", "book/manual.pdf", "music/song.flac"],
    };
    expect(matchesContent(item, "video")).toBe(true);
    expect(matchesContent(item, "documents")).toBe(true);
    expect(matchesContent(item, "audio")).toBe(true);
    expect(matchesContent(item, "iso")).toBe(false);
  });
  it("aprovecha categorías de la fuente cuando no hay extensiones", () => {
    expect(
      contentTypes({
        ...torrent("video"),
        title: "Película 1080p",
        content_types: ["video"],
      }),
    ).toEqual(["video"]);
  });
  it("no inventa un tipo por palabras del título ni pistas no reconocidas", () => {
    expect(
      contentTypes({
        ...torrent("unknown"),
        title: "Vídeo de vacaciones",
        content_types: ["untrusted"],
      }),
    ).toEqual(["unknown"]);
    expect(
      matchesContent({ ...torrent("all"), title: "Sin metadatos" }, "all"),
    ).toBe(true);
  });
  it("distingue comprimidos, programas y otros ficheros", () => {
    expect(contentTypes({ ...torrent("zip"), title: "Backup.7z" })).toEqual([
      "archives",
    ]);
    expect(contentTypes({ ...torrent("exe"), title: "Installer.msi" })).toEqual(
      ["software"],
    );
    expect(contentTypes({ ...torrent("3d"), title: "Model.blend" })).toEqual([
      "other",
    ]);
  });
  it("conserva los tipos y archivos conocidos al llegar otra fuente sin detalles", () => {
    const item = {
      ...torrent("same"),
      title: "Colección",
      files: ["film.mkv"],
      content_types: ["video"],
    };
    const [merged] = mergeResults(
      [item],
      [{ ...torrent("same", ["debian"]), title: "Colección" }],
    );
    expect(matchesContent(merged, "video")).toBe(true);
    expect(merged.files).toEqual(["film.mkv"]);
  });
});
describe("Resultados canónicos", () => {
  it("agrupa el mismo hash y conserva procedencias", () => {
    const old = torrent("a");
    const result = mergeResults([old], [torrent("a", ["debian"])]);
    expect(result).toHaveLength(1);
    expect(result[0].sources).toEqual(["ubuntu", "debian"]);
    expect(old.sources).toEqual(["ubuntu"]);
  });
  it("no fusiona títulos iguales con hashes distintos", () =>
    expect(mergeResults([torrent("a")], [torrent("b")])).toHaveLength(2));
  it("no inventa tamaño ni disponibilidad", () => {
    expect(bytes(null)).toBe("Tamaño desconocido");
    expect(bytes(0)).toBe("0 B");
    expect(torrent("a").availability_status).toBe("unknown");
  });
  it("prioriza coincidencias de texto", () =>
    expect(rank(torrent("a"), "ubuntu")).toBeGreaterThan(
      rank(torrent("b"), "debian"),
    ));
});

describe("Disponibilidad obligatoria", () => {
  const now = Date.parse("2026-10-24T12:00:00Z");
  it("las semillas del índice y los metadatos solos no permiten aparecer", () => {
    expect(
      isCurrentlyAvailable(
        {
          ...torrent("a"),
          seeds_reported: 99999,
          availability_status: "reported",
        },
        now,
      ),
    ).toBe(false);
    expect(isCurrentlyAvailable(torrent("a"), now)).toBe(false);
  });
  it("solo admite la comprobación directa reciente y retira resultados caducados", () => {
    const t = {
      ...torrent("a"),
      availability_status: "verified",
      availability_checked_at: new Date(now - 1000).toISOString(),
    };
    expect(isCurrentlyAvailable(t, now)).toBe(true);
    expect(isCurrentlyAvailable(t, now + 5 * 60_000)).toBe(false);
    expect(
      isCurrentlyAvailable({ ...t, availability_checked_at: null }, now),
    ).toBe(false);
    expect(
      isCurrentlyAvailable({ ...t, availability_checked_at: "invalid" }, now),
    ).toBe(false);
    expect(
      isCurrentlyAvailable(
        { ...t, availability_checked_at: new Date(now + 60_000).toISOString() },
        now,
      ),
    ).toBe(false);
  });
});

describe("Resultados eD2k", () => {
  const now = Date.parse("2026-10-24T12:00:00Z");
  it("no fusiona dos hashes eD2k ni los mezcla con BitTorrent", () => {
    const a = {
      ...torrent("ed2k:a:42"),
      network: "ed2k" as const,
      infohash_v1: "",
    };
    const b = { ...a, id: "ed2k:b:42" };
    expect(mergeResults([a], [b, torrent("a")])).toHaveLength(3);
    expect(
      mergeResults([a], [{ ...a, sources: ["ed2k:other"] }])[0].sources,
    ).toHaveLength(2);
  });
  it("exige informe reciente con fuentes completas sin confundirlo con una prueba directa", () => {
    const row = {
      ...torrent("ed2k:a:42"),
      network: "ed2k" as const,
      availability_status: "reported_complete",
      complete_sources_reported: 1,
      availability_checked_at: new Date(now - 1000).toISOString(),
    };
    expect(isCurrentlyAvailable(row, now)).toBe(true);
    expect(
      isCurrentlyAvailable({ ...row, complete_sources_reported: 0 }, now),
    ).toBe(false);
    expect(
      isCurrentlyAvailable({ ...row, complete_sources_reported: null }, now),
    ).toBe(false);
    expect(
      isCurrentlyAvailable({ ...row, availability_status: "verified" }, now),
    ).toBe(false);
    expect(isCurrentlyAvailable(row, now + 5 * 60_000)).toBe(false);
  });
});

it("ranks exact query words above substrings, even with many reported sources", () => {
  const cases = [
    {
      q: "ubuntu desktop",
      best: "Ubuntu Desktop 24.04.iso",
      bad: "ubuntux desktopx.iso",
    },
    {
      q: "manual español",
      best: "Manual Español.pdf",
      bad: "manuales espanoles.pdf",
    },
    { q: "debian", best: "Debian", bad: "Debian guide book.pdf" },
  ];
  for (const c of cases) {
    const a = { ...torrent("a"), title: c.best },
      b = { ...torrent("b", Array(100).fill("source")), title: c.bad };
    expect(rank(a, c.q)).toBeGreaterThan(rank(b, c.q));
  }
});
