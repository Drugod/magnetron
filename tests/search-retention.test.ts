// @vitest-environment jsdom
import { expect, it, vi } from "vitest";
import { defaults, type SearchEvent, type Torrent } from "../src/model";
const api = vi.hoisted(() => ({
  invoke: vi.fn(),
  events: new Map<string, (event: { payload: SearchEvent }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: api.invoke,
}));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: async () => "0.9.1" }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (
    name: string,
    handler: (event: { payload: SearchEvent }) => void,
  ) => {
    api.events.set(name, handler);
    return () => api.events.delete(name);
  },
}));

it("keeps every admitted result while time passes and new batches arrive, until clearing or starting a different search", async () => {
  const refresh: (() => void)[] = [];
  let now = Date.parse("2026-11-05T12:00:00Z");
  vi.spyOn(Date, "now").mockImplementation(() => now);
  vi.spyOn(window, "setInterval").mockImplementation((callback, delay) => {
    if (delay === 15000) refresh.push(callback as () => void);
    return 0;
  });
  let failSearch = false;
  api.invoke.mockImplementation(async (command: string) => {
    if (command === "start_search" && failSearch)
      throw "Search backend unavailable";
    if (command === "get_settings") return { ...defaults };
    return [];
  });
  document.body.innerHTML = '<div id="app"></div>';
  try {
    await import("../src/main");
    const query = document.querySelector<HTMLTextAreaElement>("#query")!;
    const form = document.querySelector("#search-form")!;
    const busy = (active: boolean) => {
      expect(form.getAttribute("aria-busy")).toBe(String(active));
      for (const id of ["search-spinner", "search-button-spinner"]) {
        const spinner = document.getElementById(id)!;
        expect(spinner.hidden).toBe(!active);
        expect(spinner.getAttribute("aria-hidden")).toBe("true");
      }
      expect(document.getElementById("search-arrow")!.hidden).toBe(active);
      expect(
        document.querySelector<HTMLButtonElement>("#search-button")!.disabled,
      ).toBe(false);
    };
    expect(document.getElementById("search-state")!.getAttribute("role")).toBe(
      "status",
    );
    busy(false);
    query.value = "film";
    form.dispatchEvent(new Event("submit", { cancelable: true }));
    await vi.waitFor(() =>
      expect(api.invoke).toHaveBeenCalledWith(
        "start_search",
        expect.objectContaining({ query: "film" }),
      ),
    );
    const searchId = () =>
      api.invoke.mock.calls.filter(([cmd]) => cmd === "start_search").at(-1)![1]
        .searchId as string;
    const firstId = searchId();
    busy(true);
    const event = (kind: string, results: Torrent[] = [], id = firstId) =>
      api.events.get("search-event")!({
        payload: { search_id: id, kind, results, source: null, message: null },
      });
    const item = (id: string, title: string): Torrent => ({
      id,
      title,
      infohash_v1: id,
      magnet_uri: "magnet:",
      size_bytes: 1000,
      published_at: null,
      seeds_reported: null,
      availability_status: "verified",
      availability_checked_at: new Date(now).toISOString(),
      sources: ["source"],
      files: [],
      discovered_at: new Date(now).toISOString(),
    });
    const rows = () =>
      [...document.querySelectorAll("#results > .result h3")].map(
        (n) => n.textContent,
      );
    event("checking");
    busy(true);
    expect(document.getElementById("search-state-text")!.textContent).toContain(
      "Comprobando",
    );
    const first = item("first", "Film One.mkv");
    event("batch", [first]);
    expect(rows()).toEqual(["Film One.mkv"]);
    now += 6 * 60_000;
    refresh.forEach((tick) => tick());
    expect(rows()).toEqual(["Film One.mkv"]);
    const second = {
      ...item("second", "Film Two.mp4"),
      availability_status: "verified_v2",
    };
    const ed = {
      ...item("ed2k:third:1000", "Book.pdf"),
      network: "ed2k" as const,
      availability_status: "reported_complete",
      complete_sources_reported: 1,
    };
    event("batch", [
      second,
      ed,
      { ...item("unchecked", "Hidden.mkv"), availability_status: "unknown" },
    ]);
    event("batch", []);
    event("batch", [
      {
        ...first,
        title: "Unclassified duplicate",
        availability_status: "unknown",
        availability_checked_at: null,
        sources: ["other"],
      },
    ]);
    expect(rows()).toHaveLength(3);
    expect(rows()).toContain("Film One.mkv");
    busy(true);
    event("done");
    busy(false);
    now += 24 * 60 * 60_000;
    refresh.forEach((tick) => tick());
    expect(rows()).toHaveLength(3);
    const type = document.querySelector<HTMLSelectElement>("#type-filter")!;
    type.value = "video";
    type.dispatchEvent(new Event("change"));
    expect(rows()).toHaveLength(2);
    type.value = "all";
    type.dispatchEvent(new Event("change"));
    expect(rows()).toHaveLength(3);
    const sort = document.querySelector<HTMLSelectElement>("#sort")!;
    sort.value = "name";
    sort.dispatchEvent(new Event("change"));
    expect(rows()).toHaveLength(3);
    const { setLanguage } = await import("../src/i18n");
    setLanguage("en");
    expect(rows()).toHaveLength(3);
    document.querySelector<HTMLButtonElement>("#export-results")!.click();
    await vi.waitFor(() =>
      expect(api.invoke).toHaveBeenCalledWith(
        "export_results",
        expect.objectContaining({
          items: expect.arrayContaining([
            expect.objectContaining({ id: "first" }),
            expect.objectContaining({ id: "second" }),
            expect.objectContaining({ id: ed.id }),
          ]),
        }),
      ),
    );
    document.querySelector<HTMLButtonElement>("#clear-search")!.click();
    expect(rows()).toHaveLength(0);
    event("batch", [first]);
    expect(rows()).toHaveLength(0);
    query.value = "another film";
    form.dispatchEvent(new Event("submit", { cancelable: true }));
    await vi.waitFor(() =>
      expect(api.invoke).toHaveBeenCalledWith(
        "start_search",
        expect.objectContaining({ query: "another film" }),
      ),
    );
    event("batch", [item("late", "Old search.mkv")]);
    expect(rows()).toHaveLength(0);
    event("batch", [item("new", "New search.mkv")], searchId());
    expect(rows()).toEqual(["New search.mkv"]);
    busy(true);
    event("done"); // A late completion from the old query cannot stop the new spinner.
    busy(true);
    query.value = "third search";
    form.dispatchEvent(new Event("submit", { cancelable: true }));
    expect(rows()).toHaveLength(0);
    for (const kind of ["limited", "cancelled", "error", "done"]) {
      const text = `terminal ${kind}`;
      query.value = text;
      form.dispatchEvent(new Event("submit", { cancelable: true }));
      await vi.waitFor(() =>
        expect(api.invoke).toHaveBeenCalledWith(
          "start_search",
          expect.objectContaining({ query: text }),
        ),
      );
      busy(true);
      event(kind, [], searchId());
      busy(false);
    }
    failSearch = true;
    query.value = "backend failure";
    form.dispatchEvent(new Event("submit", { cancelable: true }));
    await vi.waitFor(() =>
      expect(api.invoke).toHaveBeenCalledWith(
        "start_search",
        expect.objectContaining({ query: "backend failure" }),
      ),
    );
    await vi.waitFor(() => busy(false));
    setLanguage("es");
  } finally {
    vi.restoreAllMocks();
  }
});
