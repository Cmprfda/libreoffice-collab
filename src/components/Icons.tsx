/**
 * Inline Fluent-style icons. Kept as components (instead of an icon font or a
 * dependency) so the bundle stays tiny and everything inherits `currentColor`.
 */
import type { SVGProps } from "react";
import type { DocumentKind } from "../lib/types";

type IconProps = SVGProps<SVGSVGElement> & { size?: number };

function Svg({ size = 16, children, ...rest }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.4}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...rest}
    >
      {children}
    </svg>
  );
}

export const HomeIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M3.5 8.5 10 3.2l6.5 5.3V16a1 1 0 0 1-1 1h-3v-4.5h-5V17h-3a1 1 0 0 1-1-1Z" />
  </Svg>
);

export const SettingsIcon = (p: IconProps) => (
  <Svg {...p}>
    <circle cx="10" cy="10" r="2.4" />
    <path d="M10 2.5h.7l.4 2a5.9 5.9 0 0 1 1.7.7l1.7-1.1 1.4 1.4-1.1 1.7c.3.5.6 1.1.7 1.7l2 .4v2l-2 .4a5.9 5.9 0 0 1-.7 1.7l1.1 1.7-1.4 1.4-1.7-1.1c-.5.3-1.1.6-1.7.7l-.4 2h-2l-.4-2a5.9 5.9 0 0 1-1.7-.7l-1.7 1.1-1.4-1.4 1.1-1.7a5.9 5.9 0 0 1-.7-1.7l-2-.4v-2l2-.4c.1-.6.4-1.2.7-1.7L3.8 5.5l1.4-1.4 1.7 1.1c.5-.3 1.1-.6 1.7-.7l.4-2Z" />
  </Svg>
);

export const ServerIcon = (p: IconProps) => (
  <Svg {...p}>
    <rect x="3" y="3.5" width="14" height="5.5" rx="1.4" />
    <rect x="3" y="11" width="14" height="5.5" rx="1.4" />
    <path d="M6 6.25h.01M6 13.75h.01" strokeWidth={1.8} />
  </Svg>
);

export const RefreshIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M16.5 10a6.5 6.5 0 1 1-1.9-4.6" />
    <path d="M16.6 2.8v3.1h-3.1" />
  </Svg>
);

export const UpdateIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M10 3v9" />
    <path d="M6.5 8.5 10 12l3.5-3.5" />
    <path d="M3.5 14v1.5a1.5 1.5 0 0 0 1.5 1.5h10a1.5 1.5 0 0 0 1.5-1.5V14" />
  </Svg>
);

export const CheckIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="m4.5 10.5 3.5 3.5 7.5-8" />
  </Svg>
);

export const AlertIcon = (p: IconProps) => (
  <Svg {...p}>
    <circle cx="10" cy="10" r="7" />
    <path d="M10 6.5v4.2M10 13.6h.01" strokeWidth={1.7} />
  </Svg>
);

export const ExternalIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M11 3.5h5.5V9" />
    <path d="M16.5 3.5 9 11" />
    <path d="M15 12v3.5a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 3 15.5v-9A1.5 1.5 0 0 1 4.5 5H8" />
  </Svg>
);

export const ChevronIcon = (p: IconProps) => (
  <Svg {...p}>
    <path d="M7.5 4.5 13 10l-5.5 5.5" />
  </Svg>
);

/* ---------------------------------------------------- window caption glyphs */
/* Win11 caption glyphs are 10x10 hairlines; stroke 1 keeps them crisp. */

const Caption = ({ children }: { children: React.ReactNode }) => (
  <svg
    width="10"
    height="10"
    viewBox="0 0 10 10"
    fill="none"
    stroke="currentColor"
    strokeWidth="1"
    aria-hidden="true"
  >
    {children}
  </svg>
);

export const MinimizeGlyph = () => (
  <Caption>
    <path d="M0 5h10" />
  </Caption>
);

export const MaximizeGlyph = () => (
  <Caption>
    <rect x="0.5" y="0.5" width="9" height="9" />
  </Caption>
);

export const RestoreGlyph = () => (
  <Caption>
    <rect x="0.5" y="2.5" width="7" height="7" />
    <path d="M2.5 2.5v-2h7v7h-2" />
  </Caption>
);

export const CloseGlyph = () => (
  <Caption>
    <path d="M0 0l10 10M10 0L0 10" />
  </Caption>
);

/* ------------------------------------------------------------ document type */

/** Colour-coded document icon matching LibreOffice module colours. */
export function DocumentIcon({
  kind,
  size = 20,
}: {
  kind: DocumentKind;
  size?: number;
}) {
  const color = {
    text: "#2a6099",
    spreadsheet: "#106f39",
    presentation: "#c9591a",
    drawing: "#9a3b8f",
    other: "var(--text-secondary)",
  }[kind];

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      aria-hidden="true"
    >
      <path
        d="M4.5 3.2A1.2 1.2 0 0 1 5.7 2h5.1l4.7 4.6v10.2a1.2 1.2 0 0 1-1.2 1.2H5.7a1.2 1.2 0 0 1-1.2-1.2Z"
        fill={color}
        opacity="0.16"
      />
      <path
        d="M4.5 3.2A1.2 1.2 0 0 1 5.7 2h5.1l4.7 4.6v10.2a1.2 1.2 0 0 1-1.2 1.2H5.7a1.2 1.2 0 0 1-1.2-1.2Z"
        stroke={color}
        strokeWidth="1.2"
      />
      <path d="M10.8 2v4.6h4.7" stroke={color} strokeWidth="1.2" />
      {kind === "spreadsheet" && (
        <path
          d="M7 10.5h6M7 13.5h6M10 9.5v5"
          stroke={color}
          strokeWidth="1.1"
        />
      )}
      {kind === "text" && (
        <path d="M7 10.5h6M7 13.5h4" stroke={color} strokeWidth="1.1" />
      )}
      {kind === "presentation" && (
        <rect
          x="7"
          y="10"
          width="6"
          height="4.2"
          rx="0.6"
          stroke={color}
          strokeWidth="1.1"
        />
      )}
      {kind === "drawing" && (
        <circle cx="10" cy="12.2" r="2.2" stroke={color} strokeWidth="1.1" />
      )}
    </svg>
  );
}

/** App logo used in the title bar and the tray tooltip area. */
export const AppLogo = ({ size = 16 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 20 20" fill="none" aria-hidden="true">
    <rect x="1.5" y="1.5" width="17" height="17" rx="4" fill="var(--accent)" />
    <path
      d="M6 6.5h8M6 10h8M6 13.5h5"
      stroke="var(--text-on-accent)"
      strokeWidth="1.5"
      strokeLinecap="round"
    />
  </svg>
);
