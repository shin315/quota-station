# Quota Station, feature by feature

The [README](../README.md) gives the short tour; this page describes each part
of the app in full. Labels are the English interface's, with the Vietnamese
label in parentheses where it helps. What the app reads, runs and writes for
each part is in [how-it-works.md](how-it-works.md).

Some parts belong to a build or a setting:

- **Advanced** (`Nâng cao`): the tools made for the author's own Macs. They are
  compiled only with the `author-tools` feature, which `scripts/install.sh`
  turns on and a release build leaves out, and shown only while `Settings ›
  Advanced › Show advanced features` is on. A new install starts with it off,
  no rotation rules and accounts named `Account N` (`Tài khoản N`); a Mac that
  already had the app gets it on, with Firstmate. Parts marked *(Advanced)*
  below need it.
- **Antigravity accounts**: logging in and switching several Antigravity
  accounts, compiled only with the `antigravity-accounts` feature (CC BY-NC-SA
  4.0). Without it the Antigravity tab is `read-only` and offers a card about
  the multi-account build. Parts marked *(Antigravity accounts)* need it.

Contents: [the menu bar](#the-menu-bar-item) ·
[the popover](#the-popover) · [the main window](#the-main-window) ·
[names](#names) · [Claude](#claude) · [Antigravity](#antigravity) ·
[Codex](#codex) · [rotation rules](#rotation-rules-advanced) ·
[history](#history) · [notifications](#notifications) ·
[Firstmate](#firstmate-advanced) · [MCP](#mcp) ·
[Skills, Plugins and Instruction files](#skills-plugins-and-instruction-files) ·
[Config sets](#config-sets) · [Agent updates](#agent-updates) ·
[Backup](#backup-and-restore) · [Copy to another Mac](#copy-to-another-mac-advanced) ·
[Variables that override a sign-in](#variables-that-override-a-sign-in) ·
[Stats](#stats) · [Settings](#settings) · [the first run](#the-first-run) ·
[app updates](#app-updates)

## The menu bar item

The C04 circular gauge, a template image with a rim bead and liquid fill. Next
to it is the account in use and its share used for one service, such as
`lan.acme1 74%`. The service is Claude by default; `Settings › Display ›
Service in the menu bar` and `Menu bar service` in the right-click menu pick
Gemini (Antigravity's Gemini models) or Codex instead, on each Mac.

The number is the account's five-hour window by default; `Number in the menu
bar` in Settings and `Menu bar quota window` in the right-click menu pick the
week, Claude's Fable week, or the window used the most (a tie naming the longer
window) instead. A service without the window picked shows its five-hour
window, and the window used the most without that either.

The name is the part of the email before `@`, cut to 12 characters with `…`, or
the account's place or number while names are hidden. Codex shows `ChatGPT` and
that account's number on its ChatGPT sign-in, and only the provider's name when
it goes through a provider of its own, which reports no quota. Without a number
(no read yet, a read that failed, a window waiting for its number after a reset)
the title is the name and `–`; only the window shown needs a number, except for
the window used the most, which needs every window. A Mac without Antigravity,
or without Codex, follows Claude. The gauge fills with the same service and
window as the title; without a number its reservoir is empty. While automatic
switching is paused, two small pause bars sit inside the gauge; an exhausted
Claude rotation draws a warning triangle. The tooltip and the VoiceOver label
give every service in words, the window shown and its reset included (this
label is in Vietnamese for now).

**Right-click menu**: `Open Quota Station`, `Refresh`, `Menu bar service`
(Claude, Gemini or Codex, ticked; a service this Mac lacks is greyed out),
`Menu bar quota window` (`5 hours`, `7 days`, `Fable week` or `Tightest`,
ticked; `Fable week` is greyed out unless the title follows Claude),
`Settings…` and `Quit Quota Station`. The menu follows the app's language.

## The popover

A left click opens the popover. One header line has this Mac's name (how old
the numbers are shows only when they are not fresh) and three buttons: the sun
or moon turns the look light or dark for both windows, the bell (with the red
count of notifications not read yet) opens the main window with its
notifications panel open, and `Open Quota Station` opens the main window on
`Overview`.

Then a section per service, `CLAUDE`, `ANTIGRAVITY` and `CODEX`, each opening
on its account in use, marked `In use`, with every window: a Claude Code
account's `5-hour`, `Week` and `Fable` (a Team seat with no week has no `Week`
line), an Antigravity account's `Gemini` and `Claude/GPT` columns with their
five hours and week, each as the share **used** (the bar fills as the quota
goes; amber from 70 %, red from 90 %) with when it resets. Codex shows the
ChatGPT account in use with its `5-hour` and `Week`, or the custom provider
(`Custom API · reports no quota`). Right under it, one line (`3 other accounts`)
opens the other accounts, in the main window's order, each with its shorter
lines (Antigravity's `Gemini` and `Claude/GPT`, each the tighter of that group's
two windows). A section's header folds it to the header alone, which then says
the account in use and its number; folds and open lists stay as they were the
next time the popover opens, and every line is a button with `aria-expanded`.

The popover writes no notes on the accounts: an old number is drawn grey with
its age on hover, and the rotation's notes stay in the main window. Only an
account that must sign in again says so, in one short line (and on the closed
line, `· 1 to sign in again`). Every window, the organization and the hours
stay in the accounts' spoken labels and in the main window. It also has
banners for stale numbers, an unavailable source, numbers waiting after a
reset, an exhausted rotation and missing Keychain permission; the first read
says `Reading quota for the first time…`, and a Mac with no agent at all says
so with `Check again`. Every other Claude account is a button that opens the
switch sheet, and so is every other Antigravity account the app signs in
*(Antigravity accounts)*, and every other Codex account.

The footer, outside the scrolling part, explains the numbers (`% used · time
until reset`, with `Details`) and says when a new version of the app is out.
*(Advanced)* It also has the `Claude auto-switch` switch; the line that says why
Claude is on its account shows only when there is something to act on (a pause
or hold, waiting for quota, a switch coming, the switcher missing or failing),
and the other Macs' line only when one of them is behind.

## The main window

One sheet of glass, like the popover: macOS draws the popover's material behind
the whole window, the sidebar sits on it with no panel of its own, and each
group is a frosted panel (solid when Reduce transparency is on). Every page's
title row ends with the bell, beside the Mac's name, how fresh the numbers are
and `Refresh`. The same numbers as the popover, from the same read.

The sidebar:

- `Overview` and `Stats`
- **Accounts**: `Claude`, `Antigravity`, `Codex`
- **Automation** (with Advanced; **Activity** otherwise): `Rotation rules`
  *(Advanced)*, `History`, `Firstmate` *(Advanced)*
- **Agent**: `MCP`, `Skills`, `Plugins`, `Instruction files`, `Config sets`,
  `Updates` (Agent updates; not on Windows)
- **Backup**: `Backup`, `Copy to another Mac` *(Advanced)*
- `Settings`, at the foot, with the Mac's name under it (and, with Advanced,
  whether the other Macs have the rotation rules).

`Overview` has a card per service: the Claude accounts in a table with fixed
columns (`5-hour · Week · Fable week`), the Antigravity accounts (`5-hour` and
`Week` under `Gemini` and `Claude/GPT`), and the Codex accounts (`5-hour` and
`Week`, and the custom providers), each the share used, with `Switch` on each
account not in use and `In use` (a dot and words, nothing to press) on the one
in use. *(Advanced)* The Claude card's header has `Claude auto-switch` and its
one line, when there is more than one account. A banner at the foot says when a
new version of the app is out.

## Names

Accounts go by the part of their email before `@` when every account shares
one domain (the whole email otherwise, and always on hover), with the
organization of a Team seat or `Personal · 08–23` under it, unless a nickname
was given. Turning off `Show emails` (`Hiện email`) in `Settings › Display`
names them by place instead, for screenshots, and then no email or
organization reaches the windows at all. The place names (`Team A`,
`Personal`, `Ultra 1`) come with Advanced; without it accounts are `Account N`
and `Antigravity N`.

## Claude

**The `Claude` tab** lists the accounts (`Accounts`, with `Add account`), with
`Switch` on each account not in use and `In use` on the one in use; an old
reading gets a chip by the name. `Add account` signs one in with the browser,
takes the account Claude Code is signed in to, takes a pasted setup-token, or
imports claude-swap's accounts (see [how-it-works.md](how-it-works.md#the-claude-accounts)).
Each row's `⋯` menu has `Details and nickname…`, pausing automatic switching
for that account (or putting it back), `Sign in again…` and `Remove from this
Mac…`. A variable that overrides Claude Code's sign-in puts a red banner at the
top of the tab ([below](#variables-that-override-a-sign-in)). While an older
switcher (`com.cswap.prefer-team` or `com.cswap.auto`) is still loaded, the tab
says so with `How to turn it off`, and a guide covers moving from claude-swap.

*(Advanced)* Below the accounts, one card has two halves: `Claude auto-switch`,
with its line (the rotation's decision from a dry run, or `Paused until HH:MM:
<reason>` while a pause holds) and `Pause` (`1 hour`, `Until 18:00` or `Until I
turn it back on`; while paused it resumes at once, and the banner offers `One
more hour`); and `Background check`, which says whether launchd runs the
rotation every 3 minutes, even when the app is closed. Without Advanced, an
`Auto-switch with Quota Station` card shows only while the rotation runs or an
older switcher is loaded. Banners warn when a switcher ignores the pause flag
and when launchd does not run the rotation.

**Switching.** The confirmation sheet names the account, its allowances, and
how many Claude Code sessions running on this Mac will rebuild their prompt
cache on their next message, which costs quota. *(Advanced)* It offers a hold so
automatic switching does not switch straight back: `30 minutes`, `1 hour` (the
default), `Until reset` (the account's next five-hour reset) or `Let it
auto-switch`; outside the personal account's hours it warns that the rotation
will move away from it. Confirming switches with the app's own switch, writes
the pause flag when a hold was chosen (a hold never shortens a pause that is
already on for longer), waits 5 seconds, reads the active account again without
the app's cache and reports what it finds: switched, switched back by the
rotation (or by another switch), not yet confirmed, or why nothing changed
(Claude Code held its login locks, the account needs a new sign-in, Claude Code
is on a sign-in the app does not keep, or an older switcher is still loaded).

## Antigravity

**The `Antigravity` tab** lists the accounts with the same columns as Overview.
*(Antigravity accounts)* Each account the app signs in has `Switch` and `Sign
out`, and the card's header has `Add account`; the accounts the app signs in
that the row does not list yet stay in a short list under the table. Each such
account shows `reading…` while the app reads its quota and `read failed: …`
when that read failed. Opening the tab reads every account. A Mac without an
account list, or without Antigravity (`No Antigravity on this Mac`), says so.
Logins, the quota reads and the switch are described in
[how-it-works.md](how-it-works.md#antigravity-logins-quota-and-switching).

*(Antigravity accounts)* For six hours after a switch, a reading that shows the
old account back in use is counted as an overwrite: the tab shows the banner
`Antigravity switch overwritten` with `Switch again` and `Cancel`, and a
notification comes. The popover has no banner for it.

## Codex

Codex can go through the official ChatGPT sign-in or through a custom API
provider that its `config.toml` describes (a `[model_providers.<id>]` table
with its own key). The app switches between them and keeps the ChatGPT sign-in
in place, so Codex App's plugins and remote control keep working while Codex
goes through the custom provider.

**Accounts.** The `Codex` tab's `Accounts` card lists every ChatGPT account the
app keeps, then every provider table of `config.toml`, each with its `5-hour`
and `Week` (a provider reports no quota; its line says how fast it answers),
`Switch` and a `⋯` menu (`Details and nickname…`, `Sign in again…`, `Remove
from this Mac…`, and for a provider `Remove from config.toml…`). `Add account`
signs a ChatGPT account in with the browser (`codex login` run with a new
folder of its own as `CODEX_HOME`, so the Codex in use stays signed in; it
waits up to 10 minutes), or with a device code (`codex login --device-auth`,
offered when port 1455 is busy), or adds a custom provider (name, address, key,
`Check`, then a preview of the table it appends). Each ChatGPT account waits in
its own folder in the app's folder (`codex/accounts/<n>/`, owner only, out of
Time Machine); `Switch` moves its `auth.json` into the Codex folder and the one
there back to its own folder, never copying one, and drops `model_provider`
when Codex went through a provider. The sign-in Codex already has joins the
list as it is. An account whose token ran out while it waited shows its last
numbers, with their age, until it is used. Nicknames are the app's own;
removing a ChatGPT account deletes its folder, after `codex logout` in it when
ticked.

**Importing.** `Add account › Import a sign-in file (JSON)` takes Codex's
`auth.json` or CLIProxyAPI's `codex-*.json`, picked in the system's file picker
(at most 50 files, each up to 1 MiB) or pasted, shows who each sign-in is and
what importing it does (new, renews an account that must sign in again, already
here, refresh token only), and never renews or sends one. The sign-ins read are
held in memory for at most 10 minutes, and the picked files can be deleted
after the import.

**Wake ChatGPT** (`Tự đánh thức ChatGPT`, off until turned on, shown once the
app keeps a ChatGPT account) runs `codex exec "hi"` in each chosen account's
folder, `Every day` at its wake times (07:00 and 12:00, say; five hours apart
at least, four times at most) or `On reset` as a five-hour window resets, so the
window starts early; `Run now` runs it at once. It can also wake an account
whose token ran out while it waited, so Codex renews it, and `Keep sign-ins`
(on by default) wakes a chosen account that is waiting at most once every 12
hours, so its sign-in stays fresh. Every
run it starts keeps to one guard: five hours since the account's last run, five
runs in 24 hours, nothing while its window is open or its week is used up; a
time that falls inside it is skipped with its reason, never moved. A Mac asleep
at a wake time runs it up to three hours late; a time missed beyond that is not
made up. It rests on one line under the accounts and opens in place on
`Settings`. OpenAI says nothing of requests sent on a schedule: use it on your
own accounts only.

**Switching.** Pressing an account or provider never switches by itself: it
opens a sheet with the facts of the switch (what changes, the sessions already
running, the background process, the ChatGPT sign-in, the one file written and
its backup), plus any risk of the moment, and a `Restart the Codex background
process after switching` box (ticked by default for a switch between ChatGPT
accounts, off for a switch to a custom provider). A switch the app cannot make
safely is refused with the reason under the row: no custom provider table, a
table without a key of its own (the ChatGPT sign-in would be sent to the
provider), the placeholder key CC Switch leaves in a table it does not use, a
profile that picks its own provider, or a table written inline. The custom
provider's name and address come from the file, and its key from the file or
from the owner's paste; nothing is built in.

**The key.** While Codex is on the custom provider, a line under the table says
how its key is given, with `Paste key…`, `Change key…` or `How to set the key…`.
`Paste key…` puts the owner's key in the table as `experimental_bearer_token`,
the way CC Switch's direct switch keeps it. The key goes in a password field; a
paste with `Bearer `, quotes or the `NAME=` it was copied with gives the key
alone. When the table still points at CC Switch's local proxy
(`127.0.0.1:15721`), the same sheet takes the provider's real address. A
preview shows the lines that change, the key only as `sk-…abcd`; the write
keeps a backup with the key left out and refuses a file changed since the
preview. `Undo` puts CC Switch's placeholder (or no key) and the old address
back while the file is the one the write left; a key written over another key
cannot be undone, since the app never keeps a key, and the preview says so. A
table that reads its key from `env_key` gets the steps to set the variable, and
nothing is written. CC Switch's `model_catalog_json` is only reported: Codex
does not start without that file. The key never reaches a window, the log,
`History`, a backup, the copy to another Mac or a configuration set.

**Plugins.** `Use plugins with a Custom API` is a checkbox, ticked by default
and kept in the settings. Ticked, a switch to the custom provider keeps the
ChatGPT sign-in in use, as above. Unticked, it writes
`requires_openai_auth = false`: Codex leaves the ChatGPT sign-in alone under
the custom provider, and the plugins' parts that need it (apps, connectors, the
online catalogue) are off. Changing the box while Codex is on the custom
provider offers `Apply now…`, with the same confirmation, backup and
reopen-Codex-App note as a switch, and rewrites only that one key.

**The rest of the tab.** `Codex on this Mac` is one line (the CLI's version,
terminal sessions, Codex App, the MCP count) with `Details…` (the technical
detail: `auth.json`, the address, how the key is given, `requires_openai_auth`,
the file written and its backups, the sessions that can be resumed by provider,
the recent switches, and `Restart background process…`) and `Open MCP`. While
CC Switch still runs and manages Codex, a banner says to switch Codex in one
place only: CC Switch's next switch overwrites the provider choice. A change
made to `config.toml` outside the app shows as a banner with `See changes` and
`Got it`; the app never writes it back. Every switch, and every change seen
from outside, goes to `History` under `Codex`. Opening or installing the app
changes nothing in Codex's files: every write follows a confirmation.

**How the file is written.** For a provider switch only `model_provider` (set to
the custom provider's id, or removed for the ChatGPT sign-in) and, in the custom
provider's table, `requires_openai_auth` change. That key is on when the
table's key comes from an environment variable or a bearer token, a ChatGPT
sign-in exists and the plugins box is ticked, and off for a key sent in its own
header, as CC Switch decides it. `Paste key` writes only the table's
`experimental_bearer_token` and, when asked, its `base_url`. Every other byte
stays, comments included; a switch never writes `[mcp_servers]` or `profiles`.
`auth.json` is never edited: an account switch only moves whole files between
the Codex folder and the account folders. Before writing, the app checks the
file is still the one the sheet was built from (a SHA-256), keeps a backup with
every key in it replaced by `<redacted>`, writes a temporary file and renames
it, and reads the result. The Codex folder is `~/.codex` unless `Settings ›
Services › Codex folder` says otherwise.

## Rotation rules *(Advanced)*

The editor shows the rotation file's six keys: the Team accounts in the
rotation (chips; the number is the order), the personal account and its hours,
the two thresholds (sliders), and when an account leaves the rotation. It marks
what changed, previews what the switcher would decide with the edited rules
(its `--dry-run` pointed at a draft copy), and saves with one button named for
the Macs it reaches (`Save for all three Macs` with two other Macs set, `Save
for this Mac` with none).

A save first checks the rules as quota-axi reads the file (all six keys, each
in its form; a `rotation_…` reason otherwise, and nothing is written or sent),
since Firstmate's quota-axi reads the same file. It rewrites the six keys in
place and keeps every other readable line (a line quota-axi cannot read stays
only in the backup), puts a `# updated = <time> by <machine>` comment at the
top, writes through a temporary file and a rename, and keeps the file before as
`rotation.bak-<time>` (the newest 10 stay). Then the file goes to each other
Mac over SSH, the same way, and is read back to compare. The save bar has one
line per Mac: `received 14:30`, `unreachable · retrying at 14:35`, `has a newer
copy`. A Mac that could not take it is tried again every 5 minutes, also after
a restart. When another Mac holds a newer copy (a later `updated`), or one
edited by hand there (its content differs from its stamp, or it has no
`updated` line), nothing is overwritten: the write itself checks the file is
still the one just read, and the tab offers `Compare the two copies of the
rules` or `Keep this Mac's copy` (stamped again, so it is the newest). When a
newer copy lands on this Mac during an edit, the editor says `The copy on <Mac>
is newer. Reload?` and saves over it only when told to. Leaving the tab,
closing the window or quitting from the menu bar with unsaved changes asks
first.

## History

Every switch on this Mac, newest first, a day at a time (at most 200 shown),
filtered by `All`, `Claude`, `Antigravity` or `Codex`. Each line has its time, a
tag, the accounts before and after, why, and where it comes from: the app (a
switch made in this app), the switcher (a `switch to slot N` or `primed slot N`
line in the switcher's log, read from its last 64 KB) or `observed` (the
account in use changed between two reads and neither the app nor the switcher
made it, or Antigravity.app wrote the old sign-in back). The app keeps its own
lines, at most 500, in `history/history.jsonl`; the Antigravity and Codex
switches come from the app's own switch logs, never from Cockpit's switch
history.

## Notifications

After every read the app checks its rules and sends each new notification once,
as a macOS card with buttons; the same card stays under the bell at the right
of every page's title row, with the same buttons. The bell carries a red count
of the notifications that came since its panel was last opened (none at zero,
`9+` past nine), and opening the panel reads them. The panel lists the recent
ones a day at a time, the newest five first and the rest behind `See all`, and
ends with `Notification settings`, which opens `Settings › Notifications`; Esc
or a click outside closes it. A macOS card pressed on itself, and the popover's
bell, open the main window with the panel open. A button pressed on either
counts once, and the card then says what was chosen and how it went. Each
notification has a key, so the same event never comes twice. The cards name
accounts as the app does, never by email.

The cards' titles, words and buttons are written by the Rust side and are in
Vietnamese for now, whatever the app's language; `Settings › Notifications`
lists the kinds in both languages. The kinds:

1. **Claude quota running out, and back** (`Hạn mức Claude trong vòng sắp cạn`
   and `Claude đã có lại hạn mức`): Firstmate's rule on the accounts' numbers.
   The rotation's accounts that have not left it (the personal one only inside
   its hours) count; the pool is dry when the account in use has 10% or less
   left and no other has 15% or more (both set in `Settings › Notifications ›
   Running-out thresholds`; without Advanced every account counts and the title
   is `Hạn mức Claude sắp cạn`). Where the app keeps no Claude account it reads
   the rotation file. The card names the account in use and its share used, and
   when an account resets, and offers `Sang <account> (đã dùng N%)` when a
   better account exists, which opens the switch sheet, and `Bỏ qua`.
2. **An account is about to leave the rotation** *(Advanced)* (`<account> rời
   vòng lúc HH:MM`): 30 minutes before an account leaves the rotation (`expire`
   in the rules). For the account in use it offers `Ổn` and `Chuyển ngay` (the
   switch sheet).
3. **An account needs to sign in again** (`<account> cần đăng nhập lại`): the
   usage read reports an error only a new sign-in mends (`invalid_grant`,
   `no_refresh_token`, `no-access-token`, `relogin_required`, a relogin), with
   `Nhắc ngày mai` (asks again at 09:00 the next day) and `Đã biết`. Antigravity
   accounts that need a new sign-in, that Google blocked, or that need a
   verification (`<account> cần xác minh`) get their own card, with `Đăng nhập
   lại` (the `Antigravity` tab) or `Mở trang xác minh`.
4. **Antigravity went back to the old account** *(Antigravity accounts)*
   (`Antigravity đổi về tài khoản cũ`): the six-hour watch after an Antigravity
   switch saw the old account back. While it has happened at most twice the
   card offers `Thoát app rồi chuyển lại` and `Chuyển lại` (only `Chuyển lại`
   while Antigravity.app is not running; they switch back at once, as the
   sheet's buttons do) and `Bỏ qua`; after that it only says so.
5. **Firstmate is waiting for Claude quota** *(Advanced)* (`Firstmate đang chờ
   hạn mức Claude`): Firstmate's `quota-pool.last`, written within the last 30
   minutes, says `pool=dry`. Off by default.

The bell also gets, in the app only unless said otherwise: skills with a newer
version (a macOS card only if `Show skill updates as macOS notifications` is
on), each variable that overrides a sign-in (sent again only when it is set
somewhere new), and agent tools with a new version (from Agent updates).

`Send a test` sends a sample of each kind, marked as a test, whose buttons
change nothing. A Mac where notifications are off for the app says so, with
`Open System Settings`; the notifications still come to the bell. macOS asks
once for leave to show cards, the first time the app runs. Only the packaged
`.app` can post cards: `tauri dev` keeps the notifications under the bell. What
was sent, answered and snoozed is kept in `alerts/alerts.json` (the newest 30
cards).

## Firstmate *(Advanced)*

Whether Firstmate holds new Claude work, from the line its quota-pool watch
writes, `quota-pool.last`, at the path set in `Settings › Advanced` (nothing is
built in, and only a file of that name is taken): the account in use, what each
account has left, the next reset, whether Firstmate's quota-axi has `+cswap`
and whether it could read claude-swap, and when the file was written. A file
older than 30 minutes says Firstmate may not be running, and a Mac without the
file says so. Only the keys Firstmate writes are read.

## MCP

The `MCP` tab puts every MCP server of Claude Code, Codex and Antigravity in one
table: a row per server, with the line `on this Mac · <command>` or `over the
network · <host>`, the service's logo before the name when its name, package or
host names a service the app knows (GitHub, Notion, Figma, Slack, Linear and
others; the Plugins tab's plugins and the MCP an agent or a plugin brings get
one the same way), a small lock when it has a key, and chips only when something
is the matter (`key missing`, `command not found`, `differs between agents`);
and a column of macOS checkboxes per agent, headed by the agent's own logo.

A tick is not written at once: the box turns light blue with an amber dot and
the bar under the table lists the changes not written yet, so a tick taken back
drops out. The logo at a column's head opens its menu: `Turn all on`, `Turn all
off` (both skip a box that cannot be ticked) and discarding the changes not
written yet. `Preview and write…` shows one preview for the whole batch, by
agent: the lines each file gains and loses, the `claude mcp` commands for Claude
Code, the backups and what to restart. Each file is written once, and the line
after it offers `Undo` (the whole batch back) and the backups. `Add MCP` fills
one server in for every agent at once, by hand or by pasting the JSON or TOML a
README gives; `Edit` first compares a server that differs between agents (its
keys only as `the same` or `different`); `Delete` turns it off everywhere and
keeps it in the app's store to turn on again, or removes it for good. Under the
table, one line each: the servers put away (`Turn back on…`, `Delete for
good`), the ones Codex App and plugins bring themselves (shown, never edited),
and the backups.

Keys never show. A key typed or pasted goes to the app's own Keychain items,
with a note of where the owner keeps it (a password manager's item, say); the
preview masks every key; a server short of its key keeps its boxes locked until
`Paste key`. Codex is written in its `[mcp_servers.<name>]` tables
(`enabled = false` turns one off), Antigravity in
`~/.gemini/config/mcp_config.json` (`"disabled": true`), and Claude Code only
through `claude mcp add-json`/`remove --scope user`, which writes
`~/.claude.json` itself; Claude Code has no off, so the app keeps a server
turned off there and puts it back when it is ticked again. A file that does not
parse is shown and never written, and a file changed after the preview stops
the batch. See [`SECURITY.md`](../SECURITY.md) for what is kept where.

## Skills, Plugins and Instruction files

**Skills.** The `Skills` tab shows every skill of Claude Code, Codex and
Antigravity in the MCP tab's table, a checkbox per agent, filtered by `All`,
`From GitHub`, `By hand` and `Update available`. Skills live once, in the shared
folder `npx skills` installs into (`~/.agents/skills`): a tick for Claude Code
or Antigravity is a link in that agent's folder, and for Codex, which reads the
shared folder itself, an `enabled = false` entry under `[[skills.config]]` in
`config.toml`. Ticks go through the same pending bar, preview, backups and
`Undo` as the MCP tab, and turning a skill off never removes a folder; `Remove`
removes one. `Install from GitHub` takes a repository, lists its skills (with
`Read SKILL.md` to read one first), lets the owner pick the agents and shows the
`npx skills add` command before it runs; nothing in a skill runs on install.
Skills installed by hand in one agent's folder can be gathered into the shared
folder with `Gather…`; a link to a folder that is gone gets `Fix link`; skills
Codex or a plugin brings are listed and left alone. A Mac without `npx` gets a
banner.

Once a day (`Settings › General › Check for skill updates`) the app asks
GitHub, one question per repository in the skill lock file, whether a skill
has a newer version, and says so in a card above the table (`2 skills have
updates`) and under the bell; it never updates on its own. `Check updates` in
the header checks now. `Update` shows what changed, warns when the skill was
edited on this Mac, keeps a copy and keeps every agent's box; `Skip this
version` waits for the next version. `Update all (N)` in the card, and `Update
the set` on each repository's line in it, do the same for several skills in one
preview: each skill with its versions, the ones edited on this Mac left
unticked, a copy of every folder kept before the first update, then one update
at a time with the one at hand shown, and a line saying which updated and which
failed, and why.

**Plugins.** The `Plugins` tab lists the plugins `claude plugin list` and `codex
plugin list` report, what each brings and the marketplaces, and the agents' own
commands a restore would run; it changes nothing.

**Instruction files.** The `Instruction files` tab lists CLAUDE.md, the rules
under `~/.claude/rules`, AGENTS.md, GEMINI.md and any file added with `Add
file…`, when each changed, how it stands against the newest backup pack (with
a line-by-line `Compare`) and whether it goes into the pack. The app never
edits one, and a file with a line shaped like a key stays out of every pack,
the value masked when it is shown, unless the owner marks the line `Not a key`.
See [`SECURITY.md`](../SECURITY.md#skills-plugins-and-instruction-files-m11).

## Config sets

A set is named after a project (`Web app`, say) and says, for each MCP server
and skill, `On`, `Off` or `No change`, and for Codex `Keep as is` or a side of
the Codex switch. It applies to the whole Mac, every project and every session
opened after; nothing is written into a project's folder. `New set` lists every
server and skill of the MCP and Skills tables; `Take from the current setup`
sets `On` for an entry on in every agent that has it, `Off` for one off in all
of them and `No change` for the rest, so the set changes nothing applied
straight away. `Apply…` reads the files now and lists, agent by agent, what
changes: a server for every agent that has it (a box that is locked, or an
entry this Mac has not got, is skipped and said), a skill for every agent whose
box can be ticked; it says how many Claude Code and Codex sessions run now,
which keep the old configuration until they are opened again, and `See each
line to be written` shows the MCP and Skills tabs' own previews. `Apply set`
writes exactly that through the MCP and Skills tabs' checked batches: each file
read again, backed up without secrets, written once, only the set's entries
changed, and nothing at all when a file changed since the preview. A set that
wants Codex on the other side then opens the Codex switch's own sheet as step
2; the set never changes Codex's provider itself, and `Cancel` leaves Codex as
it is, with `Switch Codex` on the line to open the sheet again later.

`In use` names the set applied last and says how many entries differ (`1 entry
differs`) with what no longer is as the set says (Codex on the other side
included), or that it matches the set. `Undo the set` puts back every entry the
apply changed that is still as the apply left it; an entry changed or removed
since stays as it is and is named, and when step 2 switched Codex the line
offers the Codex switch back. `Delete` removes a set (with `Undo`) and changes
no agent file. A set is never applied on its own. A Mac keeps at most 50 sets,
each name up to 60 characters.

## Agent updates

`Updates` closes the sidebar's `Agent` group (macOS only). A line at the top
says whether Claude Code, the Codex CLI and the Antigravity CLI are the newest
(`All are the newest · checked at 20:41`, or `2 tools have a new version`), then
a row per tool gives its version, how it was installed and its state: the
newest, a new version (`current → new` with `Update`, which runs the tool's own
update only when pressed, and stops after 10 minutes), checking, or could not
check (with the reason). Codex waits while one of its sessions is open, and an
install the app cannot update safely shows the command to run in Terminal, with
`Copy command`. `Check now` checks now; `Check automatically` checks once a day
(an hour later after a check that could not reach the servers), and while it is
on, opening the page checks when the last check is missing or a day old. A new
version under the bell opens this page. See [`SECURITY.md`](../SECURITY.md).

## Backup and restore

The `Backup` tab saves the MCP servers of the three agents, and which agent has
each one on, to a pack in iCloud Drive's `Quota Station Agents` folder (or the
folder set in `Settings › General › Backup folder`), so any of the owner's Macs
can restore it. It shows the latest pack, every Mac's packs (a pack iCloud has
not downloaded yet says so and cannot be picked until it is), `Automatic
backup` with its two switches (after every change in the app, and daily if an
agent changed), `Back up now`, and the keys the app keeps on this Mac, by name
and where each is kept, to replace or delete; it keeps the 10 latest packs of
each Mac. `See inside` says what a pack holds in plain words, and what is never
in a pack. A pack never holds a key, and paths in it are written from the home
folder, so they change to the Mac that restores it.

`Restore…` opens a sheet in steps: pick a pack, then one preview of what
changes on this Mac. Servers to add are ticked; one that differs from the pack
is not, and `Compare` shows the two side by side; the ones already the same are
listed in one line. Next the keys the pack needs are asked for in password
fields, with where each is kept. A server left without its key, or whose
command is not installed here, is added turned off, and the result says what is
left to do. The write is the MCP tab's checked write, so a file changed after
the preview stops it, and a restore never touches a sign-in, an account or
Codex's provider.

The pack also holds the skills (those from GitHub by their source, those
installed by hand with their files, unless a file has a line shaped like a key),
the plugins and the instruction files, and the same preview shows them by group
after the servers: skills from GitHub are installed again with `npx skills`,
the newest version; skills installed by hand are copied from the pack; plugins
are installed by Claude Code's and Codex's own commands; an instruction file
this Mac has not got is written, and one that differs is not ticked, with
`Compare`, so the Mac's own stays unless the owner ticks it. Each agent's box is
set as the pack had it. One part failing (a plugin that needs a sign-in, GitHub
out of reach, a skill iCloud has not downloaded) leaves the others to run, and
the result offers `Try again` for it. Once done, `Undo the restore…` takes the
whole restore back, asking first when a file was edited since. See
[`SECURITY.md`](../SECURITY.md#backup-packs-and-restore-m10) and
[its restore section](../SECURITY.md#restoring-skills-plugins-and-instruction-files-m12).

## Copy to another Mac *(Advanced)*

The tab copies MCP servers, skills, plugins and instruction files between this
Mac and one of `Settings › Advanced › Other Macs`, either way (`From this Mac
to …` or `Bring from …`). `Compare` reads the other Mac through its own Quota
Station over SSH (names, configuration and hashes, never a key) and lists what
differs by group: what only one Mac has, a server whose configuration differs (a
command at another path is not a difference), a skill of another version, a
file of other text. What only the sending Mac has starts ticked; what both have
but not alike starts unticked, and `View` shows it on both Macs side by side.
The copy button shows what will happen, and the Mac that receives the items
writes them itself, as a restore of only those items: the same checked write,
backups first, and `Undo on …` runs on that Mac. `Also send keys over SSH` is
off by default: a server that needs a key is then added turned off and that Mac
asks for the key. A Mac that does not answer, or whose app is still version 1,
is never written to. Codex's provider, sign-ins and accounts are never part of
a copy. See [`SECURITY.md`](../SECURITY.md#copying-to-another-mac-m13).

## Variables that override a sign-in

An `ANTHROPIC_API_KEY` in `~/.zshrc` makes Claude Code in Terminal use that key,
whatever account the app switches to. The app looks for eleven such names
(`ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_BASE_URL`,
`CLAUDE_CODE_OAUTH_TOKEN`, `CLAUDE_CONFIG_DIR`, `OPENAI_API_KEY`,
`OPENAI_BASE_URL`, `CODEX_HOME`, `GEMINI_API_KEY`, `GOOGLE_API_KEY`,
`GOOGLE_CLOUD_PROJECT`) in six places: `~/.zshenv`, `~/.zprofile`, `~/.zshrc`,
`~/.bash_profile`, `~/.profile` and launchd (what apps opened from the Dock
get). It learns only whether each is set, in which file and on which line,
never its value, and never runs a shell file.

The `Environment variables that override sign-in` card in `Settings ›
Services` (the first tab, and its first card, while a variable is set) names
each variable, the agent it overrides, the file and line and what it does, with
`Open file` (in Visual Studio Code, Cursor or Zed at the line when one is
installed, else the default text editor) and `Ignore`, which has `Undo`; with
none, one green line (`No variable overrides a sign-in.`). `See the list` lists
the names and the places. A variable that overrides Claude Code also puts a red
banner at the top of the `Claude` tab, and each variable gets one notification
under the bell, in the app only, sent again only when it is set somewhere new.
The app checks at most every 10 minutes after a read, when Settings opens and
on `Check again`; it never edits a shell file and never unsets anything.

## Stats

`Stats`, right under Overview: the tokens Claude Code and Codex used on this
Mac, from their own session logs (`~/.claude/projects/**/*.jsonl`; `sessions/`
and `archived_sessions/` in the Codex folder): the total, each agent's share, a
column per hour for `Today` or per day for `7 days` and `30 days` (labels five
days apart), and the counts by Claude account, by project and by model (input,
output and cache reads). `Equivalent API price` is what the tokens would cost at
the API prices, only to compare with the subscriptions; the prices are one table
in the code with its source and date, and a model without a price is left out.
The Claude account is an estimate (`Estimated from the switch history`): each
quarter hour goes to the account the switch history says was in use.
Antigravity keeps no token log on the Mac, so the chart says `Antigravity: no
token log on this Mac`.

Each column stacks the agents, each in its own colour, with that colour beside
the agent's name under the chart; a number sits on the tallest column only, and
each value shows on hover and from the keyboard (the chart is one Tab stop, and
the arrows move along the columns). `Show table` has every number in a table,
leaving out hours and days with no tokens. Only the token counts, the model,
the time and the folder are read from a line, never the conversation. The first
read shows each agent's progress and later ones read only what was appended; a
log in a format the app does not know is skipped and counted (`Skipped 38 Codex
sessions`, with `Got it`), never guessed. The counts are `stats/index.json` in
the app's folder, kept for 62 days.

## Settings

The page is in five tabs, in a bar under its title that stays at the top while
the page scrolls. Only the tab picked shows, `General` at first; the app keeps
the one picked last while it runs, and a link from elsewhere opens the one it is
about. The arrow keys, Home and End move along the bar; where the window is too
narrow for every tab, the bar scrolls sideways, faded at the end with more past
it.

- **General** (`Chung`): `Language` (`Match macOS`, `Tiếng Việt` or `English`;
  matching macOS falls back to English for a language other than Vietnamese),
  `This Mac's name` (empty uses the name macOS gives it), `Open at login` (off
  by default; macOS's own login items, and when macOS asks for approval the row
  says so and opens System Settings), `Backup folder` (empty is iCloud Drive's
  `Quota Station Agents`), then the `Updates` card: `Version` with `Check for new
  versions`, `Tell me about new versions` (a build from source; `Check for app
  updates` in a downloaded one), `Check for skill updates` (`Daily` or `Off`),
  `Show skill updates as macOS notifications`, and `Diagnostic info` with `Copy
  diagnostic info…` (scrubbed of emails and paths; nothing is sent) and `Open
  the GitHub bug form`.
- **Display** (`Hiển thị`): `Appearance` (`Match macOS`, `Light` or `Dark`),
  `Theme` (the theme pack every window is drawn in, each shown as the window in
  small in its own glass, colours and type, light and dark split across it:
  `Mặc định`, `Kính Nhịp`, `Dải sáng tĩnh`, `Một nhịp nhìn`, `Bàn điều khiển`
  or `Chiều lặng`, named in Vietnamese in both languages; a pack changes
  colours, glass, type and corners, never what is shown; see
  [themes.md](themes.md)), `Show emails` (on by default), `Service in the menu
  bar` (Claude, Gemini or Codex; one this Mac lacks is greyed out with the
  reason) and `Number in the menu bar` (`5-hour` by default, `Week`, `Fable
  week` or `Highest`; `Fable week` is greyed out unless the title follows
  Claude).
- **Notifications** (`Thông báo`): whether macOS lets the cards show, `Send a
  test`, a switch for each kind of notification, and `Running-out thresholds`
  right under the first kind (10% and 15% by default).
- **Services** (`Dịch vụ`): `Keychain access` (a one-time `Grant once` that lets
  the app ask macOS for Claude Code's Keychain item, on a Mac where the app keeps
  no Claude account), `Codex folder`, `Codex accounts`, `Antigravity OAuth
  client` (whether `oauth-client.json` is in place and usable), and the
  `Environment variables that override sign-in` card. While a variable is set,
  this tab moves to the front of the bar and the card leads it.
- **Advanced** (`Nâng cao`, `author-tools` builds only): `Show advanced
  features` shows the `Rotation rules`, `Firstmate` and `Copy to another Mac`
  tabs, automatic switching with its pause (`Until 18:00` among them), the
  background check card, the notification for an account leaving the rotation,
  and the Personal/Team and Ultra names; its `Firstmate` switch shows the
  Firstmate tab, `Firstmate file` (the path to Firstmate's `quota-pool.last`) and
  its notification. `Other Macs` gives a name and an SSH alias for each other Mac,
  with `Add Mac`, `Remove` and `Test SSH` (reads that Mac's rotation file and
  changes nothing). `Source code` is the clone this Mac updates from, and
  `Update` (`Update from source`) pulls it and installs the new build
  ([README](../README.md#update)). A settings file from before the list of
  other Macs with a single `sshAlias` moves into it.

Links elsewhere open the tab they are about: `Open Settings` in `Firstmate` and
`Copy to another Mac` opens `Advanced`, a variable's banner or notification
opens `Services`, and `Notification settings` under the bell opens
`Notifications`.

## The first run

A new install opens on a short setup that fills the window, once: `Language`;
`This Mac` (what Claude Code, Codex and Antigravity this Mac has; nothing is
sent anywhere); `Claude accounts`, when Claude Code is here (it takes the
account Claude Code is signed in to, and can give it a nickname, sign another
in with the browser or take a pasted setup-token; one account is fine, and
`Skip, add later` moves on); `Permissions` (Keychain access with `Grant once`,
notifications with `Allow`, `Open at login` and the menu bar's service and
number); and `Done`, with `Open the main window` and `Open the popover`. A Mac
that already had the app skips it.

## App updates

The app asks GitHub Releases for a newer version of itself: once a day while
the switch in `Settings › General › Updates` is on, and on `Check for new
versions`. A new version brings a banner at the foot of `Overview` (`New
version: Quota Station X`, with `See what's new` and `Later`) and a line in the
popover's footer; its dialog has `Download and install`, which opens the
release's DMG (or its page) in the browser, and `Skip this version`. Nothing is
downloaded or installed by the app itself. A build from source warns not to
install the download over it and gives the command to update from the clone
instead (`git pull --ff-only && scripts/install.sh`, with `Copy command`). While
GitHub has no release, the status says `Already the newest: no release on
GitHub yet`.
