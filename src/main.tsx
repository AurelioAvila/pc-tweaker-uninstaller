import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
// Ecosystem typography, self-hosted like everywhere else in the PC Tweaker
// family: display face for headings, Inter for UI text. No network fonts.
import "@fontsource/space-grotesk/500.css";
import "@fontsource/space-grotesk/700.css";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import App from "./App";

const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("#root not found");

createRoot(rootEl).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
