/**
 * "Definicoes" / "Settings".
 *
 * Every control writes straight through to the Rust-owned settings file, so a
 * change survives a restart. The language dropdown re-renders the whole UI
 * immediately — there is no "apply" button and no restart prompt.
 */
import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Toggle } from "../components/Toggle";
import { useToast } from "../components/Toasts";
import { ExternalIcon, UpdateIcon } from "../components/Icons";
import { formatDateTime, useI18n, type Lang } from "../i18n";
import { checkForUpdates, checkServer } from "../lib/backend";
import { GITHUB_REPO_URL } from "../lib/config";
import type { AppSettings, Theme, UpdateInfo } from "../lib/types";

export function Settings({
  settings,
  patch,
  version,
  discoveredUrl,
  onUpdateFound,
}: {
  settings: AppSettings;
  patch: (changes: Partial<AppSettings>) => void;
  version: string;
  /** Address of the server currently found over mDNS, used to pre-fill the field. */
  discoveredUrl: string | null;
  onUpdateFound: (info: UpdateInfo) => void;
}) {
  const { t, lang } = useI18n();
  const toast = useToast();
  const [checking, setChecking] = useState(false);
  const [testing, setTesting] = useState(false);

  // Auto-populate the manual address from mDNS the moment discovery is turned
  // off, so the user is never handed an empty field to fill in by hand. Only
  // ever fills a blank value — a hand-typed address is left alone.
  useEffect(() => {
    if (
      !settings.auto_discover &&
      settings.server_url.trim() === "" &&
      discoveredUrl
    ) {
      patch({ server_url: discoveredUrl });
    }
  }, [settings.auto_discover, settings.server_url, discoveredUrl, patch]);

  async function handleCheckUpdates() {
    setChecking(true);
    try {
      const info = await checkForUpdates();
      if (info.available) {
        onUpdateFound(info);
      } else {
        toast.push(t("update.upToDate"), "success");
      }
      patch({ last_update_check: Math.floor(Date.now() / 1000) });
    } catch (error) {
      toast.push(t("update.error", { error: String(error) }), "error");
    } finally {
      setChecking(false);
    }
  }

  async function handleTestConnection() {
    const url = settings.server_url.trim() || discoveredUrl || "";
    if (url === "") {
      toast.push(t("settings.testFail"), "error");
      return;
    }
    setTesting(true);
    const alive = await checkServer(url);
    setTesting(false);
    toast.push(
      alive ? t("settings.testOk") : t("settings.testFail"),
      alive ? "success" : "error",
    );
  }

  return (
    <div className="page">
      <div className="page__header">
        <h1 className="page__title">{t("settings.title")}</h1>
      </div>

      {/* -------------------------------------------------------- language */}
      <section className="section">
        <h2 className="section__title">{t("settings.section.language")}</h2>
        <div className="setting">
          <div className="setting__main">
            <div className="setting__label">{t("settings.language")}</div>
            <div className="setting__hint">{t("settings.languageHint")}</div>
          </div>
          <div className="setting__control">
            <select
              className="select"
              value={settings.language}
              aria-label={t("settings.language")}
              onChange={(event) =>
                patch({ language: event.target.value as Lang })
              }
            >
              <option value="pt">{t("settings.language.pt")}</option>
              <option value="en">{t("settings.language.en")}</option>
            </select>
          </div>
        </div>
      </section>

      {/* ---------------------------------------------------------- server */}
      <section className="section">
        <h2 className="section__title">{t("settings.section.server")}</h2>

        <div className="setting">
          <div className="setting__main">
            <div className="setting__label">{t("settings.autoDiscover")}</div>
            <div className="setting__hint">{t("settings.autoDiscoverHint")}</div>
          </div>
          <div className="setting__control">
            <Toggle
              checked={settings.auto_discover}
              label={t("settings.autoDiscover")}
              onChange={(next) => patch({ auto_discover: next })}
            />
          </div>
        </div>

        <div className="setting">
          <div className="setting__main">
            <div className="setting__label">{t("settings.serverUrl")}</div>
            <div className="setting__hint">{t("settings.serverUrlHint")}</div>
          </div>
          <div className="setting__control" style={{ minWidth: 300, gap: 8 }}>
            <input
              className="input"
              type="url"
              inputMode="url"
              spellCheck={false}
              aria-label={t("settings.serverUrl")}
              placeholder={discoveredUrl ?? t("settings.serverUrlPlaceholder")}
              value={settings.server_url}
              disabled={settings.auto_discover}
              onChange={(event) => patch({ server_url: event.target.value })}
            />
            <button
              className="btn"
              onClick={() => void handleTestConnection()}
              disabled={testing}
              style={{ whiteSpace: "nowrap" }}
            >
              {testing ? t("common.loading") : t("settings.testConnection")}
            </button>
          </div>
        </div>
      </section>

      {/* ------------------------------------------------------ appearance */}
      <section className="section">
        <h2 className="section__title">{t("settings.section.appearance")}</h2>

        <div className="setting">
          <div className="setting__main">
            <div className="setting__label">{t("settings.theme")}</div>
          </div>
          <div className="setting__control">
            <select
              className="select"
              value={settings.theme}
              aria-label={t("settings.theme")}
              onChange={(event) =>
                patch({ theme: event.target.value as Theme })
              }
            >
              <option value="system">{t("settings.theme.system")}</option>
              <option value="light">{t("settings.theme.light")}</option>
              <option value="dark">{t("settings.theme.dark")}</option>
            </select>
          </div>
        </div>

        <div className="setting">
          <div className="setting__main">
            <div className="setting__label">{t("settings.startMinimized")}</div>
          </div>
          <div className="setting__control">
            <Toggle
              checked={settings.start_minimized}
              label={t("settings.startMinimized")}
              onChange={(next) => patch({ start_minimized: next })}
            />
          </div>
        </div>
      </section>

      {/* --------------------------------------------------------- updates */}
      <section className="section">
        <h2 className="section__title">{t("settings.section.updates")}</h2>

        <div className="setting">
          <div className="setting__main">
            <div className="setting__label">
              {t("settings.installedVersion", { version })}
            </div>
            <div className="setting__hint">
              {t("settings.lastChecked", {
                when:
                  settings.last_update_check > 0
                    ? formatDateTime(settings.last_update_check, lang)
                    : t("settings.never"),
              })}
            </div>
          </div>
          <div className="setting__control">
            <button
              className="btn btn--accent"
              onClick={() => void handleCheckUpdates()}
              disabled={checking}
            >
              <UpdateIcon className={checking ? "spin" : ""} />
              {checking ? t("settings.checking") : t("settings.checkUpdates")}
            </button>
          </div>
        </div>

        <div className="setting">
          <div className="setting__main">
            <div className="setting__label">{t("settings.autoInstall")}</div>
            <div className="setting__hint">{t("settings.autoInstallHint")}</div>
          </div>
          <div className="setting__control">
            <Toggle
              checked={settings.auto_install_updates}
              label={t("settings.autoInstall")}
              onChange={(next) => patch({ auto_install_updates: next })}
            />
          </div>
        </div>
      </section>

      {/* ----------------------------------------------------------- about */}
      <section className="section">
        <h2 className="section__title">{t("settings.section.about")}</h2>
        <div className="setting">
          <div className="setting__main">
            <div className="setting__hint">{t("settings.aboutText")}</div>
          </div>
          <div className="setting__control">
            <button
              className="btn"
              onClick={() => void openUrl(GITHUB_REPO_URL)}
            >
              <ExternalIcon />
              {t("settings.openRepo")}
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
