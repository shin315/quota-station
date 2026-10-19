// Sidebar and header icons, drawn as in the prototype sketch (`ui.js` `IC`),
// SF Symbols-like.

const P = { width: 15, height: 15, viewBox: '0 0 16 16', 'aria-hidden': true } as const;
const S = { fill: 'none', stroke: 'currentColor', strokeWidth: 1.4 } as const;

/** The popover's theme button in a light look: switch to dark. */
export function MoonIcon() {
  return (
    <svg {...P}>
      <path d="M13.2 10.1A5.6 5.6 0 015.9 2.8a5.6 5.6 0 107.3 7.3z" {...S} strokeLinejoin="round" />
    </svg>
  );
}

/** The same in a dark look: switch to light. */
export function SunIcon() {
  return (
    <svg {...P}>
      <circle cx="8" cy="8" r="3" fill="currentColor" />
      <path
        d="M8 1.2v1.6M8 13.2v1.6M1.2 8h1.6M13.2 8h1.6M3.2 3.2l1.1 1.1M11.7 11.7l1.1 1.1M3.2 12.8l1.1-1.1M11.7 4.3l1.1-1.1"
        stroke="currentColor"
        strokeWidth="1.3"
        strokeLinecap="round"
      />
    </svg>
  );
}

/** Opens the main window: a box with an arrow out of it. */
export function OpenIcon() {
  return (
    <svg {...P}>
      <path d="M7 3H4.2A1.7 1.7 0 002.5 4.7v7.1a1.7 1.7 0 001.7 1.7h7.1a1.7 1.7 0 001.7-1.7V9" {...S} strokeLinecap="round" />
      <path d="M9.5 2.5h4v4M13.5 2.5L8 8" {...S} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function ChevronIcon() {
  return (
    <svg width="11" height="11" viewBox="0 0 12 12" aria-hidden="true">
      <path d="M4 2.5L7.5 6 4 9.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export const NAV_ICONS = {
  overview: (
    <svg {...P}>
      <rect x="2" y="2" width="5" height="5" rx="1.3" fill="currentColor" />
      <rect x="9" y="2" width="5" height="5" rx="1.3" fill="currentColor" opacity=".55" />
      <rect x="2" y="9" width="5" height="5" rx="1.3" fill="currentColor" opacity=".55" />
      <rect x="9" y="9" width="5" height="5" rx="1.3" fill="currentColor" />
    </svg>
  ),
  claude: (
    <svg {...P}>
      <path d="M8 1.8l1.5 4.7L14.2 8l-4.7 1.5L8 14.2 6.5 9.5 1.8 8l4.7-1.5z" fill="currentColor" />
    </svg>
  ),
  agy: (
    <svg {...P}>
      <path d="M8 2l5.5 11h-3L8 7.6 5.5 13h-3z" fill="currentColor" />
    </svg>
  ),
  codex: (
    <svg {...P}>
      <rect x="1.8" y="2.8" width="12.4" height="10.4" rx="2.2" {...S} />
      <path d="M4.6 6.2l2 1.8-2 1.8M8.2 10.2h3.2" {...S} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  ),
  rules: (
    <svg {...P}>
      <path d="M3 5h8M3 11h10" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      <circle cx="12" cy="5" r="1.8" fill="currentColor" />
      <circle cx="5" cy="11" r="1.8" fill="currentColor" />
    </svg>
  ),
  history: (
    <svg {...P}>
      <circle cx="8" cy="8" r="6" {...S} />
      <path d="M8 4.5V8l2.4 1.6" {...S} strokeLinecap="round" />
    </svg>
  ),
  alerts: (
    <svg {...P}>
      <path d="M4 11V7.5a4 4 0 018 0V11l1.2 1.3H2.8z" fill="currentColor" />
      <circle cx="8" cy="14" r="1.3" fill="currentColor" />
    </svg>
  ),
  firstmate: (
    <svg {...P}>
      <circle cx="8" cy="3.6" r="1.6" {...S} strokeWidth="1.3" />
      <path d="M8 5.2v8.3M5 8h6M2.8 10.5a5.2 5.2 0 0010.4 0" {...S} strokeWidth="1.3" strokeLinecap="round" />
    </svg>
  ),
  mcp: (
    <svg {...P}>
      <path d="M5.5 1.8v3M10.5 1.8v3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      <path d="M3.5 4.8h9v2.6a4.5 4.5 0 01-9 0z" fill="currentColor" />
      <path d="M8 11.9v2.4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  ),
  skills: (
    <svg {...P}>
      <path d="M9.2 1.6L3.6 9h3.8l-.9 5.4L12.4 7H8.6z" fill="currentColor" />
    </svg>
  ),
  plugins: (
    <svg {...P}>
      <path d="M3 4.5h3a1.6 1.6 0 113.2 0h2.8v2.8a1.6 1.6 0 110 3.2V13.5H3V10.5a1.6 1.6 0 100-3.2z" fill="currentColor" />
    </svg>
  ),
  instr: (
    <svg {...P}>
      <path d="M4 1.8h5.5L12.5 5v9.2H4z" {...S} strokeLinejoin="round" />
      <path d="M6 8h4.5M6 10.6h4.5" {...S} strokeWidth="1.3" strokeLinecap="round" />
    </svg>
  ),
  sets: (
    <svg {...P}>
      <path d="M8 2l6 3-6 3-6-3z" fill="currentColor" />
      <path d="M2 8l6 3 6-3M2 11l6 3 6-3" {...S} strokeLinejoin="round" />
    </svg>
  ),
  backup: (
    <svg {...P}>
      <rect x="1.8" y="2.5" width="12.4" height="3.4" rx="1" fill="currentColor" />
      <path d="M2.8 6.5h10.4v6.2a1 1 0 01-1 1H3.8a1 1 0 01-1-1z" {...S} />
      <path d="M6.4 9h3.2" {...S} strokeLinecap="round" />
    </svg>
  ),
  copy: (
    <svg {...P}>
      <rect x="1.5" y="3" width="5.5" height="4" rx=".8" {...S} strokeWidth="1.3" />
      <rect x="9" y="9" width="5.5" height="4" rx=".8" {...S} strokeWidth="1.3" />
      <path d="M9.5 4.2h2.6a1.4 1.4 0 011.4 1.4v1.6M6.5 11.8H3.9a1.4 1.4 0 01-1.4-1.4V8.8" {...S} strokeWidth="1.3" strokeLinecap="round" />
    </svg>
  ),
  settings: (
    <svg {...P}>
      <circle cx="8" cy="8" r="2.3" {...S} />
      <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2" {...S} strokeLinecap="round" />
    </svg>
  ),
};

/** A server put away in the app's store. */
export const ARCHIVE_ICON = (
  <svg {...P}>
    <rect x="1.8" y="2.5" width="12.4" height="3.4" rx="1" fill="currentColor" />
    <path d="M2.8 6.5h10.4v6.2a1 1 0 01-1 1H3.8a1 1 0 01-1-1z" {...S} />
    <path d="M6.4 9h3.2" {...S} strokeLinecap="round" />
  </svg>
);

/** What an agent or a plugin brings itself. */
export const PUZZLE_ICON = (
  <svg {...P}>
    <path d="M3 4.5h3a1.6 1.6 0 113.2 0h2.8v2.8a1.6 1.6 0 110 3.2V13.5H3V10.5a1.6 1.6 0 100-3.2z" fill="currentColor" />
  </svg>
);

/** The small lock beside a server that has a secret. */
export const LOCK_ICON = (
  <svg width="11" height="11" viewBox="0 0 16 16" aria-hidden="true">
    <rect x="3" y="7" width="10" height="7.5" rx="1.6" fill="currentColor" />
    <path d="M5.2 7V5.2a2.8 2.8 0 015.6 0V7" fill="none" stroke="currentColor" strokeWidth="1.6" />
  </svg>
);

/** The big plug of the empty MCP table. */
export const PLUG_BIG = (
  <svg width="22" height="22" viewBox="0 0 16 16" aria-hidden="true">
    <path d="M5.5 1.8v3M10.5 1.8v3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    <path d="M3.5 4.8h9v2.6a4.5 4.5 0 01-9 0z" fill="currentColor" />
    <path d="M8 11.9v2.4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
  </svg>
);

/** The two Macs of the empty Chép sang Mac khác tab. */
export const SYNC_BIG = (
  <svg width="22" height="22" viewBox="0 0 16 16" aria-hidden="true">
    <rect x="1.5" y="3" width="5.5" height="4" rx=".8" fill="none" stroke="currentColor" strokeWidth="1.3" />
    <rect x="9" y="9" width="5.5" height="4" rx=".8" fill="none" stroke="currentColor" strokeWidth="1.3" />
    <path d="M9.5 4.2h2.6a1.4 1.4 0 011.4 1.4v1.6M6.5 11.8H3.9a1.4 1.4 0 01-1.4-1.4V8.8" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
  </svg>
);

/** The big box of the empty Sao lưu tab. */
export const ARCHIVE_BIG = (
  <svg width="22" height="22" viewBox="0 0 16 16" aria-hidden="true">
    <rect x="1.8" y="2.5" width="12.4" height="3.4" rx="1" fill="currentColor" />
    <path d="M2.8 6.5h10.4v6.2a1 1 0 01-1 1H3.8a1 1 0 01-1-1z" fill="none" stroke="currentColor" strokeWidth="1.4" />
    <path d="M6.4 9h3.2" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
  </svg>
);

/** Skills installed by hand, gathered into one place. */
export const STACK_ICON = (
  <svg {...P}>
    <path d="M8 2l6 3-6 3-6-3z" fill="currentColor" />
    <path d="M2 8l6 3 6-3M2 11l6 3 6-3" {...S} strokeLinejoin="round" />
  </svg>
);

/** The big spark of the empty Skills table. */
export const SPARK_BIG = (
  <svg width="22" height="22" viewBox="0 0 16 16" aria-hidden="true">
    <path d="M9.2 1.6L3.6 9h3.8l-.9 5.4L12.4 7H8.6z" fill="currentColor" />
  </svg>
);

/** The big stack of the empty Bộ cấu hình tab. */
export const STACK_BIG = (
  <svg width="22" height="22" viewBox="0 0 16 16" aria-hidden="true">
    <path d="M8 2l6 3-6 3-6-3z" fill="currentColor" />
    <path d="M2 8l6 3 6-3M2 11l6 3 6-3" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round" />
  </svg>
);
