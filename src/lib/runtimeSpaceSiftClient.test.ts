import { describe, expect, it } from "vitest";
import { getRuntimeSpaceSiftClient } from "./runtimeSpaceSiftClient";
import { tauriSpaceSiftClient } from "./tauriSpaceSiftClient";
import { unsupportedClient } from "./spaceSiftClient";

function setTauriRuntime(value: boolean) {
  Object.defineProperty(globalThis, "isTauri", {
    configurable: true,
    value,
  });
}

describe("runtime Space Sift client selection", () => {
  it("uses the unsupported client outside Tauri so browser visual review degrades cleanly", () => {
    setTauriRuntime(false);

    expect(getRuntimeSpaceSiftClient()).toBe(unsupportedClient);
  });

  it("uses the Tauri client inside the desktop runtime", () => {
    setTauriRuntime(true);

    expect(getRuntimeSpaceSiftClient()).toBe(tauriSpaceSiftClient);
  });
});
