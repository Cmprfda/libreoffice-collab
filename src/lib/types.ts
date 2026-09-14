/** Shared types. Every shape here mirrors a `serde` struct in `src-tauri/src`. */

import type { Lang } from "../i18n";

export type Theme = "system" | "light" | "dark";

/** Persisted user preferences, owned by the Rust side (settings.rs). */
export interface AppSettings {
  language: Lang;
  theme: Theme;
  /** When true the mDNS browser picks the server; when false `server_url` wins. */
  auto_discover: boolean;
  /** Manual fallback, e.g. "http://192.168.1.50:7373". Empty when unused. */
  server_url: string;
  auto_install_updates: boolean;
  start_minimized: boolean;
  /** Unix seconds of the last successful update check, 0 when never. */
  last_update_check: number;
}

/** A collaboration server found over mDNS (or entered manually). */
export interface DiscoveredServer {
  /** Stable identity: the mDNS instance fullname, or "manual" for the typed one. */
  id: string;
  /** Friendly name advertised by the host, e.g. "Escritorio - Piso 2". */
  name: string;
  host: string;
  port: number;
  /** Base REST/WOPI endpoint, e.g. "http://192.168.1.50:7373". */
  base_url: string;
  /** Collabora Online endpoint announced by the host, e.g. "http://192.168.1.50:9980". */
  cool_url: string;
  version: string;
  /** True when this entry came from the manual address instead of mDNS. */
  manual: boolean;
}

export type DocumentKind =
  | "text"
  | "spreadsheet"
  | "presentation"
  | "drawing"
  | "other";

/** One file in the host's shared folder. */
export interface SharedDocument {
  /** Opaque id used as the WOPI file id. */
  id: string;
  name: string;
  extension: string;
  kind: DocumentKind;
  size: number;
  /** Unix seconds. */
  modified: number;
}

export type ConnectionState = "searching" | "connecting" | "connected" | "disconnected";

/** Result of an update check (thin wrapper over the Tauri updater plugin). */
export interface UpdateInfo {
  available: boolean;
  current_version: string;
  /** Only meaningful when `available` is true. */
  version: string;
  notes: string;
  date: string;
}
