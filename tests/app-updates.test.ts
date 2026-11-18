// @vitest-environment jsdom
import { beforeEach, expect, it, vi } from "vitest";
const api = vi.hoisted(() => ({
  invoke: vi.fn(),
  events: new Map<string, (e: { payload: any }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: api.invoke,
}));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: async () => "0.9.3" }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, fn: any) => {
    api.events.set(name, fn);
    return () => {};
  },
}));
import { setupAppUpdates } from "../src/app-updates";
const latest = { configured: true, ready: false, version: null };
const available = { configured: true, ready: false, version: "0.10.0" };
const ready = { ...available, ready: true };
const el = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;
const send = (name: string, status: any) =>
  api.events.get(name)!({ payload: status });
const installs = () =>
  api.invoke.mock.calls.filter(([cmd]) => cmd === "install_app_update");
beforeEach(() => {
  api.events.clear();
  api.invoke.mockReset();
  api.invoke.mockImplementation(async (cmd: string) =>
    cmd === "check_app_update" ? latest : undefined,
  );
  document.body.innerHTML =
    '<main id="search-view"></main><main id="settings-dialog" hidden><form id="settings-form"><details id="sharing"><summary>Carpetas compartidas</summary><div class="dialog-actions"></div></details><div id="settings-actions" class="dialog-actions"></div></form></main>';
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
    configurable: true,
    value: function () {
      this.open = true;
    },
  });
  Object.defineProperty(HTMLDialogElement.prototype, "close", {
    configurable: true,
    value: function () {
      this.open = false;
      this.dispatchEvent(new Event("close"));
    },
  });
  Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
    configurable: true,
    value: vi.fn(),
  });
});
async function start() {
  setupAppUpdates();
  await vi.waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith("check_app_update"),
  );
}
function pending() {
  let resolve!: (s: typeof ready) => void;
  api.invoke.mockImplementation((cmd: string) =>
    cmd === "check_app_update"
      ? new Promise((r) => {
          resolve = r;
        })
      : Promise.resolve(),
  );
  return (s = ready) => resolve(s);
}
it("registers availability listeners before the startup check and keeps updates outside shared folders", async () => {
  api.invoke.mockImplementation(async () => {
    expect(api.events.has("app-update-available")).toBe(true);
    return latest;
  });
  await start();
  expect(el("app-updates").parentElement!.id).toBe("settings-form");
  expect(el("sharing").contains(el("app-updates"))).toBe(false);
  expect(el("installed-app-version").textContent).toBe("v0.9.3");
  expect(el<HTMLDialogElement>("app-update-dialog").open).toBe(false);
  expect(installs()).toHaveLength(0);
});
it("announces a signed newer version before preparation; Later keeps downloads running and suppresses duplicate prompts", async () => {
  const resolve = pending();
  await start();
  send("app-update-available", available);
  expect(el<HTMLDialogElement>("app-update-dialog").open).toBe(true);
  expect(el("new-app-version").textContent).toBe("v0.10.0");
  el("update-later").click();
  send("app-update-ready", ready);
  resolve();
  await Promise.resolve();
  expect(el<HTMLDialogElement>("app-update-dialog").open).toBe(false);
  expect(installs()).toHaveLength(0);
  el("update-available").click();
  expect(el<HTMLDetailsElement>("app-updates").open).toBe(true);
  expect(el("settings-dialog").hidden).toBe(false);
  send("app-update-ready", ready);
  expect(el<HTMLDialogElement>("app-update-dialog").open).toBe(false);
  send("app-update-available", { ...available, version: "0.10.1" });
  expect(el<HTMLDialogElement>("app-update-dialog").open).toBe(true);
});
it("Update now waits for verified package readiness and installs exactly once despite repeated events", async () => {
  const resolve = pending();
  await start();
  send("app-update-available", available);
  el("update-now").click();
  expect(installs()).toHaveLength(0);
  expect(el<HTMLButtonElement>("update-later").disabled).toBe(false);
  send("app-update-ready", ready);
  send("app-update-ready", ready);
  resolve();
  await Promise.resolve();
  expect(installs()).toHaveLength(1);
});
it("Later or Escape can cancel queued consent; a different version requires fresh consent", async () => {
  const resolve = pending();
  await start();
  send("app-update-available", available);
  el("update-now").click();
  el("update-later").click();
  send("app-update-ready", ready);
  expect(installs()).toHaveLength(0);
  send("app-update-available", { ...available, version: "0.10.1" });
  el("update-now").click();
  el("app-update-dialog").dispatchEvent(
    new Event("cancel", { cancelable: true }),
  );
  send("app-update-ready", { ...ready, version: "0.10.1" });
  expect(installs()).toHaveLength(0);
  resolve();
  await Promise.resolve();
});
it("offline or invalid checks do not announce an update; install failures allow a deliberate retry", async () => {
  api.invoke.mockRejectedValueOnce(new Error("offline"));
  await start();
  await vi.waitFor(() =>
    expect(el<HTMLButtonElement>("check-app-update").disabled).toBe(false),
  );
  expect(el<HTMLDialogElement>("app-update-dialog").open).toBe(false);
  send("app-update-ready", {
    configured: true,
    ready: true,
    version: "<script>",
  });
  expect(installs()).toHaveLength(0);
  api.invoke.mockRejectedValue(new Error("install failed"));
  send("app-update-ready", ready);
  el("update-now").click();
  await vi.waitFor(() =>
    expect(el("update-prompt-state").textContent).toContain("install failed"),
  );
  expect(el<HTMLButtonElement>("update-now").disabled).toBe(false);
  expect(installs()).toHaveLength(1);
  api.invoke.mockResolvedValue(undefined);
  el("update-now").click();
  expect(installs()).toHaveLength(2);
});

it("ignores stale readiness/version events and binds consent to the displayed version", async () => {
  const resolve = pending();
  await start();
  send("app-update-ready", ready);
  send("app-update-available", available);
  expect(el("install-app-update").hidden).toBe(false);
  send("app-update-available", { ...available, version: "0.10.1" });
  send("app-update-ready", ready);
  send("app-update-ready", { ...ready, version: "0.10.1" });
  el("update-now").click();
  expect(api.invoke).toHaveBeenCalledWith("install_app_update", {
    expectedVersion: "0.10.1",
  });
  resolve();
  await Promise.resolve();
  expect(installs()).toHaveLength(1);
});
