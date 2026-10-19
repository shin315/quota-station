// Which service an MCP server, a plugin or a connector is for, so its row
// can show that service's logo (components/logos.tsx). A small explicit list:
// a name, a package or a host names a service only when one of its words is
// on it, and a row that matches nothing keeps its plain look.

/** The services the app has a logo for. */
export type Service =
  | 'github'
  | 'notion'
  | 'figma'
  | 'slack'
  | 'linear'
  | 'supabase'
  | 'vercel'
  | 'cloudflare'
  | 'gmail'
  | 'google-drive'
  | 'google-calendar'
  | 'stripe'
  | 'atlassian'
  | 'asana'
  | 'canva'
  | 'playwright'
  | 'postgresql';

/** The words that name each service, as they show in names, packages and hosts. */
const WORDS: Record<Service, string[]> = {
  github: ['github', 'githubcopilot'],
  notion: ['notion', 'notionhq'],
  figma: ['figma'],
  slack: ['slack'],
  linear: ['linear'],
  supabase: ['supabase'],
  vercel: ['vercel'],
  cloudflare: ['cloudflare'],
  gmail: ['gmail'],
  'google-drive': ['google-drive', 'googledrive', 'gdrive'],
  'google-calendar': ['google-calendar', 'googlecalendar', 'gcal'],
  stripe: ['stripe'],
  atlassian: ['atlassian', 'jira', 'confluence'],
  asana: ['asana'],
  canva: ['canva'],
  playwright: ['playwright'],
  postgresql: ['postgres', 'postgresql'],
};

const BY_WORD = new Map(Object.entries(WORDS).flatMap(([s, words]) => words.map((w) => [w, s as Service] as const)));

/** One text's words, and each two in a row joined with and without a dash (`google-drive`, `googledrive`). */
function words(text: string): string[] {
  const one = text.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean);
  return [...one, ...one.slice(1).flatMap((w, i) => [one[i] + '-' + w, one[i] + w])];
}

/**
 * The service the first of `texts` that names one is for: a server's name,
 * then its package, then its host. `null` when none does.
 */
export function serviceOf(...texts: (string | null | undefined)[]): Service | null {
  for (const t of texts) {
    if (!t) continue;
    for (const w of words(t)) {
      const s = BY_WORD.get(w);
      if (s) return s;
    }
  }
  return null;
}

/** A plugin's name without its marketplace: `figma@claude-plugins` → `figma`. */
export const pluginName = (id: string): string => id.split('@')[0];
