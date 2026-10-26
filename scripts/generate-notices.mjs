// Input: cargo metadata --format-version 1 --filter-platform x86_64-pc-windows-msvc
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
const metadata = JSON.parse(fs.readFileSync(process.argv[2], "utf8").replace(/^\uFEFF/, ""));
const reachable = new Set(metadata.resolve.nodes.map(n => n.id));
const texts = new Map();
const version = JSON.parse(fs.readFileSync("package.json", "utf8")).version;
let output = `MAGNETRON ${version} — Rust dependency notices\n\n`;
for (const pkg of metadata.packages.filter(p => (p.source || path.relative(process.cwd(), p.manifest_path).replaceAll("\\", "/").startsWith("third_party/")) && reachable.has(p.id)).sort((a,b) => `${a.name} ${a.version}`.localeCompare(`${b.name} ${b.version}`))) {
  output += `${pkg.name} ${pkg.version}\nLicense: ${pkg.license ?? "See included license"}\nSource: ${pkg.repository ?? `https://crates.io/crates/${pkg.name}/${pkg.version}`}\n`;
  if (path.relative(process.cwd(), pkg.manifest_path).replaceAll("\\", "/").startsWith("third_party/")) {
    output += "Locally modified by MAGNETRON; reviewed changes and upstream checksums: third_party/PATCHES.json in the corresponding source release.\n";
  }
  const dir = path.dirname(pkg.manifest_path);
  const files = fs.readdirSync(dir).filter(n => /^(license|copying|notice|copyright)([.\-_]|$)/i.test(n));
  if (pkg.license_file) files.push(pkg.license_file);
  for (const name of new Set(files)) {
    const file = path.join(dir, name);
    if (!fs.statSync(file).isFile()) continue;
    const text = fs.readFileSync(file, "utf8");
    const hash = crypto.createHash("sha256").update(text).digest("hex").slice(0,16);
    texts.set(hash, text); output += `License text: ${hash}\n`;
  }
  output += "\n";
}
for (const [hash,text] of texts) output += `\n===== License text ${hash} =====\n${text}\n`;
fs.writeFileSync("artifacts/licenses/THIRD-PARTY-NOTICES.txt",output);
console.log(`${texts.size} license texts included.`);
