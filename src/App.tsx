/**
 * Application shell: owns settings, theme, language and routing.
 *
 * Settings are the single source of truth for the UI language, so switching the
 * dropdown in Settings re-renders every string in the app at once — including
 * this component's children — with no restart.
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import { Sidebar, type Route } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { ToastProvider } from "./components/Toasts";
import { UpdateDialog } from "./components/UpdateDialog";
import { I18nProvider, detectDefaultLang, type Lang } from "./i18n";
import {
  appVersion,
  getSettings,
  micaSupported,
  onNavigate,
  onUpdateAvailable,
  saveSettings,
} from "./lib/backend";
import { useCollab } from "./lib/useCollab";
import type { AppSettings, Theme, UpdateInfo } from "./lib/types";
import { Dashboard } from "./views/Dashboard";
import { Settings } from "./views/Settings";

/** Applies the effective theme to <html data-theme>, following the OS when asked. */
function useTheme(theme: Theme | undefined) {
  useEffect(() => {
    if (!theme) return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const effective =
        theme === "system" ? (media.matches ? "dark" : "light") : theme;
      document.documentElement.dataset.theme = effective;
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);
}

export default function App() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [version, setVersion] = useState("—");
  const [route, setRoute] = useState<Route>("dashboard");
  const [update, setUpdate] = useState<UpdateInfo | null>(null);

  /* ------------------------------------------------------------ bootstrap */

  useEffect(() => {
    void (async () => {
      const [loaded, appVer, mica] = await Promise.all([
        getSettings(),
        appVersion(),
        micaSupported(),
      ]);
      setVersion(appVer);
      // Mica is only available on Windows 11; without it we paint an opaque
      // background so the app never looks broken on older machines or VMs.
      if (!mica) document.body.classList.add("no-mica");
      setSettings(loaded);
    })();
  }, []);

  useTheme(settings?.theme);

  /* ---------------------------------------------- persisted settings patch */

  const patch = useCallback((changes: Partial<AppSettings>) => {
    setSettings((current) => {
      if (!current) return current;
      const next = { ...current, ...changes };
      // Fire-and-forget: the Rust side is the durable store, the UI is already
      // showing the new value optimistically.
      void saveSettings(next);
      return next;
    });
  }, []);

  const setLang = useCallback(
    (language: Lang) => patch({ language }),
    [patch],
  );

  /* ------------------------------------------------------ events from Rust */

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    void onUpdateAvailable(setUpdate).then((fn) => unlisteners.push(fn));
    void onNavigate((target) => setRoute(target as Route)).then((fn) =>
      unlisteners.push(fn),
    );
    return () => unlisteners.forEach((fn) => fn());
  }, []);

  /* ------------------------------------------------------------- collab */

  const collab = useCollab(settings);
  const discoveredUrl = useMemo(
    () => collab.servers.find((s) => !s.manual)?.base_url ?? null,
    [collab.servers],
  );

  // Before settings land we still need *a* language for the splash line.
  const lang: Lang = settings?.language ?? detectDefaultLang();

  return (
    <I18nProvider lang={lang} onLangChange={setLang}>
      <ToastProvider>
        <div className="app-shell">
          <TitleBar />
          {settings === null ? (
            <div className="empty" style={{ border: 0, margin: "auto" }}>
              <div className="progress progress--indeterminate" style={{ width: 180 }}>
                <div className="progress__fill" />
              </div>
            </div>
          ) : (
            <div className="app-body">
              <Sidebar route={route} onNavigate={setRoute} version={version} />
              {route === "dashboard" ? (
                <Dashboard collab={collab} />
              ) : (
                <Settings
                  settings={settings}
                  patch={patch}
                  version={version}
                  discoveredUrl={discoveredUrl}
                  onUpdateFound={setUpdate}
                />
              )}
            </div>
          )}
        </div>

        {update?.available && (
          <UpdateDialog info={update} onDismiss={() => setUpdate(null)} />
        )}
      </ToastProvider>
    </I18nProvider>
  );
}
