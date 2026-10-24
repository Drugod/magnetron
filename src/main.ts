import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  bytes,
  defaults,
  mergeResults,
  rank,
  contentLabels,
  contentTypes,
  matchesContent,
  isCurrentlyAvailable,
  type Torrent,
  type Settings,
  type SearchEvent,
} from "./model";
import "./style.css";
import { setupDownloads } from "./downloads";
import { setupViews, showView } from "./views";
import {
  setupLanguageAndAppearance,
  setLanguage,
  setAppearance,
  getLanguage,
  getAppearance,
  locale,
  translate,
} from "./i18n";
import type { SharedFolder } from "./model";

const icons = {
  search: '<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/>',
  grid: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
  settings:
    '<path d="m9.5 3-.5 2a7 7 0 0 0-1.5.9l-2-.6-2.5 4.4L4.5 11a7 7 0 0 0 0 2L3 14.3l2.5 4.4 2-.6A7 7 0 0 0 9 19l.5 2h5l.5-2a7 7 0 0 0 1.5-.9l2 .6 2.5-4.4-1.5-1.3a7 7 0 0 0 0-2L21 9.7l-2.5-4.4-2 .6A7 7 0 0 0 15 5l-.5-2Z"/><circle cx="12" cy="12" r="3.2"/>',
  arrow: '<path d="M5 12h14m-6-6 6 6-6 6"/>',
  link: '<path d="m10 13 4-4m-5 7-2 2a4 4 0 0 1-6-6l4-4a4 4 0 0 1 6 0m2 0 2-2a4 4 0 0 1 6 6l-4 4a4 4 0 0 1-6 0"/>',
  clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  shield:
    '<path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6Z"/><path d="m8 12 3 3 5-6"/>',
  download: '<path d="M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5"/>',
  close: '<path d="m6 6 12 12M6 18 18 6"/>',
};
function icon(name: keyof typeof icons) {
  return `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${icons[name]}</svg>`;
}
const app = document.querySelector<HTMLDivElement>("#app")!;
app.innerHTML = `
<aside class="sidebar"><a class="brand" href="#" aria-label="MAGNETRON inicio"><span class="brandmark">∩</span> MAGNETRON<span class="beta">0.7.0</span></a>
<nav><button class="nav active" id="nav-search">${icon("search")}Explorar</button></nav><button class="settings-gear" id="nav-settings" aria-label="Ajustes" title="Ajustes">${icon("settings")}</button>
</aside>
<div class="workspace">
<main><h1 class="search-title">Buscar</h1>
<form id="search-form" class="search-box">${icon("search")}<textarea id="query" rows="1" maxlength="1000000" placeholder="Buscar o pegar enlaces…" autocomplete="off" aria-label="Buscar o pegar enlaces" spellcheck="false"></textarea><button type="button" id="clear-search" class="clear-search" aria-label="Limpiar búsqueda" title="Limpiar búsqueda" hidden>×</button><kbd>Ctrl K</kbd><button type="submit" id="search-button" aria-label="Buscar o descargar">${icon("arrow")}</button></form>
<section id="browser-notice" class="notice" hidden>Vista previa de interfaz. La búsqueda real y SQLite funcionan en la aplicación de escritorio. Ejecuta <code>npm run desktop</code>.</section>
<div class="results-heading"><div><h2 id="results-title">Listo para explorar</h2><span id="result-count">Catálogos públicos, resultados reales.</span></div><span id="search-state" class="quiet-status"><span class="dot"></span> En espera</span></div>
<div class="filter-bar"><div class="filter-left"><select id="type-filter" aria-label="Filtrar por tipo de contenido"><option value="all">Todos los ficheros</option><option value="iso">ISO / imágenes de disco</option><option value="video">Vídeo</option><option value="documents">Documentos</option><option value="audio">Audio</option><option value="images">Imágenes</option><option value="archives">Comprimidos</option><option value="software">Programas / juegos</option><option value="other">Otros ficheros</option><option value="unknown">Sin clasificar</option></select></div><select id="sort" aria-label="Ordenar resultados"><option value="relevance">Orden: relevancia</option><option value="size">Orden: tamaño</option><option value="date">Orden: recientes</option><option value="name">Orden: nombre</option></select></div>
<p class="filter-hint" id="filter-hint" hidden>El tipo se obtiene de la categoría de la fuente o de las extensiones conocidas. Los paquetes mixtos pueden aparecer en varios tipos.</p><div id="results"></div><div id="empty" class="empty"><h3>Empieza por una buena búsqueda.</h3><p>Consultamos las fuentes por ti y agrupamos las coincidencias.<br>Los resultados aparecerán aquí, conforme lleguen.</p></div>
</main></div>
<dialog id="settings-dialog"><div class="dialog-heading"><div><span class="eyebrow">A TU MANERA</span><h2>Ajustes</h2></div><button class="icon-button close-dialog" aria-label="Cerrar ajustes">${icon("close")}</button></div><form id="settings-form"><h3>Fuentes de búsqueda</h3><p class="muted">Tus consultas se envían a los operadores de las fuentes activadas.</p><label class="setting-row"><span><strong>Ubuntu Releases</strong><small>Imágenes oficiales · releases.ubuntu.com</small></span><input type="checkbox" name="ubuntu"></label><label class="setting-row"><span><strong>Debian CD</strong><small>Instalación y Live · cdimage.debian.org</small></span><input type="checkbox" name="debian"></label><label class="setting-row"><span><strong>Internet Archive</strong><small>Catálogo público · libros, audio, vídeo y software</small></span><input type="checkbox" name="archive"></label><label class="setting-row"><span>Conservar índice local<small>Busca también metadatos guardados sin aplicarles caducidad.</small></span><input type="checkbox" name="local_index"></label><h3>Recursos y privacidad</h3><label class="setting-row"><span>Guardar historial<small>Solo en este equipo, hasta 100 búsquedas.</small></span><input type="checkbox" name="history"></label><label class="setting-row">Fuentes simultáneas<input name="concurrency" type="number" min="1" max="8" required></label><label class="setting-row">Tiempo por fuente (segundos)<input name="timeout_seconds" type="number" min="5" max="120" required></label><label class="setting-row">Caducidad de caché (horas)<input name="cache_hours" type="number" min="1" max="720" required></label><div class="notice">El descubrimiento DHT se activa en «Fuentes». Funciona solo mientras MAGNETRON está abierto y se pausa al cerrar. BitTorrent y eD2k descargan dentro de MAGNETRON.</div><div class="dialog-actions"><button type="button" id="clear-data" class="danger">Borrar caché e historial</button><button class="primary" type="submit">Guardar ajustes</button></div><p id="settings-feedback" role="status"></p></form></dialog>
<div id="toast" role="status" hidden></div>`;

