import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

// Suppress the webview right-click menu so the app does not feel like a browser.
window.addEventListener("contextmenu", (e) => e.preventDefault());

// Retry any image that fails to load, anywhere in the app. A full lobby fetches
// dozens of images at once and a few can drop transiently; without this they
// would fall back to a placeholder permanently. Resource load errors do not
// bubble, so this listens in the capture phase. After a few tries the element is
// removed so the placeholder behind it shows.
window.addEventListener(
  "error",
  (event) => {
    const img = event.target as HTMLImageElement | null;
    if (!img || img.tagName !== "IMG" || !img.src) return;
    const tries = Number(img.dataset.tries || "0");
    if (tries >= 3) {
      img.remove();
      return;
    }
    img.dataset.tries = String(tries + 1);
    if (!img.dataset.base) img.dataset.base = img.src.split("?")[0];
    const base = img.dataset.base;
    window.setTimeout(() => {
      img.src = `${base}?retry=${tries + 1}`;
    }, 600 * (tries + 1));
  },
  true
);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
