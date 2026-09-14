/**
 * Live connection indicator. Renders "Ligado"/"Desligado" in PT and
 * "Connected"/"Disconnected" in EN, driven purely by the i18n dictionary.
 */
import { useI18n, type TranslationKey } from "../i18n";
import type { ConnectionState } from "../lib/types";

const LABEL_KEY: Record<ConnectionState, TranslationKey> = {
  connected: "status.connected",
  disconnected: "status.disconnected",
  connecting: "status.connecting",
  searching: "status.searching",
};

export function StatusPill({ state }: { state: ConnectionState }) {
  const { t } = useI18n();
  return (
    <span
      className={`status-pill status-pill--${state}`}
      role="status"
      aria-live="polite"
    >
      <span className="status-pill__dot" />
      {t(LABEL_KEY[state])}
    </span>
  );
}
