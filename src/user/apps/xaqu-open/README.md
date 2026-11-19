# xaqu-open

Opens a file in the world that owns it.

The desktop hands a file here -- from Files, from the launcher, from a
terminal -- and `xaqu-open` says which world the file belongs to and runs it
by the runtime that world is run by:

| a file            | the world | it is run by                                   |
|-------------------|-----------|------------------------------------------------|
| a Linux program   | linux     | itself, or the loader `linux` names            |
| a `.fxpkg`        | linux     | `/bin/pkg install`                             |
| a `.exe`, a `.msi`| windows   | the runtime `windows` names, e.g. Wine         |
| an `.apk`         | android   | the runtime `android` names                    |
| a `.app`, a `.dmg`| macos     | the runtime `macos` names                      |
| anything else    | --        | `xdg-open`, the desktop's own opener           |

The runtimes are the user's: Xaqu runs a Linux program itself, and the
other worlds need what the user brings. Where they are is written in
`/etc/xaqu/app-worlds.conf`, a runtime and the words it takes before the
file. A desktop's image names Windows' runtime, bridgeplay, out of the box:

```
windows = /bin/bridgeplay run
linux = /data/linux/lib64/ld-linux-x86-64.so.2
android = /data/usr/bin/android-translation-layer
macos = /data/usr/bin/darling
```

A world with no runtime is refused, with the way out said in the refusal:
nothing is guessed, and nothing is run that the user did not ask for.
