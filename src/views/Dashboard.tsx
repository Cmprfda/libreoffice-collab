/**
 * Main screen: discovered servers on top, shared documents below.
 * One click on a document opens it in a co-authoring editor window.
 */
import { useState } from "react";
import { StatusPill } from "../components/StatusPill";
import { useToast } from "../components/Toasts";
import {
  ChevronIcon,
  DocumentIcon,
  RefreshIcon,
  ServerIcon,
} from "../components/Icons";
import { formatBytes, formatDateTime, useI18n, type TranslationKey } from "../i18n";
import { openDocument } from "../lib/backend";
import type { CollabState } from "../lib/useCollab";
import type { DocumentKind, SharedDocument } from "../lib/types";

const KIND_LABEL: Record<DocumentKind, TranslationKey> = {
  text: "doc.type.text",
  spreadsheet: "doc.type.spreadsheet",
  presentation: "doc.type.presentation",
  drawing: "doc.type.drawing",
  other: "doc.type.other",
};

export function Dashboard({ collab }: { collab: CollabState }) {
  const { t, lang, locale } = useI18n();
  const toast = useToast();
  const [opening, setOpening] = useState<string | null>(null);

  const {
    servers,
    selected,
    selectServer,
    connection,
    documents,
    loadingDocuments,
    refresh,
  } = collab;

  async function handleOpen(doc: SharedDocument) {
    if (!selected) {
      toast.push(t("error.noServer"), "error");
      return;
    }
    setOpening(doc.id);
    toast.push(t("editor.opening", { name: doc.name }));
    try {
      await openDocument({
        baseUrl: selected.base_url,
        docId: doc.id,
        docName: doc.name,
        // Collabora localises its own toolbars from this tag.
        locale,
      });
    } catch (error) {
      toast.push(t("editor.error", { error: String(error) }), "error");
    } finally {
      setOpening(null);
    }
  }

  return (
    <div className="page">
      <div className="page__header">
        <div className="inline" style={{ justifyContent: "space-between" }}>
          <div>
            <h1 className="page__title">{t("nav.dashboard")}</h1>
            <p className="page__subtitle">
              {selected
                ? `${t("dashboard.connectedTo")} ${selected.name}`
                : t("app.tagline")}
            </p>
          </div>
          <div className="inline">
            <StatusPill state={connection} />
            <button
              className="btn btn--subtle"
              onClick={() => void refresh()}
              title={t("dashboard.refresh")}
            >
              <RefreshIcon className={connection === "searching" ? "spin" : ""} />
              {t("dashboard.refresh")}
            </button>
          </div>
        </div>
      </div>

      {/* ------------------------------------------------------- servers -- */}
      <section className="section">
        <h2 className="section__title">{t("dashboard.servers")}</h2>
        <p className="section__hint">{t("dashboard.serversHint")}</p>

        {servers.length === 0 ? (
          <div className="empty">
            <ServerIcon size={28} />
            <span className="empty__title">
              {connection === "searching"
                ? t("dashboard.searching")
                : t("dashboard.noServers")}
            </span>
            <span className="empty__hint">{t("dashboard.noServersHint")}</span>
          </div>
        ) : (
          <div className="card-list">
            {servers.map((server) => (
              <button
                key={server.id}
                className={`row${selected?.id === server.id ? " row--selected" : ""}`}
                onClick={() => selectServer(server)}
              >
                <span className="row__icon">
                  <ServerIcon size={18} />
                </span>
                <span className="row__main">
                  <span className="row__title">{server.name}</span>
                  <span className="row__meta">
                    {server.base_url}
                    {server.manual ? ` · ${t("dashboard.manual")}` : ""}
                    {server.version ? ` · v${server.version}` : ""}
                  </span>
                </span>
                {selected?.id === server.id && (
                  <span className="row__trailing">
                    <StatusPill state={connection} />
                  </span>
                )}
              </button>
            ))}
          </div>
        )}
      </section>

      {/* ----------------------------------------------------- documents -- */}
      <section className="section">
        <h2 className="section__title">{t("dashboard.documents")}</h2>
        <p className="section__hint">{t("dashboard.documentsHint")}</p>

        {loadingDocuments && documents.length === 0 && (
          <div className="progress progress--indeterminate" style={{ marginBottom: 8 }}>
            <div className="progress__fill" />
          </div>
        )}

        {documents.length === 0 ? (
          <div className="empty">
            <DocumentIcon kind="text" size={28} />
            <span className="empty__title">
              {connection === "connected"
                ? t("dashboard.noDocuments")
                : t("error.noServer")}
            </span>
            <span className="empty__hint">
              {connection === "connected"
                ? t("dashboard.noDocumentsHint")
                : t("dashboard.noServersHint")}
            </span>
          </div>
        ) : (
          <div className="card-list">
            {documents.map((doc) => (
              <button
                key={doc.id}
                className="row"
                onClick={() => void handleOpen(doc)}
                disabled={opening === doc.id}
              >
                <span className="row__icon">
                  <DocumentIcon kind={doc.kind} />
                </span>
                <span className="row__main">
                  <span className="row__title">{doc.name}</span>
                  <span className="row__meta">
                    {t(KIND_LABEL[doc.kind])} · {formatBytes(doc.size, lang)} ·{" "}
                    {t("dashboard.modified")} {formatDateTime(doc.modified, lang)}
                  </span>
                </span>
                <span className="row__trailing muted tiny">
                  {t("dashboard.open")}
                  <ChevronIcon size={14} />
                </span>
              </button>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
