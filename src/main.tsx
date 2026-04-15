import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { tauriSpaceSiftClient } from "./lib/tauriSpaceSiftClient";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App client={tauriSpaceSiftClient} />
  </React.StrictMode>,
);
