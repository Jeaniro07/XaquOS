# Where this tree comes from

Exported on 2026-11-19 from the trees the build uses. `main` carries the
whole history of the system, not a squashed copy: 4.464 commits from the
first one to `45e4c42e6`.

| part | source | commit |
|---|---|---|
| the system (`kernel`, `lib`, `user/system`, `tools`, `docs`, `assets`) | `ferrix`, branch `rename/xaqu` | `45e4c42e6` |
| every app (`src/user/apps`) | `apps`, branch `desktop/a4a-spotlight` | `d31719a` |
| `LORDOS/` | the LordOS checkout in the harness workspace | its `main` |
| `TUGAS-AGENT.md`, `docs/superpowers/` | the harness workspace | 2026-11-19 |

The apps and LordOS are carried as folders here so one clone holds
everything; their own histories ride as branches (`apps/*`, `lordos/*`).
Components the build fetches for itself (`tole`, `ferrousli`, `zinc`,
`pixel7`, the website) stay out of the tree, as `components.toml` says.
