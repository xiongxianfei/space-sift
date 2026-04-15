import { readFileSync, existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const currentFile = fileURLToPath(import.meta.url);
const repoRoot = path.resolve(path.dirname(currentFile), "..");

function readRepoFile(...segments: string[]) {
  return readFileSync(path.join(repoRoot, ...segments), "utf8");
}

function readJsonFile<T>(...segments: string[]) {
  return JSON.parse(readRepoFile(...segments)) as T;
}

function readCargoPackageVersion() {
  const cargoToml = readRepoFile("src-tauri", "Cargo.toml");
  const match = cargoToml.match(/^\[package\][\s\S]*?^version = "([^"]+)"/m);

  if (!match) {
    throw new Error("Could not find package version in src-tauri/Cargo.toml");
  }

  return match[1];
}

function currentVersionDirectory() {
  const packageJson = readJsonFile<{ version: string }>("package.json");
  return path.join(
    repoRoot,
    "winget",
    "manifests",
    "x",
    "xiongxianfei",
    "SpaceSift",
    packageJson.version,
  );
}

describe("Space Sift release hardening", () => {
  it("keeps release metadata and winget manifests on the same version", () => {
    const packageJson = readJsonFile<{ version: string }>("package.json");
    const tauriConfig = readJsonFile<{ version: string }>("src-tauri", "tauri.conf.json");
    const cargoVersion = readCargoPackageVersion();
    const wingetDir = currentVersionDirectory();
    const versionManifestPath = path.join(wingetDir, "xiongxianfei.SpaceSift.yaml");
    const localeManifestPath = path.join(
      wingetDir,
      "xiongxianfei.SpaceSift.locale.en-US.yaml",
    );
    const installerManifestPath = path.join(
      wingetDir,
      "xiongxianfei.SpaceSift.installer.yaml",
    );

    expect(packageJson.version).toBe(tauriConfig.version);
    expect(packageJson.version).toBe(cargoVersion);
    expect(existsSync(versionManifestPath)).toBe(true);
    expect(existsSync(localeManifestPath)).toBe(true);
    expect(existsSync(installerManifestPath)).toBe(true);

    const versionManifest = readFileSync(versionManifestPath, "utf8");
    const localeManifest = readFileSync(localeManifestPath, "utf8");
    const installerManifest = readFileSync(installerManifestPath, "utf8");

    expect(versionManifest).toContain(`PackageVersion: ${packageJson.version}`);
    expect(localeManifest).toContain(`PackageVersion: ${packageJson.version}`);
    expect(installerManifest).toContain(`PackageVersion: ${packageJson.version}`);
  });

  it("keeps tauri release bundling configured for public Windows artifacts", () => {
    const tauriConfig = readJsonFile<{
      bundle: {
        active: boolean;
        publisher?: string;
        homepage?: string;
        shortDescription?: string;
        windows?: {
          allowDowngrades?: boolean;
          webviewInstallMode?: {
            type?: string;
          };
          signCommand?: unknown;
        };
      };
    }>("src-tauri", "tauri.conf.json");
    const releaseConfigWriter = readRepoFile("scripts", "write-tauri-release-config.mjs");

    expect(tauriConfig.bundle.active).toBe(true);
    expect(tauriConfig.bundle.publisher).toBeTruthy();
    expect(tauriConfig.bundle.homepage).toMatch(/^https:\/\/github\.com\//);
    expect(tauriConfig.bundle.shortDescription).toBeTruthy();
    expect(tauriConfig.bundle.windows?.allowDowngrades).toBe(false);
    expect(tauriConfig.bundle.windows?.webviewInstallMode?.type).toBe(
      "downloadBootstrapper",
    );
    expect(tauriConfig.bundle.windows?.signCommand).toBeTruthy();
    expect(releaseConfigWriter).toContain("TAURI_UPDATER_PUBLIC_KEY");
    expect(releaseConfigWriter).toContain("createUpdaterArtifacts: true");
    expect(releaseConfigWriter).toContain("latest/download/latest.json");
  });

  it("uses a real tag-driven Windows release workflow with signing gates", () => {
    const workflow = readRepoFile(".github", "workflows", "release.yml");

    expect(workflow).toContain('name: release');
    expect(workflow).toContain('- "v*"');
    expect(workflow).toContain("windows-latest");
    expect(workflow).toContain("tauri-apps/tauri-action@v0");
    expect(workflow).toContain("WINDOWS_CERTIFICATE");
    expect(workflow).toContain("WINDOWS_CERTIFICATE_PASSWORD");
    expect(workflow).toContain("TAURI_SIGNING_PRIVATE_KEY");
    expect(workflow).toContain("TAURI_SIGNING_PRIVATE_KEY_PASSWORD");
    expect(workflow).toContain("TAURI_UPDATER_PUBLIC_KEY");
    expect(workflow).toContain("npm run release:config");
    expect(workflow).toContain("src-tauri/tauri.release.conf.json");
    expect(workflow).toContain("scripts/release-verify.sh");
  });

  it("documents the release runbook and repository secret names", () => {
    const releaseDoc = readRepoFile("docs", "release.md");

    expect(releaseDoc).toContain("WINDOWS_CERTIFICATE");
    expect(releaseDoc).toContain("WINDOWS_CERTIFICATE_PASSWORD");
    expect(releaseDoc).toContain("TAURI_SIGNING_PRIVATE_KEY");
    expect(releaseDoc).toContain("TAURI_SIGNING_PRIVATE_KEY_PASSWORD");
    expect(releaseDoc).toContain("TAURI_UPDATER_PUBLIC_KEY");
    expect(releaseDoc).toContain("bash scripts/release-verify.sh");
    expect(releaseDoc).toContain("npm run release:config");
    expect(releaseDoc).toContain("winget/manifests");
  });

  it("points the installer manifest at the GitHub release asset for the current version", () => {
    const packageJson = readJsonFile<{ version: string }>("package.json");
    const installerManifest = readFileSync(
      path.join(
        currentVersionDirectory(),
        "xiongxianfei.SpaceSift.installer.yaml",
      ),
      "utf8",
    );

    expect(installerManifest).toContain(
      `https://github.com/xiongxianfei/space-sift/releases/download/v${packageJson.version}/`,
    );
    expect(installerManifest).toContain("InstallerSha256: REPLACE_WITH_RELEASE_SHA256");
  });
});
