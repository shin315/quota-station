// Sidebar and header icons, drawn as in the prototype sketch (`ui.js` `IC`),
// SF Symbols-like.

const P = { width: 15, height: 15, viewBox: '0 0 16 16', 'aria-hidden': true } as const;
const S = { fill: 'none', stroke: 'currentColor', strokeWidth: 1.4 } as const;

export function WindowIcon() {
  return (
    <svg {...P}>
      <rect x="2" y="3" width="12" height="10" rx="2" {...S} />
      <path d="M6 3v10" {...S} />
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
  settings: (
    <svg {...P}>
      <circle cx="8" cy="8" r="2.3" {...S} />
      <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2" {...S} strokeLinecap="round" />
    </svg>
  ),
};
