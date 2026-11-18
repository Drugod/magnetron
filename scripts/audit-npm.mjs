import { spawnSync } from "node:child_process";
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

// See docs/SECURITY-NPM-2026-11.md: temporary, build-only risk acceptance.
const advisory = "https://github.com/advisories/GHSA-fc5q-39xc-vjpf";
const versions = { picomatch: "4.0.7", fdir: "6.5.0", tinyglobby: "0.2.17", vite: "7.3.7", vitest: "5.0.5" };
const expires = "2026-12-02";

export function evaluate(report, lock, today = new Date().toISOString().slice(0, 10)) {
  if (report.auditReportVersion !== 2 || report.error || !report.vulnerabilities ||
      typeof report.vulnerabilities !== "object" || Array.isArray(report.vulnerabilities)) {
    throw new Error("Unrecognised or incomplete npm audit response");
  }
  const entries = report.vulnerabilities;
  const accepted = [], blocked = [];
  function covered(name, visiting = new Set()) {
    const item = entries[name];
    if (!item || visiting.has(name) || today >= expires || !versions[name] ||
        item.name !== name || !Array.isArray(item.nodes) || item.nodes.length !== 1 ||
        item.nodes[0] !== `node_modules/${name}` || !Array.isArray(item.via) || !item.via.length) return false;
    const installed = lock.packages?.[item.nodes[0]];
    if (!installed || installed.dev !== true || installed.version !== versions[name]) return false;
    const next = new Set(visiting).add(name);
    return item.via.every(via => typeof via === "string" ? covered(via, next) :
      name === "picomatch" && via?.url === advisory && via.name === "picomatch" &&
      via.dependency === "picomatch" && via.source === 1247710 && via.severity === "high");
  }
  for (const [name, item] of Object.entries(entries)) {
    if (!["info", "low", "moderate", "high", "critical"].includes(item.severity)) {
      throw new Error(`Unrecognised severity for ${name}`);
    }
    if (covered(name)) accepted.push(name);
    else if (["moderate", "high", "critical"].includes(item.severity)) blocked.push(name);
  }
  return { accepted, blocked, advisory, exception_expires: expires };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  // Constant command; no repository or user data is interpolated into the shell.
  const audit = spawnSync("npm audit --json", { shell: true, encoding: "utf8", maxBuffer: 8 * 1024 * 1024 });
  if (audit.error || ![0, 1].includes(audit.status)) throw new Error("npm audit failed to run");
  const report = JSON.parse(audit.stdout);
  const lock = JSON.parse(readFileSync("package-lock.json", "utf8"));
  const result = evaluate(report, lock);
  mkdirSync("artifacts/security", { recursive: true });
  writeFileSync("artifacts/security/npm-audit.json", JSON.stringify({ report, evaluation: result }, null, 2) + "\n");
  console.log(JSON.stringify(result, null, 2));
  if (result.accepted.length) console.warn(`WARNING: unpatched build-only ${advisory}; reviewed exception expires ${expires}. See docs/SECURITY-NPM-2026-11.md.`);
  if (result.blocked.length) process.exitCode = 1;
}
