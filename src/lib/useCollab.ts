/**
 * The one hook that owns "which server are we talking to, is it alive, and what
 * documents does it have". Everything the dashboard shows comes from here.
 *
 * Two sources feed the server list:
 *   1. mDNS discovery (default, zero-config) — pushed from Rust.
 *   2. A manually typed address in Settings — used when auto-discovery is off.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  checkServer,
  listDocuments,
  listServers,
  onServersChanged,
  probeManualServer,
  restartDiscovery,
} from "./backend";
import {
  DOCUMENTS_POLL_INTERVAL_MS,
  HEALTH_POLL_INTERVAL_MS,
} from "./config";
import type {
  AppSettings,
  ConnectionState,
  DiscoveredServer,
  SharedDocument,
} from "./types";

export interface CollabState {
  servers: DiscoveredServer[];
  selected: DiscoveredServer | null;
  selectServer: (server: DiscoveredServer) => void;
  connection: ConnectionState;
  documents: SharedDocument[];
  loadingDocuments: boolean;
  /** Re-runs mDNS discovery and reloads the document list. */
  refresh: () => Promise<void>;
}

export function useCollab(settings: AppSettings | null): CollabState {
  const [discovered, setDiscovered] = useState<DiscoveredServer[]>([]);
  const [manual, setManual] = useState<DiscoveredServer | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [connection, setConnection] = useState<ConnectionState>("searching");
  const [documents, setDocuments] = useState<SharedDocument[]>([]);
  const [loadingDocuments, setLoadingDocuments] = useState(false);

  // Guards against a slow response from a server we have already left.
  const requestGuard = useRef(0);

  /* --------------------------------------------------------- mDNS discovery */

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    void listServers().then((found) => {
      if (!cancelled) setDiscovered(found);
    });
    void onServersChanged((found) => {
      if (!cancelled) setDiscovered(found);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  /* ------------------------------------------------------- manual fallback */

  useEffect(() => {
    if (!settings) return;
    const url = settings.server_url.trim();
    if (settings.auto_discover || url === "") {
      setManual(null);
      return;
    }
    let cancelled = false;
    void probeManualServer(url)
      .then((server) => {
        if (!cancelled) setManual(server);
      })
      .catch(() => {
        if (!cancelled) {
          // Keep the entry visible even when unreachable, so the user sees
          // *why* nothing works instead of an empty screen.
          setManual({
            id: "manual",
            name: url,
            host: url,
            port: 0,
            base_url: url,
            cool_url: "",
            version: "?",
            manual: true,
          });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [settings]);

  const servers = useMemo(() => {
    if (manual) return [manual, ...discovered.filter((s) => s.id !== "manual")];
    return discovered;
  }, [manual, discovered]);

  /* -------------------------------------------------- selection bookkeeping */

  const selected = useMemo(
    () => servers.find((s) => s.id === selectedId) ?? servers[0] ?? null,
    [servers, selectedId],
  );

  // Auto-select the first server that shows up, and drop a selection that
  // disappeared from the network.
  useEffect(() => {
    if (selectedId && !servers.some((s) => s.id === selectedId)) {
      setSelectedId(null);
    }
  }, [servers, selectedId]);

  const selectServer = useCallback((server: DiscoveredServer) => {
    setSelectedId(server.id);
    setDocuments([]);
    setConnection("connecting");
  }, []);

  /* ------------------------------------------------------------ health poll */

  useEffect(() => {
    if (!selected) {
      setConnection(servers.length === 0 ? "searching" : "disconnected");
      setDocuments([]);
      return;
    }

    let cancelled = false;
    setConnection((current) =>
      current === "connected" ? current : "connecting",
    );

    const probe = async () => {
      const alive = await checkServer(selected.base_url);
      if (!cancelled) setConnection(alive ? "connected" : "disconnected");
    };

    void probe();
    const timer = window.setInterval(probe, HEALTH_POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [selected, servers.length]);

  /* --------------------------------------------------------- document list */

  const loadDocuments = useCallback(async () => {
    if (!selected || connection !== "connected") return;
    const token = ++requestGuard.current;
    setLoadingDocuments(true);
    try {
      const docs = await listDocuments(selected.base_url);
      if (token === requestGuard.current) setDocuments(docs);
    } catch {
      if (token === requestGuard.current) setDocuments([]);
    } finally {
      if (token === requestGuard.current) setLoadingDocuments(false);
    }
  }, [selected, connection]);

  useEffect(() => {
    void loadDocuments();
    const timer = window.setInterval(
      () => void loadDocuments(),
      DOCUMENTS_POLL_INTERVAL_MS,
    );
    return () => window.clearInterval(timer);
  }, [loadDocuments]);

  const refresh = useCallback(async () => {
    setConnection("searching");
    await restartDiscovery();
    setDiscovered(await listServers());
    await loadDocuments();
  }, [loadDocuments]);

  return {
    servers,
    selected,
    selectServer,
    connection,
    documents,
    loadingDocuments,
    refresh,
  };
}