const $ = <T extends HTMLElement = HTMLElement>(selector: string) =>
  document.querySelector<T>(selector)!;
interface Indexer {
  id: string;
  name: string;
  type: string;
  site_link: string;
  language: string;
}
interface IndexState {
  records: number;
  dht: {
    running: boolean;
    nodes: number;
    hashes: number;
    indexed: number;
    failures: number;
    message: string;
  };
}
let indexers: Indexer[] = [];
const directSources: Indexer[] = [
  {
    id: "@kad",
    name: "Kad / eMule",
    type: "direct",
    site_link: "Red descentralizada eD2k",
    language: "Todas",
  },
  {
    id: "@ed2k",
    name: "eD2k / eMule",
    type: "direct",
    site_link: "Servidores eD2k públicos",
    language: "Todas",
  },
  {
    id: "@ubuntu",
    name: "Ubuntu Releases",
    type: "direct",
    site_link: "https://releases.ubuntu.com",
    language: "Directa",
  },
  {
    id: "@debian",
    name: "Debian CD",
    type: "direct",
    site_link: "https://cdimage.debian.org",
    language: "Directa",
  },
  {
    id: "@archive",
    name: "Internet Archive",
    type: "direct",
    site_link: "https://archive.org",
    language: "Directa",
  },
];
const availableSources = () => [...directSources, ...indexers];
app.insertAdjacentHTML(
  "beforeend",
  `<dialog id="sources-dialog" class="large-dialog"><div class="dialog-heading"><div><span class="eyebrow">MÁS LUGARES DONDE BUSCAR</span><h2>Fuentes integradas</h2></div><button class="icon-button close-dialog" aria-label="Cerrar fuentes">${icon("close")}</button></div><p class="muted">BitTorrent y eD2k están integrados. eD2k busca en servidores públicos y descarga dentro de MAGNETRON. El motor de Jackett está incluido en MAGNETRON. Elige los conectores públicos que quieras consultar; cada sitio puede estar caído, bloquear el acceso o requerir configuración adicional. No se sortean sus controles.</p><input id="source-query" class="catalog-search" placeholder="Filtrar por nombre, idioma o dominio" aria-label="Filtrar fuentes"><div class="catalog-summary" id="catalog-summary"></div><details class="dht-source"><summary>DHT · descubrimiento adicional</summary><p class="muted">Descubre torrents y guarda sus metadatos. La comprobación de disponibilidad al buscar funciona automáticamente, aunque este descubrimiento esté pausado.</p><div id="index-metrics" class="index-metrics"></div><p id="index-message" role="status"></p><div class="dialog-actions"><button id="dht-pause" class="text-button">Pausar DHT</button><button id="dht-start" class="primary">Activar DHT</button></div></details><div id="catalog-list" class="catalog-list"></div><div class="dialog-actions"><div class="source-bulk-actions"><button id="sources-all" class="text-button">Marcar todas</button><button id="sources-clear" class="text-button">Desmarcar todas</button></div><button id="sources-save" class="primary">Guardar selección</button></div><p id="sources-feedback" role="status"></p></dialog>
`,
);
let settings: Settings = { ...defaults };
let pendingShares: SharedFolder[] = [];
let results: Torrent[] = [];
let savedItems: Torrent[] = [];
let savedIds = new Set<string>();
let showSaved = false;
let currentId = "";
let searchRequests: Promise<void> = Promise.resolve();
let currentQuery = "";
let searching = false;
let searched = false;
let sourceErrors = 0;
let startedAt = 0;
const desktop = isTauri();
const sourceName = (id: string) =>
  id === "ubuntu"
    ? "Ubuntu Releases"
    : id === "debian"
      ? "Debian CD"
      : id === "archive"
        ? "Internet Archive"
        : id === "dht"
          ? "Índice DHT"
          : id.startsWith("indexer:")
            ? (indexers.find((i) => `indexer:${i.id}` === id)?.name ??
              id.slice(8))
            : id;
