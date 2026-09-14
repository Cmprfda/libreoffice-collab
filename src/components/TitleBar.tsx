/**
 * Custom Windows 11 title bar.
 *
 * The native decorations are off (tauri.conf.json -> decorations: false) so the
 * Mica backdrop can run edge to edge. `data-tauri-drag-region` is what makes the
 * bar draggable; it is the Tauri v2 equivalent of -webkit-app-region: drag.
 */
import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useI18n } from "../i18n";
import {
  AppLogo,
  CloseGlyph,
  MaximizeGlyph,
  MinimizeGlyph,
  RestoreGlyph,
} from "./Icons";

export function TitleBar() {
  const { t } = useI18n();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;

    void win.isMaximized().then(setMaximized);
    void win
      .onResized(() => {
        void win.isMaximized().then(setMaximized);
      })
      .then((fn) => {
        unlisten = fn;
      });

    return () => unlisten?.();
  }, []);

  const win = getCurrentWindow();

  return (
    <header className="titlebar" data-tauri-drag-region>
      <AppLogo />
      <span className="titlebar__title" data-tauri-drag-region>
        {t("app.name")}
      </span>
      <div className="titlebar__spacer" data-tauri-drag-region />

      <div className="titlebar__controls">
        <button
          className="caption-btn"
          title={t("titlebar.minimize")}
          aria-label={t("titlebar.minimize")}
          onClick={() => void win.minimize()}
        >
          <MinimizeGlyph />
        </button>
        <button
          className="caption-btn"
          title={maximized ? t("titlebar.restore") : t("titlebar.maximize")}
          aria-label={maximized ? t("titlebar.restore") : t("titlebar.maximize")}
          onClick={() => void win.toggleMaximize()}
        >
          {maximized ? <RestoreGlyph /> : <MaximizeGlyph />}
        </button>
        <button
          className="caption-btn caption-btn--close"
          title={t("titlebar.close")}
          aria-label={t("titlebar.close")}
          /* Closing hides to the system tray; quitting is done from the tray
             menu. This matches how Teams/Slack behave on Windows. */
          onClick={() => void win.hide()}
        >
          <CloseGlyph />
        </button>
      </div>
    </header>
  );
}
