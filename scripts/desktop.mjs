import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";

const mode = process.argv[2];
if (!["dev", "build"].includes(mode)) throw new Error("Usage: node scripts/desktop.mjs dev|build");
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const env = { ...process.env };
// Avoid OneDrive's read-only directory attribute breaking Rust build-script probes.
if (process.platform === "win32" && !env.CARGO_TARGET_DIR) {
  env.CARGO_TARGET_DIR = join(env.LOCALAPPDATA || tmpdir(), "DungeonWorldAI", "cargo-target");
}
if (env.CARGO_TARGET_DIR) {
  mkdirSync(env.CARGO_TARGET_DIR, { recursive: true });
  console.log("Rust build output: " + env.CARGO_TARGET_DIR);
}
const task = mode === "build" ? "tauri:build" : "dev";
const child = process.platform === "win32"
  ? spawn(env.ComSpec || "cmd.exe", ["/d", "/s", "/c", "npm.cmd run " + task + " --workspace @dwa/desktop"], { cwd: root, env, stdio: "inherit" })
  : spawn("npm", ["run", task, "--workspace", "@dwa/desktop"], { cwd: root, env, stdio: "inherit" });
child.on("error", error => { console.error(error.message); process.exitCode = 1; });
child.on("exit", code => { process.exitCode = code ?? 1; });
process.on("SIGINT", () => child.kill("SIGINT"));
process.on("SIGTERM", () => child.kill("SIGTERM"));
