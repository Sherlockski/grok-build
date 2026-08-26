<div align="center">

<h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://media.x.ai/v1/website/spacexai-symbol-white-transparent-0c31957f.png">
    <source media="(prefers-color-scheme: light)" srcset="https://media.x.ai/v1/website/spacexai-symbol-black-transparent-6435cf42.png">
    <img alt="SpaceXAI logo" src="https://media.x.ai/v1/website/spacexai-symbol-black-transparent-6435cf42.png" width="96">
  </picture>
  <br>
  Grok Build (<code>grok</code>)
</h1>

**Grok Build** is SpaceXAI's terminal-based AI coding agent. It runs as a
full-screen TUI that understands your codebase, edits files, executes shell
commands, searches the web, and manages long-running tasks — interactively,
headlessly for scripting/CI, or embedded in editors via the Agent Client
Protocol (ACP).

[Theming](#live-reloading-config-file-based-themes) ·
[Installing the released binary](#installing-the-released-binary) ·
[Building from source](#building-from-source) ·
[Documentation](#documentation) ·
[Repository layout](#repository-layout) ·
[Development](#development) ·
[Contributing](#contributing) ·
[License](#license)

![Grok Build TUI](https://media.x.ai/v1/website/universe-tui-screenshot-6f7a0837.png)

**Learn more about Grok Build at [x.ai/cli](https://x.ai/cli)**

This repository contains the Rust source for the `grok` CLI/TUI and its agent
runtime. It is synced periodically from the SpaceXAI monorepo.

A small `SOURCE_REV` file at the root records the full monorepo commit SHA
for the version of the code present in this tree.

</div>

---

## Live-reloading config-file based themes

Grok ships **Aura** (daltonmenezes/aura-theme) as a bundled file theme, and you
can add your own themes as plain config files — no rebuild required. Anything in
the theme directory is picked up automatically, appears in `/theme` and the
settings picker (Settings → Appearance → Theme), and **hot-reloads live** while
grok is running: edit a theme file and the UI recolors immediately; edit the
pointer file to switch themes without touching the app.

### Layout

Everything lives under your grok home (`~/.grok` by default, or `$GROK_HOME`):

```
~/.grok/
└── themes/
    ├── config.toml        # pointer file — which theme is active
    └── my-theme.toml      # one theme per file
```

The pointer file selects the active theme. The canonical name is
`themes/config.toml`, but these equivalents all work — first match wins:
`themes/theme.toml`, `themes/theme-config.toml|yaml|yml|json`,
`themes/config.yaml|yml|json`, or `theme-config.*` / `theme.*` at the grok home
root.

```toml
# ~/.grok/themes/config.toml
active = "my-theme"
```

(`active` is canonical; `theme`, `current`, and `name` are accepted aliases, and
TOML, YAML, and JSON are all parsed.)

### Writing a theme

One file under `themes/` per theme, named `<theme-name>.toml` (`.yaml`, `.yml`,
and `.json` also work). Files are **sparse**: any key you omit falls back to the
built-in Grok Night palette, so a two-line theme is valid.

```toml
# ~/.grok/themes/my-theme.toml
[meta]
display = "My Theme"            # picker label (defaults to Title Case of filename)
description = "Warm amber"      # shown in the picker
requires_truecolor = true       # set false if it looks fine on 256-color terminals

[theme]
bg_base = "#0f0f0f"             # main canvas background
accent_user = "#a277ff"
text_primary = "#edecee"

md_heading_h1_mod = "bold"      # modifiers: bold,dim,italic,underline,...
```

Every field of the internal `Theme` struct is overridable using its Rust field
name — backgrounds (`bg_base`, `bg_light`, `bg_dark`, `bg_highlight`,
`bg_hover`, `bg_terminal`, `bg_visual`), accents (`accent_user`,
`accent_assistant`, `accent_thinking`, `accent_tool`, `accent_system`,
`accent_error`, `accent_success`, `accent_running`, `accent_skill`,
`accent_plan`, `accent_verify`, `accent_remember`, `accent_model`),
text/grays (`text_primary`, `text_secondary`, `gray_dim`, `gray`,
`gray_bright`), semantics (`command`, `path`, `running`, `warning`,
`fuzzy_accent`), borders (`selection_border`, `hover_border`, `prompt_border`,
`prompt_border_active`), scrollbar/diff/paste colors, and the markdown palette
(`md_heading_h1`…`md_heading_h6`, `md_code`, `md_code_bg`, `md_muted`,
`md_text`, `link_fg`, `md_task_checked`, `md_task_unchecked`). Heading
modifiers use `<field>_mod`.

Color literals: `#rrggbb`, `#rgb`, `#rrggbbaa` (alpha composited at parse),
named ANSI (`red`, `lightblue`, `darkgray`, …), `idx:N` for xterm-256 indices,
or `none`/`reset`.

Grok ships **no** file themes — everything in the picker beyond the built-ins
is there because you put it in `themes/`.

### Selecting a theme

- **Slash command:** `/theme my-theme` (Tab completes custom themes). Bare
  `/theme` cycles built-ins.
- **Settings modal:** Settings → Appearance → Theme; custom themes appear
  alongside the built-ins. Arrow keys preview live, Enter commits, Esc reverts.
- **Config:** `[ui].theme = "my-theme"` in `config.toml`.
- **Pointer file:** write `active = "…"` to `themes/config.toml` — even while
  grok is running. It switches within ~120 ms, no restart.
- **Env:** `GROK_THEME=my-theme grok`.

Committing a theme via `/theme` or the picker keeps the pointer file in sync,
so the two stay consistent.

### Live reload behavior

- Theme *files* are watched with inotify (debounced ~120 ms). Editing the file
  of the **active** theme recolors the running app instantly — handy for
  iterating on palettes with a second monitor/editor.
- Creating/deleting theme files updates the pickers on next open.
- Events that land while you're mid-preview (arrowing through the picker) are
  suppressed so a stale write can't snap you back to the committed theme.
- If backgrounds look wrong over SSH/tmux, truecolor detection may be degraded
  — force it with `COLORTERM=truecolor` or `GROK_FORCE_COLOR_LEVEL=truecolor`
  (also accepts `none`/`basic`/`256`).

---

## Installing the released binary

Prebuilt binaries are published for macOS, Linux, and Windows:

```sh
curl -fsSL https://x.ai/cli/install.sh | bash   # macOS / Linux / Git Bash
irm https://x.ai/cli/install.ps1 | iex          # Windows PowerShell
grok --version
```

See the [changelog](https://x.ai/build/changelog) for the latest fixes,
features, and improvements in each release.

## Building from source

Requirements:

- **Rust** — the toolchain is pinned by [`rust-toolchain.toml`](rust-toolchain.toml);
  `rustup` installs it automatically on first build.
- **[DotSlash](https://dotslash-cli.com)** — required so hermetic tools under
  [`bin/`](bin/) (notably [`bin/protoc`](bin/protoc)) can download and run.
  Install it and ensure `dotslash` is on your `PATH` **before** building:

  ```sh
  cargo install dotslash
  # or: prebuilt packages — https://dotslash-cli.com/docs/installation/
  /usr/bin/env dotslash --help   # sanity check
  ```

- **protoc** — proto codegen resolves [`bin/protoc`](bin/protoc) via DotSlash,
  or falls back to a `protoc` on `PATH` / `$PROTOC`.
- macOS and Linux are supported build hosts; Windows builds are best-effort
  and not currently tested from this tree.

```sh
cargo run -p xai-grok-pager-bin              # build + launch the TUI
cargo build -p xai-grok-pager-bin --release  # release binary: target/release/xai-grok-pager
cargo check -p xai-grok-pager-bin            # fast validation
```

The binary artifact is named `xai-grok-pager`; official installs ship it as
`grok`. On first launch it opens your browser to authenticate — see the
[authentication guide](crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md).

## Documentation

Full online documentation is available at
[docs.x.ai/build/overview](https://docs.x.ai/build/overview).

The user guide ships with the pager crate:
[`crates/codegen/xai-grok-pager/docs/user-guide/`](crates/codegen/xai-grok-pager/docs/user-guide/)
— getting started, keyboard shortcuts, slash commands, configuration, theming,
MCP servers, skills, plugins, hooks, headless mode, sandboxing, and more.

## Repository layout

| Path | Contents |
|------|----------|
| `crates/codegen/xai-grok-pager-bin` | Composition-root package; builds the `xai-grok-pager` binary |
| `crates/codegen/xai-grok-pager` | The TUI: scrollback, prompt, modals, rendering |
| `crates/codegen/xai-grok-shell` | Agent runtime + leader/stdio/headless entry points |
| `crates/codegen/xai-grok-tools` | Tool implementations (terminal, file edit, search, ...) |
| `crates/codegen/xai-grok-workspace` | Host filesystem, VCS, execution, checkpoints |
| `crates/codegen/...` | The rest of the CLI crate closure (config, MCP, markdown, sandbox, ...) |
| `crates/common/`, `crates/build/`, `prod/mc/` | Small shared leaf crates pulled in by the closure |
| `third_party/` | Vendored upstream source (Mermaid diagram stack) — see below |

> [!IMPORTANT]
> The root `Cargo.toml` (workspace members, dependency versions, lints,
> profiles) is **generated** — treat it as read-only. Prefer editing per-crate
> `Cargo.toml` files.

## Development

```sh
cargo check -p <crate>        # always target specific crates; full-workspace builds are slow
cargo test -p xai-grok-config # per-crate tests
cargo clippy -p <crate>       # lint config: clippy.toml at the repo root
cargo fmt --all               # rustfmt.toml at the repo root
```

## Contributing

> [!NOTE]
> External contributions are not accepted. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

First-party code in this repository is licensed under the **Apache License,
Version 2.0** — see [`LICENSE`](LICENSE).

Third-party and vendored code remains under its original licenses. See:

- [`THIRD-PARTY-NOTICES`](THIRD-PARTY-NOTICES) — crates.io / git dependencies,
  bundled UI themes, and **in-tree source ports** (including openai/codex and
  sst/opencode tool implementations)
- [`crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md`](crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md)
  — crate-local notice for the codex and opencode ports (license texts +
  Apache §4(b) change notice)
- [`third_party/NOTICE`](third_party/NOTICE) — vendored Mermaid-stack index
