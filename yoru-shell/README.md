# yoru-shell

Yoru's shell: the dock of the customer's "Yoru WM" design (Ferrix's
[docs/DESKTOP.md](https://github.com/ferrix-os/ferrix/blob/develop/docs/DESKTOP.md)
§6), a layer-shell surface floating above the foot of the screen, drawn
with tole in the Nocturne design system's tokens, Inter for words and
Phosphor for icons. An app (Ferrix's
[docs/APPS.md](https://github.com/ferrix-os/ferrix/blob/develop/docs/APPS.md)):
`app.toml` says what it installs and how it is built; its licence is `MIT`, and the Phosphor icon font it carries
(`share/fonts/Phosphor.ttf`, Phosphor 2.1.1) is MIT too (`LICENSE-phosphor`).

![Yoru on Ferrix: the theme's windows and the dock](screenshot.png)

![The dock, drawn by `--render`](dock.png)

## The dock and the menu bar

- **Dock** - a bar of glass in the middle of the screen's foot. The launchers
  (Terminal, Browser, Files, Settings) as tiles, a dot under each that has a
  window on any desktop or minimized; the other programs with windows after
  them; a line; the stash, where files can be dropped. A press brings the
  window used last (a minimized one comes back) or starts the program.
- **Menu bar** - a strip of glass across the top, on a thread of the same
  program. At the left the FX menu (Settings, Wallpaper, Restart or Shut
  Down), the focused program's (About, Hide, Quit) and Window (Minimize,
  Full Screen, Move to Desktop 1-4, Close); at the right the search and the
  clock. `--menubar` runs it alone; `--render-menubar FILE [--open N]` draws
  it to a PNG.
- **Wallpaper** - the switcher's first card, Default, is the theme's own
  gradient; pictures (PNG, JPEG, SVG) and videos as before.
- **Power** - the FX menu's Restart or Shut Down: `yoru-shell --power`, a
  card over the dimmed screen with Lock, Restart and Shut down (Lock not in
  the live session, whose person has no password). Restart and Shut down ask once
  more, then call `svc reboot` or `svc poweroff`, which the init allows the
  person at the machine (Ferrix's docs/INIT.md §10.1); a refusal is shown in
  the card. `--render-power FILE [--live] [--confirm]` draws it.

The sidebars (`--sidebar left|right`) are no longer started by the theme;
their contents move to Control Center and Notices (A4).

It follows hyprix through Hyprland's IPC: `.socket2.sock`'s events, and
`j/workspaces`, `j/activeworkspace` and `j/clients` after each one that
moves something. Yoru's theme starts it (`exec-once`) and blurs what is
behind it (`layerrule = blur true, match:namespace ^(yoru-.*)$`).

`yoru-shell --render FILE` draws the dock of a sample desktop into a PNG,
as above; `cargo test` checks the model, the IPC's events and a drawn dock.