function toast(message: string) {
  $("#toast").textContent = message;
  $("#toast").hidden = false;
  setTimeout(() => ($("#toast").hidden = true), 5000);
}
setupViews();
const downloader = setupDownloads(toast);
$("#settings-form").insertAdjacentHTML(
  "afterbegin",
  `<h3>Descargas</h3><label for="download-folder">Carpeta de descarga</label><div class="folder-setting"><input id="download-folder" name="download_folder" readonly placeholder="Se preguntará en la primera descarga"><button type="button" id="choose-default-folder" class="text-button">Elegir carpeta</button><button type="button" id="reset-default-folder" class="text-button">Quitar</button></div><p class="muted">Los archivos se guardan directamente aquí, sin carpetas añadidas por MAGNETRON.</p><label class="setting-row">Límite de bajada (KiB/s)<input name="download_limit_kib" type="number" min="0" max="1048576" step="1" required></label><label class="setting-row">Límite de subida (KiB/s)<input name="upload_limit_kib" type="number" min="0" max="1048576" step="1" required></label><p class="muted">0 = sin límite. Mínimo limitado: 32 KiB/s. Se reparte entre BitTorrent y eD2k cuando ambas redes están activas.</p>`,
);
$("#settings-form").insertAdjacentHTML(
  "afterbegin",
  `<section class="appearance-settings"><h3>Personalización</h3><label class="setting-row"><span>Idioma</span><select id="language-select" aria-label="Idioma"><option value="es" data-no-i18n>Español</option><option value="en" data-no-i18n>English</option></select></label><label class="setting-row"><span>Apariencia</span><select id="appearance-select" aria-label="Apariencia"><option value="system">Sistema</option><option value="light">Claro</option><option value="dark">Oscuro</option></select></label></section>`,
);
for (const name of ["ubuntu", "debian", "archive"])
  document.querySelector(`[name="${name}"]`)?.closest("label")?.remove();
const duplicateSourcesHeading = Array.from(
  $("#settings-form").querySelectorAll("h3"),
).find((el) => el.textContent === "Fuentes de búsqueda");
if (duplicateSourcesHeading) {
  duplicateSourcesHeading.nextElementSibling?.remove();
  duplicateSourcesHeading.remove();
}
const localIndex = document
  .querySelector('[name="local_index"]')
  ?.closest("label");
if (localIndex) $("#sources-dialog").prepend(localIndex);
$(".appearance-settings").insertAdjacentHTML(
  "beforeend",
  '<p class="muted background-note">Al cerrar la ventana, las descargas continúan en la bandeja del sistema. Para cerrar el programa, elige Salir en su icono.</p>',
);
$("#settings-form .dialog-actions").insertAdjacentHTML(
  "beforebegin",
  `<details class="advanced-settings"><summary>Carpetas compartidas</summary><p class="muted">Solo se comparten las carpetas que actives, incluidos sus subdirectorios. BitTorrent crea un enlace magnet para compartir esa carpeta; eD2k publica sus archivos. Durante las descargas se siguen intercambiando piezas.</p><div id="shared-folders"></div><div class="dialog-actions"><button type="button" id="add-shared-folder" class="text-button">Añadir carpeta</button><button type="button" id="refresh-shares" class="text-button">Actualizar archivos compartidos</button></div><p id="sharing-feedback" role="status"></p></details><details class="advanced-settings"><summary>Índice y recursos</summary><label class="setting-row"><span>Rastreo DHT programado</span><input type="checkbox" name="discovery_enabled"></label><label class="setting-row">Desde la hora (0–23)<input type="number" name="discovery_start_hour" min="0" max="23" required></label><label class="setting-row">Hasta la hora (0–24)<input type="number" name="discovery_end_hour" min="0" max="24" required></label><label class="setting-row">Máximo de registros<input type="number" name="index_max_records" min="1000" max="1000000" required></label><label class="setting-row">Presupuesto de metadatos (MiB)<input type="number" name="index_max_mib" min="32" max="4096" required></label><label class="setting-row">Pausa entre rastreos (segundos)<input type="number" name="discovery_interval_seconds" min="2" max="3600" required></label><label class="setting-row">Tiempo por recuperación (segundos)<input type="number" name="discovery_timeout_seconds" min="5" max="120" required></label><p class="muted">Una recuperación simultánea limita CPU y red. El presupuesto corresponde al texto de metadatos; SQLite, su índice y los temporales ocupan espacio adicional. Los favoritos se conservan al limpiar la caché.</p></details>`,
);
function renderSharedFolders() {
  const list = $("#shared-folders");
  list.replaceChildren();
  for (const folder of pendingShares) {
    const row = document.createElement("section");
    row.className = "shared-folder";
    const path = textEl("strong", folder.path);
    path.dataset.noI18n = "";
    row.append(path);
    for (const [key, label] of [
      ["bittorrent", "BitTorrent"],
      ["ed2k", "eD2k"],
    ] as const) {
      const wrap = document.createElement("label");
      wrap.className = "share-toggle";
      wrap.append(textEl("span", label));
      const input = document.createElement("input");
      input.type = "checkbox";
      input.checked = folder[key];
      input.onchange = () => (folder[key] = input.checked);
      wrap.append(input);
      row.append(wrap);
    }
    const remove = textEl(
      "button",
      "Quitar",
      "text-button",
    ) as HTMLButtonElement;
    remove.type = "button";
    remove.onclick = () => {
      pendingShares = pendingShares.filter((f) => f !== folder);
      renderSharedFolders();
    };
    row.append(remove);
    row.dataset.sharePath = folder.path;
    list.append(row);
  }
}
$("#add-shared-folder").onclick = async () => {
  try {
    const path = await invoke<string | null>("choose_download_folder", {
      language: getLanguage(),
      purpose: "share",
    });
    if (
      path &&
      !pendingShares.some((f) => f.path.toLowerCase() === path.toLowerCase())
    ) {
      pendingShares.push({ path, bittorrent: false, ed2k: false });
      renderSharedFolders();
    }
  } catch (e) {
    toast(String(e));
  }
};
$("#refresh-shares").onclick = () => {
  void invoke("refresh_sharing")
    .then(() => toast("Compartición actualizada"))
    .catch((e) => toast(String(e)));
};
async function refreshSharing() {
  if (!desktop || $("#settings-dialog").hidden) return;
  try {
    const statuses =
      await invoke<
        {
          path: string;
          state: string;
          magnet: string | null;
          error: string | null;
        }[]
      >("sharing_status");
    for (const row of $("#shared-folders").querySelectorAll<HTMLElement>(
      ".shared-folder",
    )) {
      row.querySelector(".share-status")?.remove();
      row.querySelector(".copy-share")?.remove();
      const state = statuses.find((s) => s.path === row.dataset.sharePath);
      if (state) {
        const info = textEl("p", state.error ?? state.state, "share-status");
        row.append(info);
        if (state.magnet && !row.querySelector(".copy-share")) {
          const copy = textEl(
            "button",
            "Copiar enlace magnet",
            "text-button copy-share",
          ) as HTMLButtonElement;
          copy.type = "button";
          copy.onclick = () => {
            void navigator.clipboard
              .writeText(state.magnet!)
              .then(() => toast("Enlace copiado"));
          };
          row.append(copy);
        }
      }
    }
  } catch (e) {
    $("#sharing-feedback").textContent = String(e);
  }
}
window.setInterval(() => void refreshSharing(), 2000);
$<HTMLSelectElement>("#language-select").value = getLanguage();
$<HTMLSelectElement>("#appearance-select").value = getAppearance();
$("#language-select").onchange = () =>
  setLanguage($<HTMLSelectElement>("#language-select").value as "es" | "en");
