/**
 * "Nova versao disponivel / New version available" modal.
 *
 * The heavy lifting (download, minisign signature verification, NSIS silent
 * install, relaunch) happens in Rust. This component only drives it and shows
 * progress in the active language.
 */
import { useEffect, useState } from "react";
import { useI18n } from "../i18n";
import {
  installUpdate,
  onUpdateProgress,
  type DownloadProgress,
} from "../lib/backend";
import type { UpdateInfo } from "../lib/types";
import { useToast } from "./Toasts";

export function UpdateDialog({
  info,
  onDismiss,
}: {
  info: UpdateInfo;
  onDismiss: () => void;
}) {
  const { t } = useI18n();
  const toast = useToast();
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<DownloadProgress | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onUpdateProgress(setProgress).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  async function handleInstall() {
    setBusy(true);
    try {
      // Resolves only if something goes wrong — on success the app relaunches.
      await installUpdate();
    } catch (error) {
      setBusy(false);
      toast.push(t("update.error", { error: String(error) }), "error");
    }
  }

  const percent = progress?.percent ?? 0;
  const downloading = busy && percent < 100;

  return (
    <div
      className="scrim"
      role="dialog"
      aria-modal="true"
      aria-labelledby="update-dialog-title"
    >
      <div className="dialog">
        <div className="dialog__body">
          <h2 className="dialog__title" id="update-dialog-title">
            {t("update.available.title")}
          </h2>
          <p className="dialog__text">
            {t("update.available.body", { version: info.version })}
          </p>

          {info.notes.trim() !== "" && !busy && (
            <>
              <p
                className="tiny muted"
                style={{ margin: "14px 0 4px", fontWeight: 600 }}
              >
                {t("update.notes")}
              </p>
              <div className="dialog__notes">{info.notes.trim()}</div>
            </>
          )}

          {busy && (
            <div style={{ marginTop: 18 }}>
              <p className="tiny muted" style={{ margin: "0 0 8px" }}>
                {downloading
                  ? t("update.downloading", { percent: Math.round(percent) })
                  : t("update.installing")}
              </p>
              <div
                className={`progress${downloading ? "" : " progress--indeterminate"}`}
              >
                <div
                  className="progress__fill"
                  style={downloading ? { width: `${percent}%` } : undefined}
                />
              </div>
            </div>
          )}
        </div>

        <div className="dialog__footer">
          <button
            className="btn btn--accent"
            onClick={() => void handleInstall()}
            disabled={busy}
          >
            {t("update.action")}
          </button>
          <button className="btn" onClick={onDismiss} disabled={busy}>
            {t("update.later")}
          </button>
        </div>
      </div>
    </div>
  );
}
