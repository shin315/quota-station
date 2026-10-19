//! A fake `claude` for the MCP tests: it keeps Claude Code's list as one
//! file per server and writes `~/.claude.json` from them, with keys of its
//! own around `mcpServers` (one of them a sentinel the app must never read).

use std::path::{Path, PathBuf};

use super::script;

/// A fake `claude`: `mcp add-json` and `mcp remove` keep one file per
/// server, and every run writes `~/.claude.json` again from them, with
/// keys of its own around `mcpServers` as Claude Code's file has.
pub fn fake_claude(dir: &Path, home: &Path, sentinel: &str) -> PathBuf {
    std::fs::create_dir_all(dir.join("servers")).unwrap();
    let body = r#"d='@DIR@'
for a in "$@"; do printf '%s\037' "$a" >> "$d/calls"; done
printf '\n' >> "$d/calls"
if [ -e "$d/fail" ]; then echo "could not add $3: $4" >&2; exit 1; fi
case "$1 $2" in
  "mcp add-json")
    [ -e "$d/servers/$3" ] && { echo "MCP server $3 already exists in user config" >&2; exit 1; }
    printf '%s' "$4" > "$d/servers/$3" ;;
  "mcp remove")
    [ -e "$d/servers/$3" ] || { echo "No user-scoped MCP server found with name: $3" >&2; exit 1; }
    rm "$d/servers/$3" ;;
esac
{
  printf '{\n  "numStartups": 7,\n  "oauthAccount": { "emailAddress": "someone@example.com" },\n  "primaryApiKey": "@SENTINEL@",\n  "mcpServers": {'
  first=1
  for f in "$d"/servers/*; do
    [ -e "$f" ] || continue
    [ "$first" = 1 ] || printf ','
    first=0
    printf '\n    "%s": ' "$(basename "$f")"
    cat "$f"
  done
  printf '\n  },\n  "projects": { "/work/app": { "mcpServers": { "local-only": { "command": "x" } } } }\n}\n'
} > '@HOME@/.claude.json'
"#
    .replace("@DIR@", &dir.display().to_string())
    .replace("@HOME@", &home.display().to_string())
    .replace("@SENTINEL@", sentinel);
    script(&dir.join("bin/claude"), &body)
}
