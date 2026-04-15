import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const currentFile = fileURLToPath(import.meta.url);
const repoRoot = path.resolve(path.dirname(currentFile), "..");
const outputPath = path.join(repoRoot, "src-tauri", "tauri.release.conf.json");
const publicKey = process.env.TAURI_UPDATER_PUBLIC_KEY?.trim();

if (!publicKey) {
  console.error(
    "Missing TAURI_UPDATER_PUBLIC_KEY. Set the updater public key before generating release config.",
  );
  process.exit(1);
}

if (publicKey.startsWith("REPLACE_WITH_")) {
  console.error(
    "TAURI_UPDATER_PUBLIC_KEY is still a placeholder. Provide the real updater public key.",
  );
  process.exit(1);
}

const releaseConfig = {
  bundle: {
    createUpdaterArtifacts: true,
  },
  plugins: {
    updater: {
      pubkey: publicKey,
      endpoints: [
        "https://github.com/xiongxianfei/space-sift/releases/latest/download/latest.json",
      ],
      windows: {
        installMode: "passive",
      },
    },
  },
};

mkdirSync(path.dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${JSON.stringify(releaseConfig, null, 2)}\n`, "utf8");

console.log(`Wrote ${path.relative(repoRoot, outputPath)}`);
