import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { getRuntimeSpaceSiftClient } from "./lib/runtimeSpaceSiftClient";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App client={getRuntimeSpaceSiftClient()} />
  </React.StrictMode>,
);
