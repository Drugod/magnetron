// @vitest-environment jsdom
import { it, expect, vi } from "vitest";
const api = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(async () => () => {}),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: api.invoke,
  isTauri: () => true,
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: api.listen }));
import { setupDownloads, type Download } from "../src/downloads";
import { defaults } from "../src/model";
it("imports pasted links using the saved destination and separates list removal from confirmed file deletion", async () => {
  vi.spyOn(window, "setInterval").mockReturnValue(0);
  document.body.innerHTML =
    '<nav><button id="nav-search"></button></nav><div class="workspace"><main id="search-view"><form id="search-form"><textarea id="query"></textarea><button id="search-button"></button></form></main></div>';
  const sample: Download = {
    id: "ed2k:test:123",
    name: "Mi archivo.pdf",
    folder: "C:\\Downloads",
    state: "completed",
    completed_bytes: 123,
    total_bytes: 123,
    download_speed: 0,
    upload_speed: 0,
    error: null,
  };
  let rows = [sample];
  let cfg = { ...defaults };
  api.invoke.mockImplementation(
    async (command: string, args?: Record<string, unknown>) => {
      if (command === "list_downloads") return rows;
      if (command === "take_download_links") return [];
      if (command === "get_settings") return cfg;
      if (command === "choose_download_folder") return "C:\\Downloads";
      if (command === "set_download_folder") {
        cfg = { ...cfg, download_folder: args!.folder as string };
        return cfg;
      }
      if (command === "parse_download_links")
        return [
          { uri: "ed2k://test", name: "Archivo" },
          { uri: "magnet:test", name: "Torrent" },
        ];
      if (command === "remove_download") {
        rows = [];
        return;
      }
      return "added";
    },
  );
  const manager = setupDownloads(vi.fn());
  manager.show();
  expect(document.querySelectorAll("#search-view textarea")).toHaveLength(1);
  expect(document.querySelector("#download-links-form")).toBeNull();
  expect(
    document.querySelector("#downloads-view #download-links-form"),
  ).toBeNull();
  expect(document.querySelector("#nav-paste-links")).toBeNull();
  await vi.waitFor(() =>
    expect(document.querySelector("tbody")!.textContent).toContain(
      "Mi archivo.pdf",
    ),
  );
  document.querySelector("tbody tr")!.dispatchEvent(
    new MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
      clientX: 30,
      clientY: 50,
    }),
  );
  const menuButton = (text: string) =>
    Array.from(
      document.querySelectorAll<HTMLButtonElement>('[role="menu"] button'),
    ).find((b) => b.textContent!.includes(text))!;
  menuButton("Eliminar también").click();
  expect(api.invoke.mock.calls.some(([c]) => c === "remove_download")).toBe(
    false,
  );
  const confirmation = document.querySelector<HTMLElement>(
    ".download-delete-confirmation",
  )!;
  expect(confirmation.hidden).toBe(false);
  expect(confirmation.textContent).toContain("carpeta de destino");
  Array.from(confirmation.querySelectorAll("button"))
    .find((b) => b.textContent === "Conservar")!
    .click();
  document
    .querySelector("tbody tr")!
    .dispatchEvent(
      new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
    );
  menuButton("Quitar del listado").click();
  await vi.waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith("remove_download", {
      id: sample.id,
      deleteFiles: false,
    }),
  );
  await manager.addLinks("dos enlaces");
  expect(api.invoke).toHaveBeenCalledWith("add_download", {
    uri: "ed2k://test",
    name: "Archivo",
    folder: "C:\\Downloads",
  });
  expect(api.invoke).toHaveBeenCalledWith("add_download", {
    uri: "magnet:test",
    name: "Torrent",
    folder: "C:\\Downloads",
  });
  expect(
    api.invoke.mock.calls.filter(([c]) => c === "choose_download_folder"),
  ).toHaveLength(1);
  await manager.addLinks("otros enlaces");
  expect(
    api.invoke.mock.calls.filter(([c]) => c === "choose_download_folder"),
  ).toHaveLength(1);
  rows = [{ ...sample, id: "ed2k:second:123" }];
  manager.show();
  await vi.waitFor(() =>
    expect(document.querySelector("tbody")!.textContent).toContain(
      "Mi archivo.pdf",
    ),
  );
  document
    .querySelector("tbody tr")!
    .dispatchEvent(
      new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
    );
  menuButton("Eliminar también").click();
  Array.from(confirmation.querySelectorAll("button"))
    .find((b) => b.textContent === "Eliminar archivos")!
    .click();
  await vi.waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith("remove_download", {
      id: "ed2k:second:123",
      deleteFiles: true,
    }),
  );
  vi.restoreAllMocks();
});

