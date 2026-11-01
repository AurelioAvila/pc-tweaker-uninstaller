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

// The app chrome is not selectable text (App.css), so Ctrl+A must not paint the
// whole window, or the page behind an open dialog: it selects inside a text
// field as usual, inside a focused log or command block, and nowhere else.
document.addEventListener("keydown", (event) => {
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.key.toLowerCase() !== "a") return;
  const target = event.target instanceof Element ? event.target : null;
  if (target?.closest("input, textarea, [contenteditable='true']")) return;
  event.preventDefault();
  const block = target?.closest("pre, code, .selectable");
  if (block) window.getSelection()?.selectAllChildren(block);
});

const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("#root not found");

createRoot(rootEl).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
