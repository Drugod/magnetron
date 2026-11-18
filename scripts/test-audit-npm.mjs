import test from "node:test";
import assert from "node:assert/strict";
import { evaluate } from "./audit-npm.mjs";

const url = "https://github.com/advisories/GHSA-fc5q-39xc-vjpf";
function fixture() {
  return {
    report: { auditReportVersion: 2, vulnerabilities: {
      picomatch: { name: "picomatch", severity: "high", nodes: ["node_modules/picomatch"], via: [{ url, name: "picomatch", dependency: "picomatch", source: 1247710, severity: "high" }] },
      vite: { name: "vite", severity: "high", nodes: ["node_modules/vite"], via: ["picomatch"] },
    } },
    lock: { packages: { "node_modules/picomatch": { version: "4.0.7", dev: true }, "node_modules/vite": { version: "7.3.7", dev: true } } },
  };
}
const run = (f, date = "2026-11-18") => evaluate(f.report, f.lock, date);
test("only exact dev-only advisory and its dependency chain are accepted temporarily", () => {
  assert.deepEqual(run(fixture()).accepted, ["picomatch", "vite"]);
  assert.deepEqual(run(fixture()).blocked, []);
});
test("production, changed versions, nested copies and expired exceptions fail closed", () => {
  for (const mutate of [
    f => delete f.lock.packages["node_modules/picomatch"].dev,
    f => f.lock.packages["node_modules/picomatch"].version = "4.0.8",
    f => f.report.vulnerabilities.picomatch.nodes.push("node_modules/other/node_modules/picomatch"),
  ]) {
    const f = fixture(); mutate(f);
    assert.deepEqual(run(f).blocked, ["picomatch", "vite"]);
  }
  assert.deepEqual(run(fixture(), "2026-12-02").blocked, ["picomatch", "vite"]);
});
test("new advisories, new dependants, missing/cyclic evidence fail closed", () => {
  for (const mutate of [
    f => f.report.vulnerabilities.picomatch.via.push({ url: "https://example.com/new", severity: "high" }),
    f => f.report.vulnerabilities.picomatch.via[0].source++,
    f => f.report.vulnerabilities.picomatch.via = ["vite"],
    f => f.report.vulnerabilities.picomatch.via = [],
    f => delete f.report.vulnerabilities.picomatch,
  ]) {
    const f = fixture(); mutate(f);
    assert.ok(run(f).blocked.includes("vite"));
  }
  const f = fixture();
  f.report.vulnerabilities.runtime = { name: "runtime", severity: "moderate", nodes: [], via: ["picomatch"] };
  assert.deepEqual(run(f).blocked, ["runtime"]);
});
test("audit errors and schema changes fail; a clean audit needs no exception", () => {
  assert.throws(() => evaluate({ error: {} }, {}));
  assert.throws(() => evaluate({ auditReportVersion: 3, vulnerabilities: {} }, {}));
  assert.deepEqual(evaluate({ auditReportVersion: 2, vulnerabilities: {} }, {}, "2030-01-01").accepted, []);
});