$("#appearance-select").onchange = () =>
  setAppearance(
    $<HTMLSelectElement>("#appearance-select").value as
      "system" | "light" | "dark",
  );
document.addEventListener("magnetron:appearance", () => {
  if (desktop)
    void invoke("set_window_appearance", { theme: getAppearance() }).catch(
      () => {},
    );
});
document.addEventListener("magnetron:language", () => {
  render();
  if (desktop)
    void invoke("set_ui_language", { language: getLanguage() }).catch(() => {});
});
setupLanguageAndAppearance();
$("#choose-default-folder").onclick = async () => {
  try {
    if (!desktop) return toast("Disponible en escritorio");
    const folder = await invoke<string | null>("choose_download_folder", {
      language: getLanguage(),
    });
    if (folder) $<HTMLInputElement>("#download-folder").value = folder;
  } catch (e) {
    toast(String(e));
  }
};
$("#reset-default-folder").onclick = () => {
  $<HTMLInputElement>("#download-folder").value = "";
};
document.addEventListener("magnet:settings", (e) => {
  settings = (e as CustomEvent<Settings>).detail;
});
function textEl(tag: string, text: string, className = "") {
  const el = document.createElement(tag);
  el.textContent = text;
  el.className = className;
  return el;
}
function state(text: string) {
  $("#search-state").textContent = text;
  $("#search-state").classList.toggle("running", searching);
}
function render() {
  const type = $<HTMLSelectElement>("#type-filter").value;

  const sort = $<HTMLSelectElement>("#sort").value;
  const viewItems = showSaved ? savedItems : results;
  const filtered = viewItems.filter(
    (t) => matchesContent(t, type) && (showSaved || isCurrentlyAvailable(t)),
  );
  filtered.sort((a, b) =>
    sort === "size"
      ? (b.size_bytes ?? -1) - (a.size_bytes ?? -1)
      : sort === "date"
        ? Date.parse(b.published_at ?? "1970") -
          Date.parse(a.published_at ?? "1970")
        : sort === "name"
          ? a.title.localeCompare(b.title)
          : rank(b, currentQuery) - rank(a, currentQuery),
  );
  $("#results").replaceChildren();
  for (const t of filtered) {
    const row = document.createElement("article");
    row.className = "result";
    const glyph = textEl(
      "div",
      t.network === "ed2k"
        ? "eD"
        : t.sources.includes("archive")
          ? "A"
          : t.title.toLowerCase().includes("ubuntu")
            ? "U"
            : "D",
      "torrent-glyph",
    );
    const body = document.createElement("div");
    body.className = "result-body";
    const title = textEl("h3", t.title);
    title.dataset.noI18n = "";
    body.append(title);
    const meta = document.createElement("div");
    meta.className = "result-meta";
    meta.append(
      textEl("span", bytes(t.size_bytes)),
      textEl(
        "span",
        t.published_at
          ? new Date(t.published_at).toLocaleDateString(locale())
          : "Fecha desconocida",
      ),
      textEl(
        "span",
        showSaved && !isCurrentlyAvailable(t)
          ? "Guardado · vuelve a comprobar al descargar"
          : t.network === "ed2k"
            ? `eD2k · ${t.complete_sources_reported} completas (red)`
            : ["verified", "verified_v2"].includes(t.availability_status)
              ? isCurrentlyAvailable(t)
                ? "BitTorrent · Fuente completa accesible"
                : "Guardado · vuelve a comprobar al descargar"
              : t.availability_status === "reported"
                ? "Reportada"
                : "Sin comprobar",
        "availability",
      ),
    );
    body.append(meta);
    const types = contentTypes(t);
    body.append(
      textEl(
        "span",
        types.map((type) => contentLabels[type]).join(" · "),
        "content-badge",
      ),
    );
    const details = document.createElement("details");
    const summary = textEl(
      "summary",
      `${t.sources.length} ${t.sources.length === 1 ? "fuente" : "fuentes"} · Ver detalles`,
    );
    details.append(
      summary,
      textEl("p", t.sources.map(sourceName).join(" · ")),
      textEl(
        "code",
        t.network === "ed2k"
          ? t.id.split(":")[1]
          : t.infohash_v1 || t.id.replace("bt2:", ""),
      ),
      textEl(
        "p",
        `Descubierto: ${new Date(t.discovered_at).toLocaleString(locale())}`,
      ),
      textEl(
        "p",
        t.network === "ed2k"
          ? `Fuentes completas reportadas por la red: ${new Date(t.availability_checked_at!).toLocaleTimeString(locale())}. Indica fuentes completas; no se ha comprobado su conexión ni su cola de descarga. La descarga se gestiona dentro de MAGNETRON.`
          : t.availability_status === "verified_v2"
            ? "BitTorrent v2: metadatos verificados y una fuente completa conectada. Su disponibilidad puede cambiar."
            : `Comprobación directa: ${new Date(t.availability_checked_at!).toLocaleTimeString(locale())}. Un peer anuncia todas las piezas y permite solicitarlas. Sus metadatos coinciden con el hash. La conexión puede cambiar; no se ha descargado el contenido.`,
      ),
    );
    if (t.seeds_reported !== null)
      details.append(
        textEl(
          "p",
          `Semillas reportadas por la fuente: ${t.seeds_reported}. No es una medición propia.`,
        ),
      );
    if (t.files.length) {
      details.append(textEl("p", `${t.files.length} archivos indexados`));
      for (const name of t.files.slice(0, 10)) {
        const file = textEl("p", name);
        file.dataset.noI18n = "";
        details.append(file);
      }
    }
    body.append(details);
    const actions = document.createElement("div");
    actions.className = "result-actions";
    const favorite = textEl(
      "button",
      savedIds.has(t.id) ? "★" : "☆",
      "favorite-button",
    ) as HTMLButtonElement;
    favorite.setAttribute(
      "aria-label",
      savedIds.has(t.id) ? "Quitar de favoritos" : "Guardar favorito",
    );
    favorite.setAttribute("aria-pressed", String(savedIds.has(t.id)));
    favorite.title = savedIds.has(t.id)
      ? "Quitar de favoritos"
      : "Guardar favorito";
    favorite.onclick = async () => {
      try {
        const enabled = !savedIds.has(t.id);
        await invoke("set_favorite", { torrent: t, enabled });
        if (enabled) savedIds.add(t.id);
        else savedIds.delete(t.id);
        if (showSaved) savedItems = await invoke<Torrent[]>("list_favorites");
        render();
      } catch (e) {
        toast(String(e));
      }
    };
    actions.append(favorite);
    const copy = textEl("button", "Copiar", "copy-button");
    copy.onclick = async () => {
      try {
        await navigator.clipboard.writeText(t.ed2k_uri ?? t.magnet_uri);
        toast("Enlace copiado");
      } catch {
        toast("No se pudo acceder al portapapeles");
      }
    };
    const openLabel = "Descargar";
    const open = textEl(
      "button",
      openLabel,
      "open-button",
    ) as HTMLButtonElement;
    open.onclick = async () => {
      open.disabled = true;
      open.textContent = "Preparando…";
      try {
        await downloader.download(t);
      } catch (e) {
        toast(String(e));
      } finally {
        open.disabled = false;
        open.textContent = openLabel;
      }
    };
    actions.append(copy, open);
    row.append(glyph, body, actions);
    $("#results").append(row);
  }
  $("#empty").hidden = filtered.length > 0;
  if (searched) {
    $("#empty h3").textContent = searching
      ? "Buscando fuentes completas accesibles…"
      : results.length
        ? "Ningún resultado con estos filtros."
        : sourceErrors
          ? "Búsqueda finalizada con errores de fuente."
          : "No se encontraron fuentes completas accesibles.";
    $("#empty p").textContent = searching
      ? "Solo aparecen archivos que superan la comprobación directa. Puede tardar unos minutos."
      : sourceErrors
        ? "Puedes consultar la caché disponible o volver a intentarlo."
        : "No pudimos comprobar una fuente completa desde tu equipo. Eso no significa que el archivo no exista; puedes probar más tarde o cambiar la búsqueda.";
  } else {
    $("#empty h3").textContent = "Empieza por una buena búsqueda.";
    $("#empty p").textContent =
      "Escribe lo que quieres encontrar y pulsa Buscar. Después puedes filtrar los resultados.";
  }
  $("#results-title").textContent = searched
    ? "Resultados de búsqueda"
    : "Listo para explorar";
  $("#result-count").textContent = searched
    ? `${filtered.length} ${filtered.length === 1 ? "coincidencia" : "coincidencias"}${filtered.length !== results.length ? ` de ${results.length}` : ""} · ${currentQuery}`
    : "BitTorrent comprobado en directo · eD2k con fuentes completas reportadas.";
  if (showSaved) {
    $("#results-title").textContent = "Favoritos";
    $("#result-count").textContent = `${filtered.length} guardados`;
    $<HTMLElement>("#empty h3").textContent = "Todavía no hay favoritos";
    $("#empty p").textContent =
      "Guarda resultados con la estrella para encontrarlos aquí.";
  }
  $("#filter-hint").hidden = type === "all";
  updateClearButton();
}
function finish(message: string) {
  searching = false;
  state(message);
  render();
}
async function search(query: string) {
  query = query.trim();
  if (!query) return;
  showSaved = false;
  showView("search");
  if (/^(ed2k:\/\/|magnet:)/i.test(query)) {
    await downloader.addLinks(query);
    return;
  }
  if (query.length > 200) {
    toast("La búsqueda admite hasta 200 caracteres");
    return;
  }
  if (!desktop) {
    toast("Abre MAGNETRON de escritorio para consultar las fuentes reales.");
    return;
  }
  currentId = crypto.randomUUID();
  currentQuery = query;
  results = [];
  searching = true;
  searched = true;
  sourceErrors = 0;
  startedAt = performance.now();
  state("Buscando y comprobando fuentes completas…");
  render();
  const searchId = currentId;
  searchRequests = searchRequests
    .catch(() => {})
    .then(async () => {
      if (currentId !== searchId) return;
      try {
        await invoke("start_search", { query, searchId });
      } catch (e) {
        if (currentId !== searchId) return;
        finish("No se pudo buscar");
        toast(String(e));
      }
    });
  await searchRequests;
}
$("#search-form").onsubmit = (e) => {
  e.preventDefault();
  void search($<HTMLInputElement>("#query").value);
};
$("#query").onkeydown = (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    void search($<HTMLTextAreaElement>("#query").value);
  }
};
document.querySelectorAll<HTMLButtonElement>("[data-query]").forEach(
  (button) =>
    (button.onclick = () => {
      $<HTMLInputElement>("#query").value = button.dataset.query!;
      void search(button.dataset.query!);
    }),
);
for (const id of ["#type-filter", "#sort"]) $(id).onchange = render;
function updateClearButton() {
  $("#clear-search").hidden = !searched && !$<HTMLInputElement>("#query").value;
}
function resetFilters() {
  $<HTMLSelectElement>("#type-filter").value = "all";
  $<HTMLSelectElement>("#sort").value = "relevance";
}
$<HTMLInputElement>("#query").oninput = updateClearButton;
$("#clear-search").onclick = () => {
  currentId = "";
  currentQuery = "";
  results = [];
  searched = false;
  sourceErrors = 0;
  $<HTMLInputElement>("#query").value = "";
  resetFilters();
  finish("En espera");
  $<HTMLInputElement>("#query").focus();
  if (desktop) void invoke("cancel_search").catch((e) => toast(String(e)));
};
document.addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === "k") {
    e.preventDefault();
    $<HTMLInputElement>("#query").focus();
  }
});
function showSettings() {
  pendingShares = settings.shared_folders.map((f) => ({ ...f }));
  renderSharedFolders();
  void refreshSharing();
  $("#nav-sources .muted").textContent = settings.auto_sources
    ? "Automáticas"
    : "Manual";
  const form = $<HTMLFormElement>("#settings-form");
  for (const [key, value] of Object.entries(settings)) {
    const input = form.elements.namedItem(key) as HTMLInputElement;
    if (!input || Array.isArray(value)) continue;
    if (typeof value === "boolean") input.checked = value;
    else input.value = String(value);
  }
  $("#settings-feedback").textContent = "";
  showView("settings");
}
$("#nav-settings").onclick = showSettings;

