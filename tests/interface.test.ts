// @vitest-environment jsdom
import { it, expect, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => false,
  invoke: vi.fn(async () => []),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));
it("keeps one search/link field, places sources first in settings and switches language and theme without reload", async () => {
  vi.spyOn(window, "setInterval").mockReturnValue(0);
  document.body.innerHTML = '<div id="app"></div>';
  await import("../src/main");
  const { setLanguage, setAppearance } = await import("../src/i18n");
  expect(document.querySelectorAll("#search-view textarea")).toHaveLength(1);
  expect(document.querySelector("#download-links-form")).toBeNull();
  expect(document.querySelector(".sidebar #nav-sources")).toBeNull();
  expect(
    document.querySelector("#settings-dialog #sources-settings"),
  ).not.toBeNull();
  expect(document.querySelector("#settings-dialog")!.children[1].id).toBe(
    "sources-settings",
  );
  expect(document.querySelector("#nav-settings")!.textContent).toBe("");
  expect(document.querySelector("#nav-settings svg")).not.toBeNull();
  document.querySelector<HTMLButtonElement>("#nav-settings")!.click();
  expect(
    (document.querySelector("#settings-dialog") as HTMLElement).hidden,
  ).toBe(false);
  setLanguage("en");
  await Promise.resolve();
  expect(document.documentElement.lang).toBe("en");
  expect(document.querySelector("#query")!.getAttribute("placeholder")).toBe(
    "Search or paste links…",
  );
  expect(
    document.querySelector("#nav-settings")!.getAttribute("aria-label"),
  ).toBe("Settings");
  expect(document.querySelector("#settings-form")!.textContent).toContain(
    "Download folder",
  );
  expect(document.querySelector("#settings-form")!.textContent).toContain(
    "Language",
  );
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  const untranslated: string[] = [];
  while (walker.nextNode()) {
    const node = walker.currentNode;
    if (
      !node.parentElement?.closest("[data-no-i18n],script,style") &&
      /[áéíóúñ¿]/i.test(node.textContent ?? "")
    )
      untranslated.push(node.textContent!.trim());
  }
  expect(untranslated).toEqual([]);
  const filename = document.createElement("span");
  filename.dataset.noI18n = "";
  filename.textContent = "Descargas";
  document.body.append(filename);
  setAppearance("dark");
  expect(document.documentElement.dataset.theme).toBe("dark");
  expect(localStorage.getItem("magnetron.appearance")).toBe("dark");
  setLanguage("es");
  await Promise.resolve();
  expect(
    document.querySelector("#nav-settings")!.getAttribute("aria-label"),
  ).toBe("Ajustes");
  expect(document.querySelector("#settings-form")!.textContent).toContain(
    "Carpeta de descarga",
  );
  expect(filename.textContent).toBe("Descargas");
  setAppearance("light");
  expect(document.documentElement.dataset.theme).toBe("light");
  vi.restoreAllMocks();
});
