import { invoke, isTauri } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import packageInfo from "../package.json";
import { translate } from "./i18n";
import { showView } from "./views";

type Status = { configured: boolean; ready: boolean; version: string | null };
export function setupAppUpdates() {
  const form = document.getElementById("settings-form")!;
  const panel = document.createElement("details");
  panel.id = "app-updates";
  panel.className = "advanced-settings";
  panel.innerHTML = `<summary><span>Actualizaciones de MAGNETRON</span> · <span id="installed-app-version" data-no-i18n>v${packageInfo.version}</span></summary><p id="app-update-state" class="muted" role="status">Las actualizaciones se comprueban automáticamente.</p><div class="dialog-actions"><button type="button" id="check-app-update" class="text-button">Comprobar actualizaciones</button><button type="button" id="install-app-update" class="text-button" hidden>Instalar actualización y cerrar</button></div>`;
  form.insertBefore(panel, document.getElementById("settings-actions"));
  document.body.insertAdjacentHTML(
    "beforeend",
    `<button id="update-available" class="update-available" hidden>Actualización disponible</button><dialog id="app-update-dialog" aria-labelledby="app-update-title" aria-describedby="app-update-description"><h2 id="app-update-title">Actualización disponible</h2><p><span>Hay una nueva versión de MAGNETRON:</span> <strong id="new-app-version" data-no-i18n></strong></p><p id="app-update-description">Puedes actualizar ahora o continuar y hacerlo más tarde. La instalación cerrará MAGNETRON.</p><p id="update-prompt-state" role="status"></p><div class="dialog-actions"><button type="button" id="update-later" class="text-button" autofocus>Más tarde</button><button type="button" id="update-now" class="primary">Actualizar ahora</button></div></dialog>`,
  );
  const el = <T extends HTMLElement = HTMLElement>(id: string) =>
    document.getElementById(id) as T;
  const dialog = el<HTMLDialogElement>("app-update-dialog");
  const checkButton = el<HTMLButtonElement>("check-app-update");
  const nowButton = el<HTMLButtonElement>("update-now");
  const laterButton = el<HTMLButtonElement>("update-later");
  const installButton = el<HTMLButtonElement>("install-app-update");
  const prompted = new Set<string>();
  let status: Status | null = null;
  let requestedVersion: string | null = null;
  let installing = false;
  let checking = false;
  const setText = (id: string, text: string) => {
    el(id).textContent = translate(text);
  };
  function showSettings() {
    showView("settings");
    panel.open = true;
    panel.scrollIntoView({ block: "center" });
  }
  async function install() {
    if (!status?.ready || requestedVersion !== status.version || installing)
      return;
    installing = true;
    nowButton.disabled = true;
    laterButton.disabled = true;
    installButton.disabled = true;
    setText("update-prompt-state", "Instalando actualización…");
    try {
      await invoke("install_app_update", { expectedVersion: requestedVersion });
    } catch (error) {
      installing = false;
      requestedVersion = null;
      nowButton.disabled = false;
      laterButton.disabled = false;
      installButton.disabled = false;
      setText("update-prompt-state", String(error));
      setText("app-update-state", String(error));
    }
  }
  function display(next: Status) {
    if (
      !next ||
      typeof next.configured !== "boolean" ||
      typeof next.ready !== "boolean" ||
      !(
        next.version === null ||
        (typeof next.version === "string" &&
          /^\d+\.\d+\.\d+$/.test(next.version))
      )
    )
      return;
    if (next.ready && (!next.configured || !next.version)) return;
    if (next.version && status?.version) {
      const before = status.version.split(".").map(Number);
      const after = next.version.split(".").map(Number);
      const part = after.findIndex((n, i) => n !== before[i]);
      if (part >= 0 && after[part] < before[part]) return;
      if (part < 0 && status.ready && !next.ready) return;
    }
    status = next;
    const available = next.configured && !!next.version;
    setText(
      "app-update-state",
      !next.configured
        ? "En este sistema, descarga las nuevas versiones desde GitHub."
        : next.ready
          ? "Actualización verificada y preparada"
          : available
            ? "Preparando actualización…"
            : "Ya tienes la última versión",
    );
    installButton.hidden = !next.ready;
    el("update-available").hidden = !available;
    if (available && !prompted.has(next.version!)) {
      prompted.add(next.version!);
      requestedVersion = null;
      el("new-app-version").textContent = `v${next.version}`;
      setText("update-prompt-state", "");
      nowButton.disabled = false;
      if (!dialog.open) dialog.showModal();
    }
    if (next.ready && requestedVersion === next.version) void install();
  }
  function later() {
    if (installing) return;
    requestedVersion = null;
    dialog.close();
  }
  laterButton.onclick = later;
  dialog.addEventListener("cancel", (event) => {
    if (installing) event.preventDefault();
    else requestedVersion = null;
  });
  nowButton.onclick = () => {
    if (!status?.version || installing) return;
    requestedVersion = status.version;
    nowButton.disabled = true;
    if (status.ready) void install();
    else setText("update-prompt-state", "Preparando actualización…");
  };
  installButton.onclick = () => {
    if (!status?.ready) return;
    requestedVersion = status.version;
    void install();
  };
  el("update-available").onclick = showSettings;
  async function check(manual: boolean) {
    if (checking) return;
    checking = true;
    checkButton.disabled = true;
    setText("app-update-state", "Comprobando…");
    try {
      display(await invoke<Status>("check_app_update"));
    } catch (error) {
      requestedVersion = null;
      nowButton.disabled = false;
      setText("app-update-state", String(error));
      if (dialog.open) setText("update-prompt-state", String(error));
      if (manual) panel.open = true;
    } finally {
      checking = false;
      checkButton.disabled = false;
    }
  }
  checkButton.onclick = () => void check(true);
  if (isTauri()) {
    void getVersion()
      .then((v) => {
        el("installed-app-version").textContent = `v${v}`;
      })
      .catch(console.error);
    void (async () => {
      // Register before checking: the signed availability event can precede download completion.
      await Promise.all([
        listen<Status>("app-update-available", (e) => display(e.payload)),
        listen<Status>("app-update-ready", (e) => display(e.payload)),
        listen<string>("app-update-install-error", (e) => {
          installing = false;
          requestedVersion = null;
          nowButton.disabled = false;
          laterButton.disabled = false;
          installButton.disabled = false;
          setText("app-update-state", e.payload);
          setText("update-prompt-state", e.payload);
        }),
      ]);
      await check(false);
    })().catch((error) => setText("app-update-state", String(error)));
  }
}