$("#nav-search").onclick = () => {
  showSaved = false;
  showView("search");
  $("#nav-favorites").classList.remove("active");
  render();
  $<HTMLInputElement>("#query").focus();
};
$(".brand").onclick = (e) => {
  e.preventDefault();
  showView("search");
  $<HTMLInputElement>("#query").focus();
};
document
  .querySelectorAll<HTMLButtonElement>(".close-dialog")
  .forEach((b) => (b.onclick = () => showView("search")));
$("#settings-form").onsubmit = async (e) => {
  e.preventDefault();
  const form = $<HTMLFormElement>("#settings-form");
  const next = { ...settings };
  next.shared_folders = pendingShares.map((f) => ({ ...f }));
  for (const key of Object.keys(next) as (keyof Settings)[]) {
    const input = form.elements.namedItem(key) as HTMLInputElement;
    if (!input || Array.isArray(next[key])) continue;
    Object.assign(next, {
      [key]:
        typeof next[key] === "boolean"
          ? input.checked
          : typeof next[key] === "string"
            ? input.value
            : Number(input.value),
    });
  }
  try {
    if (!desktop)
      throw new Error(
        "Los ajustes persistentes requieren la aplicación de escritorio.",
      );
    await invoke("save_settings", { settings: next });
    settings = next;
    showView("search");
    toast("Ajustes guardados");
  } catch (e) {
    $("#settings-feedback").textContent = String(e);
  }
};
$("#clear-data").onclick = async () => {
  if (!desktop) return toast("Disponible en la aplicación de escritorio");
  if (
    !confirm(
      translate(
        "¿Borrar la caché y todo el historial local? Las preferencias se conservan.",
      ),
    )
  )
    return;
  try {
    currentId = "";
    await invoke("clear_data");
    results = [];
    finish("Datos locales eliminados");
    $("#settings-feedback").textContent = "Caché e historial eliminados.";
  } catch (e) {
    $("#settings-feedback").textContent = String(e);
  }
};
async function init() {
  if (!desktop) {
    $("#browser-notice").hidden = false;
    return;
  }
  try {
    await listen<SearchEvent>("search-event", ({ payload: e }) => {
      if (e.search_id !== currentId) return;
      if (e.kind === "cache" || e.kind === "batch") {
        results = mergeResults(results, e.results);
        render();
      } else if (e.kind === "checking")
        state(e.message ?? "Comprobando disponibilidad…");
      else if (e.kind === "limited")
        finish("Tiempo de comprobación agotado · resultados parciales");
      else if (e.kind.startsWith("source-")) {
        if (e.kind === "source-error") sourceErrors++;
      } else if (e.kind === "done")
        finish(
          `${sourceErrors ? "Resultados parciales" : "Comprobación terminada"} · ${((performance.now() - startedAt) / 1000).toFixed(1)} s`,
        );
      else if (e.kind === "cancelled") finish("Búsqueda cancelada");
      else if (e.kind === "error") {
        finish("Error de búsqueda");
        toast(e.message ?? "Error");
      } else if (e.kind === "warning") toast(e.message ?? "Aviso");
    });
    settings = await invoke<Settings>("get_settings");
    savedItems = await invoke<Torrent[]>("list_favorites");
    savedIds = new Set(savedItems.map((t) => t.id));
    await invoke("set_ui_language", { language: getLanguage() });
    await invoke("set_window_appearance", { theme: getAppearance() });
  } catch (e) {
    toast(`No se pudo inicializar: ${String(e)}`);
  }
}
void init();
$("#nav-downloads").insertAdjacentHTML(
  "afterend",
  `<button class="nav" id="nav-favorites" aria-label="Favoritos"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2L12 17.3l-5.6 2.9 1.1-6.2L3 9.6l6.2-.9Z"/></svg>Favoritos</button>`,
);
$("#nav-favorites").onclick = async () => {
  try {
    savedItems = await invoke<Torrent[]>("list_favorites");
    savedIds = new Set(savedItems.map((t) => t.id));
    showSaved = true;
    showView("search");
    $("#nav-search").classList.remove("active");
    $("#nav-favorites").classList.add("active");
    $("#nav-favorites").setAttribute("aria-current", "page");
    render();
  } catch (e) {
    toast(String(e));
  }
};
$(".filter-bar").insertAdjacentHTML(
  "beforeend",
  `<div class="export-actions"><select id="export-format" aria-label="Formato de exportación"><option value="links">Enlaces</option><option value="csv">CSV</option><option value="json">JSON</option></select><button type="button" id="export-results" class="text-button" title="Exportar resultados">Exportar</button></div>`,
);
$("#export-results").onclick = async () => {
  try {
    const type = $<HTMLSelectElement>("#type-filter").value;
    const items = (showSaved ? savedItems : results).filter(
      (t) => matchesContent(t, type) && (showSaved || isCurrentlyAvailable(t)),
    );
    const saved = await invoke<boolean>("export_results", {
      items,
      format: $<HTMLSelectElement>("#export-format").value,
      language: getLanguage(),
    });
    if (saved) toast("Resultados exportados");
  } catch (e) {
    toast(String(e));
  }
};
window.setInterval(() => {
  if (results.length) render();
}, 15000);

