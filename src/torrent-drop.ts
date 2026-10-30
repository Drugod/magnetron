import { getCurrentWebview, type DragDropEvent } from "@tauri-apps/api/webview";

/** Native positions are physical pixels; DOM bounds are CSS pixels (including display scaling). */
export function torrentDropHandler(
  bar: HTMLElement,
  add: (paths: string[]) => Promise<void>,
  notify: (message: string) => void,
) {
  return (event: DragDropEvent) => {
    bar.classList.remove("torrent-drop-target");
    if (
      event.type === "leave" ||
      bar.closest<HTMLElement>("[hidden]") ||
      document.querySelector("dialog[open]")
    )
      return;
    const rect = bar.getBoundingClientRect();
    const x = event.position.x / window.devicePixelRatio;
    const y = event.position.y / window.devicePixelRatio;
    if (
      !rect.width ||
      !rect.height ||
      x < rect.left ||
      x > rect.right ||
      y < rect.top ||
      y > rect.bottom
    )
      return;
    if (
      event.type === "enter" &&
      !event.paths.some((path) => /\.torrent$/i.test(path))
    )
      return;
    if (event.type !== "drop") {
      bar.classList.add("torrent-drop-target");
      return;
    }
    void add([...new Set(event.paths)]).catch((error) => notify(String(error)));
  };
}
export async function setupTorrentDrop(
  add: (paths: string[]) => Promise<void>,
  notify: (message: string) => void,
) {
  const bar = document.querySelector<HTMLElement>("#search-form")!;
  const handle = torrentDropHandler(bar, add, notify);
  document.addEventListener("magnet:view", () =>
    bar.classList.remove("torrent-drop-target"),
  );
  return getCurrentWebview().onDragDropEvent(({ payload }) => handle(payload));
}
