import { isTauri } from "@tauri-apps/api/core";
import type { SpaceSiftClient } from "./spaceSiftClient";
import { unsupportedClient } from "./spaceSiftClient";
import { tauriSpaceSiftClient } from "./tauriSpaceSiftClient";

export function getRuntimeSpaceSiftClient(): SpaceSiftClient {
  return isTauri() ? tauriSpaceSiftClient : unsupportedClient;
}