let pendingSources = new Set<string>();
$("#source-query").insertAdjacentHTML(
  "beforebegin",
  `<label class="setting-row"><span><strong>Fuentes automáticas</strong><small>Incluye nuevas fuentes públicas, prioriza las que responden y reintenta las que fallan. El motor se actualiza en segundo plano y se activa al volver a abrir MAGNETRON.</small></span><input type="checkbox" id="auto-sources"></label>`,
);
function renderCatalog() {
  const filter = $<HTMLInputElement>("#source-query").value.toLowerCase();
  const visible = availableSources().filter((i) =>
    `${i.name} ${i.language} ${i.site_link}`.toLowerCase().includes(filter),
  );
  const list = $("#catalog-list");
  list.replaceChildren();
  $("#catalog-summary").textContent =
    `${availableSources().length} fuentes disponibles · ${pendingSources.size} seleccionadas · ${visible.length} visibles`;
  for (const i of visible) {
    const row = document.createElement("label");
    row.className = "catalog-row";
    const input = document.createElement("input");
    input.type = "checkbox";
    input.checked = pendingSources.has(i.id);
    input.onchange = () => {
      $<HTMLInputElement>("#auto-sources").checked = false;
      if (input.checked) pendingSources.add(i.id);
      else pendingSources.delete(i.id);
      $("#catalog-summary").textContent =
        `${availableSources().length} fuentes disponibles · ${pendingSources.size} seleccionadas · ${visible.length} visibles`;
    };
    const info = document.createElement("span");
    info.append(
      textEl("strong", i.name),
      textEl("small", `${i.language} · ${i.site_link}`),
    );
    row.append(input, info);
    list.append(row);
  }
}
$("#nav-sources").onclick = async () => {
  pendingSources = new Set(settings.indexers);
  $<HTMLInputElement>("#auto-sources").checked = settings.auto_sources;
  if (settings.ubuntu) pendingSources.add("@ubuntu");
  if (settings.debian) pendingSources.add("@debian");
  if (settings.archive) pendingSources.add("@archive");
  if (settings.ed2k) pendingSources.add("@ed2k");
  if (settings.kad) pendingSources.add("@kad");
  $<HTMLButtonElement>("#sources-all").disabled = true;
  renderCatalog();
  $("#sources-feedback").textContent =
    "Arrancando el motor integrado y leyendo conectores…";
  showView("sources");
  void refreshIndex();
  try {
    if (!desktop) throw new Error("Disponible en la aplicación de escritorio");
    indexers = await invoke<Indexer[]>("list_indexers");
    if (settings.auto_sources)
      for (const i of indexers) pendingSources.add(i.id);
    renderCatalog();
    $<HTMLButtonElement>("#sources-all").disabled = false;
    $("#sources-feedback").textContent =
      "Los cambios se aplican a la siguiente búsqueda.";
  } catch (e) {
    $("#sources-feedback").textContent = String(e);
  }
};
$("#source-query").oninput = renderCatalog;
$("#sources-all").onclick = () => {
  $<HTMLInputElement>("#auto-sources").checked = false;
  pendingSources = new Set(availableSources().map((i) => i.id));
  renderCatalog();
};
$("#sources-clear").onclick = () => {
  $<HTMLInputElement>("#auto-sources").checked = false;
  pendingSources.clear();
  renderCatalog();
};
$("#auto-sources").onchange = () => {
  if ($<HTMLInputElement>("#auto-sources").checked) {
    for (const i of indexers) pendingSources.add(i.id);
    renderCatalog();
  }
};
$("#sources-save").onclick = async () => {
  try {
    const next = {
      ...settings,
      auto_sources: $<HTMLInputElement>("#auto-sources").checked,
      indexers: [...pendingSources].filter((id) => !id.startsWith("@")),
      ubuntu: pendingSources.has("@ubuntu"),
      debian: pendingSources.has("@debian"),
      archive: pendingSources.has("@archive"),
      ed2k: pendingSources.has("@ed2k"),
      kad: pendingSources.has("@kad"),
    };
    await invoke("save_settings", { settings: next });
    settings = next;
    showView("settings");
    toast("Fuentes guardadas. Se aplicarán a la siguiente búsqueda.");
  } catch (e) {
    $("#sources-feedback").textContent = String(e);
  }
};
async function refreshIndex() {
  if (
    !desktop ||
    $("#settings-dialog").hidden ||
    !$<HTMLDetailsElement>("#sources-settings").open
  )
    return;
  try {
    const s = await invoke<IndexState>("index_status");
    const metrics = $("#index-metrics");
    metrics.replaceChildren();
    for (const [label, value] of [
      ["Registros guardados", s.records],
      ["Nodos que respondieron", s.dht.nodes],
      ["Hashes descubiertos", s.dht.hashes],
      ["Metadatos recuperados", s.dht.indexed],
      ["Intentos sin metadatos", s.dht.failures],
    ]) {
      const box = document.createElement("div");
      box.append(
        textEl("strong", String(value)),
        textEl("span", String(label)),
      );
      metrics.append(box);
    }
    $("#index-message").textContent = s.dht.message;
    $<HTMLButtonElement>("#dht-start").disabled = s.dht.running;
    $<HTMLButtonElement>("#dht-pause").disabled = !s.dht.running;
  } catch (e) {
    $("#index-message").textContent = String(e);
  }
}
$("#dht-start").onclick = async () => {
  try {
    await invoke("start_dht");
    settings = await invoke<Settings>("get_settings");
    await refreshIndex();
  } catch (e) {
    $("#index-message").textContent = String(e);
  }
};
$("#dht-pause").onclick = async () => {
  try {
    await invoke("pause_dht");
    settings = await invoke<Settings>("get_settings");
    await refreshIndex();
  } catch (e) {
    $("#index-message").textContent = String(e);
  }
};
setInterval(() => void refreshIndex(), 3000);

