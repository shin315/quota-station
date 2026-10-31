# Switching over from claude-swap and Cockpit Tools, Mac by Mac

Quota Station replaces two tools the author ran before it: claude-swap (with
its `cswap-prefer-team` switcher) for the Claude accounts, and Cockpit Tools for
the Antigravity accounts. This page is how a Mac moves from them to Quota
Station alone. A new user who never ran either can skip it.

The goal is one menu bar icon on each Mac: Quota Station's. Do one Mac at a
time. `quota-station` below is
`/Applications/Quota Station.app/Contents/MacOS/quota-station`.

1. **Install** Quota Station (see the [README](../README.md#install)) and open
   it. Check that it reads every service on this Mac, and on a Mac with
   Antigravity, that `oauth-client.json` is usable (`Settings › Services`,
   [how-it-works.md](how-it-works.md#the-oauth-client-file)) and the accounts
   this Mac uses are signed in, with fresh numbers and none that needs a new
   login.
2. **Check that Cockpit Tools is gone:**

   ```sh
   ls -d "/Applications/Cockpit Tools.app" ~/.cockpit_tools ~/.antigravity_cockpit
   lsof -i :19528
   launchctl list | grep -i cockpit
   ```

   and look for Cockpit in System Settings › General › Login Items. Before
   relying on the app alone, also check that the `Antigravity` tab lists every
   account with its numbers, and that one Antigravity switch from the app, and
   one back, both say they switched.

   If Cockpit is found: quit it, remove it from Login Items (it starts itself
   through its autostart setting), and move `/Applications/Cockpit Tools.app`
   to the Trash. Keep `~/.cockpit_tools` for 7 days: it holds Cockpit's own
   copy of the logins, and reinstalling Cockpit with that folder in place
   brings them back. Delete it after that. Do not delete the
   `gemini`/`antigravity` Keychain item: Antigravity.app and agy use it.
3. **Move the Claude accounts into Quota Station**, when you decide to on this
   Mac. Nothing here switches the account Claude Code is on, except the
   rotation once it is turned on (step 3.4), which switches as
   `cswap-prefer-team` did; a switch makes the running Claude Code sessions
   rebuild their prompt cache, so pick a quiet moment. While the old switcher
   is loaded, the `Claude` tab says so (`The old auto-switcher is still
   running`) with `How to turn it off`.

   1. Stop the older switcher, keeping its plist so it can come back:

      ```sh
      cp ~/Library/LaunchAgents/com.cswap.prefer-team.plist \
         ~/Library/LaunchAgents/com.cswap.prefer-team.plist.bak
      launchctl bootout gui/$(id -u)/com.cswap.prefer-team
      launchctl bootout gui/$(id -u)/com.cswap.auto 2>/dev/null  # if it was loaded
      launchctl list | grep -i cswap                              # prints nothing
      ```

      Until then Quota Station switches nothing and refreshes no token, so the
      two never rotate the same refresh token.
   2. In the `Claude` tab, press `Add account` and then `Import from
      claude-swap` (or run `quota-station claude import`). Every account comes
      over under its slot number with its alias and its login, without a new
      login, and the rotation file and the priming flag are copied to the app's
      folder. Every account comes in the rotation: claude-swap's own `disabled`
      is not carried over (the old switcher never read it).
      `~/.claude-swap-backup` and claude-swap's Keychain items are only read.
      Running it again takes only a newer login.
   3. Check the list: every account is there, the one in use is marked
      `In use`, and after `Refresh` each has numbers. `quota-station claude
      status` prints the same.
   4. Turn on automatic switching (`Claude auto-switch` with `Settings ›
      Advanced` on, or `quota-station claude rotation on`), then check that
      `launchctl list | grep quota-station.rotation` prints a line and that
      `claude/rotation.log` in the app's folder gets a decision within three
      minutes.
   5. Point Firstmate at `quota-station claude status --json` (in place of
      `cswap list --json`) and `quota-station claude run N --` (in place of
      `cswap run N --`); see [the command line](how-it-works.md#the-command-line-for-firstmate).

   To go back: `quota-station claude rotation off`, then
   `launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.cswap.prefer-team.plist`.
   claude-swap's data was never changed. An account the app refreshed in the
   meantime has a newer refresh token than claude-swap's copy, so claude-swap
   may ask for a new login of that account. Leave claude-swap installed until
   every Mac has run on Quota Station for a while, and ask before removing it.
4. **Turn off claude-swap's own menu bar**, when you decide to on this Mac:

   ```sh
   cswap menubar --uninstall-service
   ```

   Claude numbers stay fresh without it once Quota Station keeps the accounts:
   the app reads them itself.
5. **Confirm one icon:** the menu bar shows Quota Station and neither
   claude-swap's item nor Cockpit's.

A Mac without Antigravity (no Antigravity.app and no agy) has no Cockpit step:
its Antigravity section says `No Antigravity on this Mac`.
