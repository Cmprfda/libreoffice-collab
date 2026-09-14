import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/app.css";

// Block the WebView2 context menu and F5/Ctrl+R so the window behaves like a
// native app rather than a browser page.
document.addEventListener("contextmenu", (event) => {
  const target = event.target as HTMLElement;
  if (!target.closest("input, textarea, .dialog__notes")) event.preventDefault();
});

document.addEventListener("keydown", (event) => {
  if (event.key === "F5" || (event.ctrlKey && event.key.toLowerCase() === "r")) {
    event.preventDefault();
  }
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
