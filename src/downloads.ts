import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { bytes, type Torrent, type Settings } from "./model";
import { showView } from "./views";
import { getLanguage } from "./i18n";
export interface Download {
  id: string;
  name: string;
  folder: string;
  state: string;
  completed_bytes: number;
  total_bytes: number;
  download_speed: number;
  upload_speed: number;
  error: string | null;
}
const labels: Record<string, string> = {
  resolving: "Buscando fuentes",
  checking: "Comprobando",
  downloading: "Descargando",
  waiting: "Esperando fuentes o turno",
  finalizing: "Verificando archivo",
  paused: "Pausada",
  cancelled: "Cancelada",
  completed: "Completada",
  error: "Error",
};
function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  text = "",
  className = "",
) {
  const n = document.createElement(tag);
  n.textContent = text;
  n.className = className;
  return n;
}
export function remaining(d: Download): string {
  if (
    d.state === "completed" ||
    d.download_speed <= 0 ||
    d.total_bytes <= d.completed_bytes
  )
    return "—";
  const s = Math.ceil((d.total_bytes - d.completed_bytes) / d.download_speed);
  if (s < 60) return `${s} s`;
  if (s < 3600) return `${Math.ceil(s / 60)} min`;
  return `${Math.floor(s / 3600)} h ${Math.ceil((s % 3600) / 60)} min`;
}
export function setupDownloads(notify: (s: string) => void) {
  document
    .querySelector("#nav-search")!
    .insertAdjacentHTML(
      "afterend",
      `<button class="nav" id="nav-downloads"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5"/></svg>Descargas</button>`,
    );
  document
    .querySelector(".workspace")!
    .insertAdjacentHTML(
      "beforeend",
      `<main id="downloads-view" class="downloads-view" hidden><div class="downloads-heading"><h1>Descargas</h1><span id="downloads-summary" class="muted"></span></div><div class="downloads-table-wrap"><table class="downloads-table"><caption class="sr-only">Descargas BitTorrent y eD2k</caption><thead><tr><th scope="col">Nombre</th><th scope="col">Red</th><th scope="col">Tamaño</th><th scope="col">Progreso</th><th scope="col">Estado</th><th scope="col">↓ Bajada</th><th scope="col">↑ Subida</th><th scope="col">Tiempo restante</th><th scope="col">Acciones</th></tr></thead><tbody id="downloads-list"></tbody></table></div><p id="downloads-feedback" role="status"></p><p class="muted downloads-note">Los archivos se guardan en la carpeta elegida. Durante la descarga se intercambian piezas. Al completar se liberan los archivos; cancelar conserva los parciales.</p></main>`,
    );
  const panel = document.querySelector<HTMLElement>("#downloads-view")!,
    body = document.querySelector<HTMLTableSectionElement>("#downloads-list")!,
    feedback = document.querySelector<HTMLElement>("#downloads-feedback")!;
  document
    .querySelector("#search-form")!
    .insertAdjacentHTML(
      "afterend",
      `<p id="download-links-status" role="status"></p>`,
    );
  const linkInput = document.querySelector<HTMLTextAreaElement>("#query")!;
  const linkStatus = document.querySelector<HTMLElement>(
    "#download-links-status",
  )!;
  const linkButton =
    document.querySelector<HTMLButtonElement>("#search-button")!;
  const menu = el("div", "", "download-context-menu");
  menu.hidden = true;
  menu.setAttribute("role", "menu");
  document.body.append(menu);
  const confirmation = el("section", "", "download-delete-confirmation");
  confirmation.hidden = true;
  confirmation.setAttribute("role", "alert");
  panel.prepend(confirmation);
  let menuInvoker: HTMLElement | null = null;
  function context(d: Download, x: number, y: number) {
    menuInvoker = document.activeElement as HTMLElement | null;
    menu.replaceChildren();
    menu.hidden = false;
    const button = (label: string, run: () => void) => {
      const b = el("button", label);
      b.setAttribute("role", "menuitem");
      b.onclick = () => {
        menu.hidden = true;
        run();
      };
      menu.append(b);
    };
    button("Abrir carpeta", () => {
      void invoke("open_download_folder", { id: d.id }).catch((e) =>
        notify(String(e)),
      );
    });
    const remove = async (deleteFiles: boolean) => {
      confirmation.hidden = true;
      try {
        await invoke("remove_download", { id: d.id, deleteFiles });
        await refresh();
        notify(
          deleteFiles
            ? "Descarga y archivos eliminados"
            : "Descarga quitada del listado. Archivos conservados.",
        );
      } catch (e) {
        notify(String(e));
      }
    };
    button("Quitar del listado (conservar archivos)", () => {
      void remove(false);
    });
    button("Eliminar también los archivos…", () => {
      confirmation.replaceChildren(
        el("strong", `¿Eliminar «${d.name}» y sus archivos?`),
        el(
          "p",
          "Se eliminarán definitivamente los archivos de esta descarga y sus subcarpetas vacías. Se conservarán la carpeta de destino y los demás archivos.",
        ),
      );
      const yes = el("button", "Eliminar archivos", "danger"),
        no = el("button", "Conservar", "text-button");
      yes.onclick = () => {
        yes.disabled = true;
        void remove(true);
      };
      no.onclick = () => {
        confirmation.hidden = true;
      };
      confirmation.append(yes, no);
      confirmation.hidden = false;
      no.focus();
    });
    menu.style.left = `${Math.max(8, Math.min(x, window.innerWidth - menu.offsetWidth - 8))}px`;
    menu.style.top = `${Math.max(8, Math.min(y, window.innerHeight - menu.offsetHeight - 8))}px`;
    menu.querySelector<HTMLButtonElement>("button")!.focus();
  }
  document.addEventListener("pointerdown", (e) => {
    if (!menu.contains(e.target as Node)) menu.hidden = true;
  });
  menu.onkeydown = (e) => {
    const items = [...menu.querySelectorAll<HTMLButtonElement>("button")];
    const at = items.indexOf(document.activeElement as HTMLButtonElement);
    let next: number | undefined;
    if (e.key === "ArrowDown") next = (at + 1) % items.length;
    if (e.key === "ArrowUp") next = (at - 1 + items.length) % items.length;
    if (e.key === "Home") next = 0;
    if (e.key === "End") next = items.length - 1;
    if (next !== undefined) {
      e.preventDefault();
      items[next].focus();
    }
  };
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && (!menu.hidden || !confirmation.hidden)) {
      menu.hidden = true;
      confirmation.hidden = true;
      menuInvoker?.focus();
    }
  });
  const rows = new Map<
    string,
    {
      row: HTMLTableRowElement;
      cells: HTMLTableCellElement[];
      progress: HTMLProgressElement;
      percent: HTMLElement;
      state: string;
    }
  >();
  let busy = false;
  async function refresh() {
    if (panel.hidden || !isTauri() || busy) return;
    busy = true;
    try {
      const items = await invoke<Download[]>("list_downloads");
      feedback.textContent = items.length
        ? ""
        : "Todavía no hay descargas. Busca un archivo y pulsa Descargar.";
      document.querySelector("#downloads-summary")!.textContent =
        `${items.length} archivos · ↓ ${bytes(items.reduce((n, d) => n + d.download_speed, 0))}/s · ↑ ${bytes(items.reduce((n, d) => n + d.upload_speed, 0))}/s`;
      const ids = new Set(items.map((d) => d.id));
      for (const [id, r] of rows)
        if (!ids.has(id)) {
          r.row.remove();
          rows.delete(id);
        }
      for (const d of items) {
        let r = rows.get(d.id);
        if (!r) {
          const row = el("tr"),
            cells = Array.from({ length: 9 }, () => el("td"));
          row.append(...cells);
          const progress = el("progress");
          progress.max = 1;
          progress.setAttribute("aria-label", `Progreso de ${d.name}`);
          const percent = el("span");
          cells[3].append(progress, percent);
          body.append(row);
          r = { row, cells, progress, percent, state: "" };
          rows.set(d.id, r);
        }
        const c = r.cells;
        c[0].textContent = d.name;
        c[0].dataset.noI18n = "";
        r.row.tabIndex = 0;
        r.row.oncontextmenu = (e) => {
          e.preventDefault();
          context(d, e.clientX, e.clientY);
        };
        r.row.onkeydown = (e) => {
          if (e.key === "ContextMenu" || (e.shiftKey && e.key === "F10")) {
            e.preventDefault();
            const rect = r!.row.getBoundingClientRect();
            context(d, rect.left + 20, rect.top + 20);
          }
        };
        c[0].title = d.folder;
        c[1].textContent = d.id.startsWith("youtube:") ? "YouTube" : d.id.startsWith("ed2k:")
          ? "eD2k"
          : d.id.startsWith("bt2:")
            ? "BitTorrent v2"
            : "BitTorrent";
        c[2].textContent = d.total_bytes ? bytes(d.total_bytes) : "—";
        if (d.total_bytes) {
          r.progress.value = Math.min(1, d.completed_bytes / d.total_bytes);
          r.percent.textContent = `${(100 * r.progress.value).toFixed(1)} %`;
        } else {
          r.progress.removeAttribute("value");
          r.percent.textContent = "—";
        }
        c[4].textContent = labels[d.state] ?? d.state;
        c[4].title = d.error ?? "";
        r.row.classList.toggle("download-error", d.state === "error");
        c[5].textContent = `${bytes(d.download_speed)}/s`;
        c[6].textContent = `${bytes(d.upload_speed)}/s`;
        c[7].textContent = remaining(d);
        if (r.state !== d.state) {
          r.state = d.state;
          c[8].replaceChildren();
          const action = (label: string, op: string) => {
            const b = el("button", label, "text-button");
            b.onclick = async () => {
              b.disabled = true;
              try {
                await invoke("download_action", { id: d.id, action: op });
                await refresh();
              } catch (e) {
                notify(String(e));
              } finally {
                b.disabled = false;
              }
            };
            c[8].append(b);
          };
          if (
            ["downloading", "waiting", "resolving", "checking"].includes(
              d.state,
            )
          )
            action("Pausar", "pause");
          if (["paused", "error", "cancelled"].includes(d.state))
            action(d.state === "error" ? "Reintentar" : "Reanudar", "resume");
          if (!["completed", "cancelled"].includes(d.state))
            action("Cancelar", "cancel");
          const folder = el("button", "Carpeta", "text-button");
          folder.onclick = () =>
            void invoke("open_download_folder", { id: d.id }).catch((e) =>
              notify(String(e)),
            );
          c[8].append(folder);
          const more = el("button", "⋯", "text-button");
          more.setAttribute("aria-label", `Acciones de ${d.name}`);
          more.onclick = () => {
            const rect = more.getBoundingClientRect();
            context(d, rect.left, rect.bottom);
          };
          c[8].append(more);
        }
      }
    } catch (e) {
      feedback.textContent = String(e);
    } finally {
      busy = false;
    }
  }
  function show() {
    showView("downloads");
    void refresh();
  }
  document.querySelector<HTMLButtonElement>("#nav-downloads")!.onclick = show;
  document.addEventListener("magnet:view", () => void refresh());
  window.setInterval(() => void refresh(), 1500);
  let folderRequest: Promise<string | null> | null = null;
  async function folder() {
    const cfg = await invoke<Settings>("get_settings");
    if (cfg.download_folder) return cfg.download_folder;
    if (!folderRequest)
      folderRequest = (async () => {
        const path = await invoke<string | null>("choose_download_folder", {
          language: getLanguage(),
        });
        if (!path) return null;
        const next = await invoke<Settings>("set_download_folder", {
          folder: path,
        });
        document.dispatchEvent(
          new CustomEvent("magnet:settings", { detail: next }),
        );
        return path;
      })().finally(() => {
        folderRequest = null;
      });
    return folderRequest;
  }
  let imports = Promise.resolve();
  function addLinks(text: string): Promise<void> {
    const task = imports
      .catch(() => {})
      .then(async () => {
        if (!isTauri()) {
          notify("Disponible en MAGNETRON de escritorio");
          return;
        }
        showView("search");
        linkInput.value = text;
        linkButton.disabled = true;
        try {
          const links = await invoke<{ uri: string; name: string }[]>(
            "parse_download_links",
            { text },
          );
          const selected = await folder();
          if (!selected) {
            linkStatus.textContent =
              "Elige una carpeta para iniciar las descargas.";
            return;
          }
          let added = 0;
          const failed: string[] = [];
          for (const link of links) {
            try {
              await invoke("add_download", { ...link, folder: selected });
              added++;
            } catch (e) {
              failed.push(link.uri);
              linkStatus.textContent = String(e);
            }
          }
          linkInput.value = failed.join("\n");
          if (!failed.length)
            linkStatus.textContent = `${added} ${added === 1 ? "enlace añadido" : "enlaces añadidos"} a Descargas.`;
          else
            linkStatus.textContent = `${added} añadidos; ${failed.length} pendientes. ${linkStatus.textContent}`;
          if (added) {
            show();
            notify(linkStatus.textContent!);
          }
          await refresh();
        } catch (e) {
          linkStatus.textContent = String(e);
        } finally {
          linkButton.disabled = false;
        }
      });
    imports = task;
    return task;
  }
  if (isTauri()) {
    let draining = Promise.resolve();
    const drain = () => {
      draining = draining
        .catch(() => {})
        .then(async () => {
          const links = await invoke<string[]>("take_download_links");
          if (links.length) await addLinks(links.join("\n"));
        })
        .catch((e) => notify(String(e)));
    };
    void listen("download-links-ready", drain)
      .then(drain)
      .catch((e) => notify(String(e)));
  }
  return {
    show,
    addLinks,
    async download(t: Torrent) {
      if (!isTauri()) return notify("Disponible en MAGNETRON de escritorio");
      const current = await invoke<Download[]>("list_downloads");
      if (current.some((d) => d.id === t.id)) {
        show();
        return;
      }
      const selected = await folder();
      if (!selected) return;
      show();
      feedback.textContent = "Preparando descarga…";
      await invoke("add_download", {
        uri: t.ed2k_uri ?? t.magnet_uri,
        name: t.title,
        folder: selected,
      });
      await refresh();
    },
  };
}
