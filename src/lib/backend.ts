/**
 * Thin, fully typed wrapper around the Rust commands and events.
 * Nothing else in the UI should call `invoke` or `listen` directly.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSettings,
  DiscoveredServer,
  SharedDocument,
  UpdateInfo,
} from "./types";

/* ------------------------------------------------------------------ settings */

export const getSettings = () => invoke<AppSettings>("get_settings");

export const saveSettings = (settings: AppSettings) =>
  invoke<AppSettings>("save_settings", { settings });

export const appVersion = () => invoke<string>("app_version");

/** False on Windows 10 / VMs where the Mica backdrop could not be applied. */
export const micaSupported = () => invoke<boolean>("mica_supported");

/* ----------------------------------------------------------------- discovery */

/** Snapshot of everything the mDNS browser currently sees. */
export const listServers = () => invoke<DiscoveredServer[]>("list_servers");

/** Tears the mDNS browser down and starts it again (used by the Refresh button). */
export const restartDiscovery = () => invoke<void>("restart_discovery");

/** Fires whenever a server appears on or disappears from the network. */
export function onServersChanged(
  handler: (servers: DiscoveredServer[]) => void,
): Promise<UnlistenFn> {
  return listen<DiscoveredServer[]>("servers://changed", (event) =>
    handler(event.payload),
  );
}

/* -------------------------------------------------------------------- server */

/** Cheap liveness probe against `${baseUrl}/api/health`. Never throws. */
export async function checkServer(baseUrl: string): Promise<boolean> {
  try {
    return await invoke<boolean>("check_server", { baseUrl });
  } catch {
    return false;
  }
}

/** Builds a `DiscoveredServer` from a hand-typed address by probing it. */
export const probeManualServer = (baseUrl: string) =>
  invoke<DiscoveredServer>("probe_manual_server", { baseUrl });

export const listDocuments = (baseUrl: string) =>
  invoke<SharedDocument[]>("list_documents", { baseUrl });

/**
 * Opens a dedicated, maximised window that loads the Collabora editor for one
 * document. The window is what makes co-authoring work: it talks WOPI to the
 * host, so no `.~lock` file is ever created on the client.
 */
export const openDocument = (args: {
  baseUrl: string;
  docId: string;
  docName: string;
  locale: string;
}) => invoke<void>("open_document", args);

/* ------------------------------------------------------------------- updates */

export const checkForUpdates = () => invoke<UpdateInfo>("check_for_updates");

/** Downloads, verifies the signature, installs, then relaunches the app. */
export const installUpdate = () => invoke<void>("install_update");

export interface DownloadProgress {
  downloaded: number;
  total: number;
  percent: number;
}

export function onUpdateProgress(
  handler: (progress: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgress>("update://progress", (event) =>
    handler(event.payload),
  );
}

/** Emitted by the silent startup check when a newer release exists. */
export function onUpdateAvailable(
  handler: (info: UpdateInfo) => void,
): Promise<UnlistenFn> {
  return listen<UpdateInfo>("update://available", (event) =>
    handler(event.payload),
  );
}

/** Emitted when the tray menu asks the UI to show Settings. */
export function onNavigate(
  handler: (route: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("app://navigate", (event) => handler(event.payload));
}
