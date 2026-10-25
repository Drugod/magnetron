export type View = "search" | "downloads" | "settings" | "sources";
const ids: Record<View, string> = {
  search: "search-view",
  downloads: "downloads-view",
  settings: "settings-dialog",
  sources: "sources-dialog",
};
export function showView(view: View) {
  if (view === "sources") view = "settings";
  for (const [name, id] of Object.entries(ids)) {
    if (name === "sources") continue;
    const panel = document.getElementById(id);
    if (panel) panel.hidden = name !== view;
  }
  for (const name of ["search", "downloads", "settings", "sources"]) {
    const nav = document.getElementById(`nav-${name}`);
    nav?.classList.toggle("active", name === view);
    if (name === view) nav?.setAttribute("aria-current", "page");
    else nav?.removeAttribute("aria-current");
  }
  document.dispatchEvent(new CustomEvent("magnet:view", { detail: view }));
}
export function setupViews() {
  const workspace = document.querySelector(".workspace")!;
  workspace.querySelector("main")!.id = "search-view";
  for (const id of ["settings-dialog", "sources-dialog"]) {
    const old = document.getElementById(id);
    if (!old) continue;
    const panel = document.createElement(
      id === "sources-dialog" ? "section" : "main",
    );
    panel.id = id;
    panel.className = "integrated-panel";
    panel.hidden = true;
    while (old.firstChild) panel.append(old.firstChild);
    old.remove();
    if (id === "sources-dialog") {
      panel.hidden = false;
      panel.querySelector(".dialog-heading")?.remove();
      const details = document.createElement("details");
      details.id = "sources-settings";
      details.innerHTML =
        '<summary id="nav-sources"><span>Fuentes</span><span class="muted">Automáticas</span></summary>';
      details.append(panel);
      const settings = document.getElementById("settings-dialog")!;
      settings.insertBefore(details, settings.querySelector("form"));
    } else workspace.append(panel);
    panel.querySelectorAll<HTMLButtonElement>(".close-dialog").forEach((b) => {
      b.textContent = "Volver a buscar";
      b.className = "text-button close-dialog";
    });
  }
  document
    .getElementById("nav-search")!
    .addEventListener("click", () => showView("search"));
  document
    .querySelector(".brand")!
    .addEventListener("click", () => showView("search"));
}
