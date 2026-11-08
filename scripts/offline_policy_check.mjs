import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join, relative, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const targets = [
  "apps/vtt-ui/src", "apps/vtt-ui/index.html", "services/ai/src",
  "services/art/worker.py", "services/engine/src", "apps/desktop/src-tauri/src", "apps/scrying-cef"
];
const allowedUrls = new Set(["http://www.w3.org/2000/svg"]);
const findings = [];
function scan(path) {
  const entries = readdirSync(path, { withFileTypes: true });
  for (const entry of entries) {
    const file = join(path, entry.name);
    if (entry.isDirectory()) scan(file);
    else if (/\.(ts|tsx|js|jsx|mjs|cjs|rs|css|html|py|cs)$/.test(file)) check(file);
  }
}
function check(file) {
  const lines = readFileSync(file, "utf8").split("\n");
  lines.forEach((line, index) => {
    const cloudSdk = /(?:from\s*|require\s*\(\s*|import\s*\(\s*)["'](?:openai|anthropic|@anthropic-ai\/sdk|@google\/generative-ai|cohere(?:-ai)?)(?:["'\/])/.test(line);
    const urls = line.match(/https?:\/\/[^\s"'<>\x60)]+/g) ?? [];
    const remoteUrl = urls.some(url => !allowedUrls.has(url));
    if (cloudSdk || remoteUrl) findings.push(relative(root, file) + ":" + (index + 1) + ": " + line.trim());
  });
}
for (const target of targets) {
  const path = join(root, target);
  if (!existsSync(path)) continue;
  if (/\.[a-z]+$/.test(target)) check(path); else scan(path);
}
if (findings.length) {
  console.error("Offline runtime policy FAILED\n" + findings.join("\n"));
  process.exitCode = 1;
} else console.log("Offline runtime policy PASSED");