it("imports torrent batches with one first-download folder request, preserves the query and continues after invalid metadata", async () => {
  vi.spyOn(window, "setInterval").mockReturnValue(0);
  document.body.innerHTML =
    '<nav><button id="nav-search"></button></nav><div class="workspace"><main id="search-view"><form id="search-form"><textarea id="query">my search</textarea><button id="search-button"></button></form></main></div>';
  api.invoke.mockClear();
  let destination = "";
  api.invoke.mockImplementation(
    async (command: string, args?: Record<string, unknown>) => {
      if (command === "get_settings")
        return { ...defaults, download_folder: destination };
      if (command === "choose_download_folder") return "C:/Downloads";
      if (command === "set_download_folder") {
        destination = args!.folder as string;
        return { ...defaults, download_folder: destination };
      }
      if (command === "list_downloads" || command === "take_download_links")
        return [];
      if (
        command === "add_torrent_file" &&
        String(args!.path).includes("broken")
      )
        throw "Archivo .torrent inválido";
      return "hash";
    },
  );
  const notify = vi.fn();
  const manager = setupDownloads(notify);
  await manager.addTorrentFiles([
    "C:/a.torrent",
    "C:/a.torrent",
    "C:/broken.torrent",
    "C:/image.jpg",
    "C:/b.TORRENT",
  ]);
  expect(
    api.invoke.mock.calls.filter(([c]) => c === "choose_download_folder"),
  ).toHaveLength(1);
  expect(
    api.invoke.mock.calls.filter(([c]) => c === "add_torrent_file"),
  ).toHaveLength(3);
  expect(api.invoke).toHaveBeenCalledWith("add_torrent_file", {
    path: "C:/b.TORRENT",
    folder: "C:/Downloads",
  });
  expect(document.querySelector<HTMLTextAreaElement>("#query")!.value).toBe(
    "my search",
  );
  expect(document.querySelector<HTMLElement>("#downloads-view")!.hidden).toBe(
    false,
  );
  expect(notify).toHaveBeenCalledWith(
    expect.stringContaining("broken.torrent"),
  );
  await manager.addTorrentFiles(["C:/next.torrent"]);
  expect(
    api.invoke.mock.calls.filter(([c]) => c === "choose_download_folder"),
  ).toHaveLength(1);
  const count = api.invoke.mock.calls.filter(
    ([c]) => c === "add_torrent_file",
  ).length;
  await manager.addTorrentFiles(["C:/photo.jpg"]);
  await manager.addTorrentFiles(
    Array.from({ length: 101 }, (_, i) => `C:/${i}.torrent`),
  );
  expect(
    api.invoke.mock.calls.filter(([c]) => c === "add_torrent_file"),
  ).toHaveLength(count);
  vi.restoreAllMocks();
});

it("does not queue a torrent when the destination chooser is cancelled", async () => {
  vi.spyOn(window, "setInterval").mockReturnValue(0);
  document.body.innerHTML =
    '<nav><button id="nav-search"></button></nav><div class="workspace"><main id="search-view"><form id="search-form"><textarea id="query"></textarea><button id="search-button"></button></form></main></div>';
  api.invoke.mockClear();
  api.invoke.mockImplementation(async (command: string) => {
    if (command === "get_settings") return { ...defaults, download_folder: "" };
    if (command === "choose_download_folder") return null;
    return [];
  });
  const manager = setupDownloads(vi.fn());
  await manager.addTorrentFiles(["C:/a.torrent"]);
  expect(api.invoke.mock.calls.some(([c]) => c === "add_torrent_file")).toBe(
    false,
  );
  expect(
    document.querySelector<HTMLButtonElement>("#search-button")!.disabled,
  ).toBe(false);
  vi.restoreAllMocks();
});
