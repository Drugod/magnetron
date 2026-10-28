import { locale } from "./i18n";
export interface Torrent {
  id: string;
  title: string;
  infohash_v1: string;
  magnet_uri: string;
  size_bytes: number | null;
  published_at: string | null;
  seeds_reported: number | null;
  availability_status: string;
  availability_checked_at: string | null;
  sources: string[];
  files: string[];
  content_types?: string[];
  network?: "bittorrent" | "ed2k";
  ed2k_uri?: string | null;
  complete_sources_reported?: number | null;
  discovered_at: string;
}
export interface Settings {
  ubuntu: boolean;
  debian: boolean;
  archive: boolean;
  ed2k: boolean;
  kad: boolean;
  auto_sources: boolean;
  download_folder: string;
  video_quality: number;
  download_limit_kib: number;
  upload_limit_kib: number;
  history: boolean;
  concurrency: number;
  timeout_seconds: number;
  cache_hours: number;
  indexers: string[];
  local_index: boolean;
  shared_folders: SharedFolder[];
  index_max_records: number;
  index_max_mib: number;
  discovery_interval_seconds: number;
  discovery_timeout_seconds: number;
  discovery_enabled: boolean;
  discovery_start_hour: number;
  discovery_end_hour: number;
}
export interface SharedFolder {
  path: string;
  bittorrent: boolean;
  ed2k: boolean;
}
export interface SearchEvent {
  search_id: string;
  kind: string;
  results: Torrent[];
  source: string | null;
  message: string | null;
}
export const defaults: Settings = {
  ubuntu: true,
  debian: true,
  archive: true,
  ed2k: true,
  kad: true,
  auto_sources: true,
  download_folder: "",
  video_quality: 1080,
  download_limit_kib: 0,
  upload_limit_kib: 0,
  history: false,
  concurrency: 4,
  timeout_seconds: 60,
  cache_hours: 24,
  indexers: ["torrentscsv", "knaben", "nyaasi", "animetosho-xyz"],
  local_index: true,
  shared_folders: [],
  index_max_records: 50000,
  index_max_mib: 256,
  discovery_interval_seconds: 5,
  discovery_timeout_seconds: 25,
  discovery_enabled: false,
  discovery_start_hour: 0,
  discovery_end_hour: 24,
};
export function mergeResults(
  previous: Torrent[],
  incoming: Torrent[],
): Torrent[] {
  const map = new Map(
    previous.map((t) => [t.id, { ...t, sources: [...t.sources] }]),
  );
  for (const t of incoming) {
    const old = map.get(t.id);
    map.set(
      t.id,
      old
        ? {
            ...t,
            sources: [...new Set([...old.sources, ...t.sources])],
            content_types: [
              ...new Set([
                ...(old.content_types ?? []),
                ...(t.content_types ?? []),
              ]),
            ],
            files: t.files.length ? t.files : old.files,
            size_bytes: t.size_bytes ?? old.size_bytes,
            published_at: t.published_at ?? old.published_at,
          }
        : t,
    );
  }
  return [...map.values()];
}
export function bytes(n: number | null): string {
  if (n === null) return "Tamaño desconocido";
  if (n < 1024) return `${n} B`;
  const units = ["KiB", "MiB", "GiB", "TiB"];
  const exp = Math.min(Math.floor(Math.log(n) / Math.log(1024)), 4);
  return `${(n / 1024 ** exp).toLocaleString(locale(), { maximumFractionDigits: 2 })} ${units[exp - 1]}`;
}
export function rank(t: Torrent, query: string): number {
  const normalize = (s: string) =>
    s
      .normalize("NFKD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, " ")
      .trim();
  const title = normalize(t.title),
    words = new Set(title.split(" "));
  const q = normalize(query),
    tokens = [...new Set(q.split(" ").filter(Boolean))];
  if (!tokens.length) return 0;
  const exact = tokens.filter((w) => words.has(w)).length;
  const partial = tokens.filter(
    (w) => !words.has(w) && title.includes(w),
  ).length;
  return (
    (exact === tokens.length ? 1000 : 0) +
    (title === q
      ? 500
      : title.startsWith(q + " ")
        ? 150
        : title.includes(q)
          ? 100
          : 0) +
    exact * 30 +
    partial * 3 +
    Math.min(5, t.sources.length) * 2 +
    (t.size_bytes !== null ? 1 : 0)
  );
}

export const contentLabels = {
  iso: "ISO / imágenes de disco",
  video: "Vídeo",
  documents: "Documentos",
  audio: "Audio",
  images: "Imágenes",
  archives: "Comprimidos",
  software: "Programas / juegos",
  other: "Otros ficheros",
  unknown: "Sin clasificar",
} as const;
export type ContentType = keyof typeof contentLabels;
const extensions: Record<string, ContentType> = {
  iso: "iso",
  img: "iso",
  bin: "iso",
  nrg: "iso",
  mdf: "iso",
  vhd: "iso",
  vhdx: "iso",
  vmdk: "iso",
  mp4: "video",
  mkv: "video",
  avi: "video",
  mov: "video",
  webm: "video",
  wmv: "video",
  m4v: "video",
  mpg: "video",
  mpeg: "video",
  ts: "video",
  m2ts: "video",
  vob: "video",
  flv: "video",
  pdf: "documents",
  epub: "documents",
  mobi: "documents",
  azw: "documents",
  azw3: "documents",
  doc: "documents",
  docx: "documents",
  odt: "documents",
  rtf: "documents",
  txt: "documents",
  xls: "documents",
  xlsx: "documents",
  ods: "documents",
  csv: "documents",
  ppt: "documents",
  pptx: "documents",
  odp: "documents",
  djvu: "documents",
  cbz: "documents",
  cbr: "documents",
  mp3: "audio",
  flac: "audio",
  wav: "audio",
  ogg: "audio",
  opus: "audio",
  aac: "audio",
  m4a: "audio",
  wma: "audio",
  aiff: "audio",
  alac: "audio",
  jpg: "images",
  jpeg: "images",
  png: "images",
  gif: "images",
  webp: "images",
  bmp: "images",
  svg: "images",
  tif: "images",
  tiff: "images",
  heic: "images",
  avif: "images",
  zip: "archives",
  rar: "archives",
  "7z": "archives",
  tar: "archives",
  gz: "archives",
  bz2: "archives",
  xz: "archives",
  zst: "archives",
  exe: "software",
  msi: "software",
  msix: "software",
  apk: "software",
  appx: "software",
  deb: "software",
  rpm: "software",
  dmg: "software",
  appimage: "software",
  stl: "other",
  obj: "other",
  blend: "other",
  psd: "other",
  psb: "other",
  ai: "other",
  sql: "other",
  sqlite: "other",
  db: "other",
  json: "other",
  xml: "other",
  srt: "other",
  ass: "other",
};
export function contentTypes(t: Torrent): ContentType[] {
  const found = new Set<ContentType>();
  for (const type of t.content_types ?? [])
    if (Object.hasOwn(contentLabels, type) && type !== "unknown")
      found.add(type as ContentType);
  for (const name of [t.title, ...t.files]) {
    const normalized = name.toLowerCase().replace(/\.torrent$/, "");
    for (const match of normalized.matchAll(
      /\.([a-z0-9]{1,8})(?=$|[\s()[\]{};,])/g,
    )) {
      const type = extensions[match[1]];
      if (type) found.add(type);
    }
  }
  return found.size ? [...found] : ["unknown"];
}
export function matchesContent(t: Torrent, type: string): boolean {
  return type === "all" || contentTypes(t).includes(type as ContentType);
}

// A cached verdict is never proof of current availability.
export function isCurrentlyAvailable(t: Torrent, now = Date.now()): boolean {
  const checked = Date.parse(t.availability_checked_at ?? "");
  const ed2k = t.network === "ed2k";
  const admitted = ed2k
    ? t.availability_status === "reported_complete" &&
      (t.complete_sources_reported ?? 0) > 0
    : ["verified", "verified_v2"].includes(t.availability_status);
  return (
    admitted &&
    Number.isFinite(checked) &&
    checked <= now &&
    now - checked < 5 * 60_000
  );
}
