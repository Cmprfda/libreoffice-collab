/**
 * Key-based internationalisation.
 *
 * Design rules:
 *  - `pt.json` is the source of truth AND the fallback dictionary. Any key missing
 *    from `en.json` silently falls back to Portuguese instead of showing a raw key.
 *  - Switching language only swaps a React context value, so the whole UI re-renders
 *    instantly. No reload, no restart.
 *  - Placeholders use `{name}` and are replaced at call time: t("x", { version: "1.2.0" }).
 */
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  type ReactNode,
} from "react";
import pt from "./pt.json";
import en from "./en.json";

export type Lang = "pt" | "en";

/** Every translatable key, derived from the Portuguese dictionary. */
export type TranslationKey = keyof typeof pt;

type Dictionary = Record<string, string>;

const DICTIONARIES: Record<Lang, Dictionary> = {
  pt: pt as Dictionary,
  en: en as Dictionary,
};

/** BCP-47 tags, used for date formatting and for the Collabora editor `lang=` param. */
export const LOCALE_TAG: Record<Lang, string> = {
  pt: "pt-PT",
  en: "en-GB",
};

export type TranslateFn = (
  key: TranslationKey,
  vars?: Record<string, string | number>,
) => string;

interface I18nContextValue {
  lang: Lang;
  setLang: (lang: Lang) => void;
  t: TranslateFn;
  /** BCP-47 tag for the active language, e.g. "pt-PT". */
  locale: string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

function interpolate(
  template: string,
  vars?: Record<string, string | number>,
): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in vars ? String(vars[name]) : match,
  );
}

export function I18nProvider({
  lang,
  onLangChange,
  children,
}: {
  lang: Lang;
  onLangChange: (lang: Lang) => void;
  children: ReactNode;
}) {
  const t = useCallback<TranslateFn>(
    (key, vars) => {
      const dict = DICTIONARIES[lang];
      // Fall back to Portuguese, then to the key itself (visible in dev only).
      const template = dict[key] ?? DICTIONARIES.pt[key] ?? key;
      return interpolate(template, vars);
    },
    [lang],
  );

  // Keep the document language in sync so WebView2 hyphenation / a11y behave.
  useEffect(() => {
    document.documentElement.lang = LOCALE_TAG[lang];
  }, [lang]);

  const value = useMemo<I18nContextValue>(
    () => ({ lang, setLang: onLangChange, t, locale: LOCALE_TAG[lang] }),
    [lang, onLangChange, t],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used inside <I18nProvider>");
  return ctx;
}

/**
 * Detects a sensible default language from the OS/browser locale.
 * Anything Portuguese stays Portuguese; everything else gets English.
 * Only used on first launch, before any saved preference exists.
 */
export function detectDefaultLang(): Lang {
  const candidates = [
    ...(navigator.languages ?? []),
    navigator.language ?? "",
  ].map((l) => l.toLowerCase());
  if (candidates.some((l) => l.startsWith("pt"))) return "pt";
  return candidates.length > 0 && !candidates[0].startsWith("pt") ? "en" : "pt";
}

/** A small, locale-aware "17 de março, 14:32" style formatter. */
export function formatDateTime(iso: string | number, lang: Lang): string {
  const date = typeof iso === "number" ? new Date(iso * 1000) : new Date(iso);
  if (Number.isNaN(date.getTime())) return "—";
  return new Intl.DateTimeFormat(LOCALE_TAG[lang], {
    day: "2-digit",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

/** Human-readable file size that respects the decimal separator of the locale. */
export function formatBytes(bytes: number, lang: Lang): string {
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const formatted = new Intl.NumberFormat(LOCALE_TAG[lang], {
    maximumFractionDigits: value < 10 && unit > 0 ? 1 : 0,
  }).format(value);
  return `${formatted} ${units[unit]}`;
}
