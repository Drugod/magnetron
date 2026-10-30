// @vitest-environment jsdom
import { expect, it, vi } from "vitest";
import { torrentDropHandler } from "../src/torrent-drop";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
it("accepts drops only on the visible search bar, handles display scaling and clears hover", async () => {
  document.body.innerHTML = '<main><form id="search-form"></form></main>';
  const bar = document.querySelector<HTMLElement>("form")!;
  vi.spyOn(bar, "getBoundingClientRect").mockReturnValue({
    left: 100,
    top: 50,
    right: 500,
    bottom: 110,
    width: 400,
    height: 60,
  } as DOMRect);
  vi.stubGlobal("devicePixelRatio", 2);
  const add = vi.fn(async () => {}),
    notify = vi.fn();
  const handle = torrentDropHandler(bar, add, notify);
  const inside = new PhysicalPosition(300, 150);
  handle({ type: "enter", paths: ["C:/a.torrent"], position: inside });
  expect(bar.classList.contains("torrent-drop-target")).toBe(true);
  handle({
    type: "drop",
    paths: ["C:/a.torrent", "C:/a.torrent", "C:/b.TORRENT"],
    position: inside,
  });
  expect(add).toHaveBeenCalledExactlyOnceWith(["C:/a.torrent", "C:/b.TORRENT"]);
  expect(bar.classList.contains("torrent-drop-target")).toBe(false);
  handle({
    type: "drop",
    paths: ["C:/outside.torrent"],
    position: new PhysicalPosition(80, 150),
  });
  document.querySelector("main")!.hidden = true;
  handle({ type: "drop", paths: ["C:/hidden.torrent"], position: inside });
  expect(add).toHaveBeenCalledTimes(1);
  document.querySelector("main")!.hidden = false;
  handle({ type: "enter", paths: ["C:/photo.jpg"], position: inside });
  expect(bar.classList.contains("torrent-drop-target")).toBe(false);
  handle({ type: "over", position: inside });
  handle({ type: "leave" });
  expect(bar.classList.contains("torrent-drop-target")).toBe(false);
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