$("#settings-form .dialog-actions").insertAdjacentHTML(
  "beforebegin",
  `<details class="advanced-settings"><summary>Actualizaciones de MAGNETRON</summary><p id="app-update-state" class="muted" role="status">El canal público se activará al publicar la primera versión.</p><div class="dialog-actions"><button type="button" id="check-app-update" class="text-button">Comprobar actualizaciones</button><button type="button" id="install-app-update" class="text-button" hidden>Instalar actualización y cerrar</button></div></details>`,
);
$("#check-app-update").onclick = async () => {
  const button = $<HTMLButtonElement>("#check-app-update");
  button.disabled = true;
  $("#app-update-state").textContent = "Comprobando…";
  try {
    const s = await invoke<{
      configured: boolean;
      ready: boolean;
      version: string | null;
    }>("check_app_update");
    $("#app-update-state").textContent = !s.configured
      ? "El canal público se activará al publicar la primera versión."
      : s.ready
        ? "Actualización verificada y preparada"
        : "Ya tienes la última versión";
    $("#install-app-update").hidden = !s.ready;
  } catch (e) {
    $("#app-update-state").textContent = String(e);
  } finally {
    button.disabled = false;
  }
};
$("#install-app-update").onclick = () => {
  void invoke("install_app_update").catch((e) => toast(String(e)));
};
