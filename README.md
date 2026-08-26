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

Themes are plain config files under your grok home — no rebuild required.
**Grok ships none**: everything in the picker beyond the six built-ins is
there because you put it in `themes/`. Files are discovered automatically,
appear in `/theme` and the settings picker (Settings → Appearance → Theme),
are resolved at launch from `[ui].theme` or the pointer file, and
**hot-reload live** while grok is running — edit a theme file and the UI
recolors immediately, no restart.

An example port of **Aura Dark** (daltonmenezes/aura-theme) is included at the
bottom of this section; save it as `~/.grok/themes/aura.toml` to use it.

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

### Mapping VS Code themes

Grok's field set is semantic, not a 1:1 mirror of VS Code keys. The reference
pair is **VS Code Tokyo Night ↔ the built-in Tokyo Night**:
[`theme/tokyonight.rs`](crates/codegen/xai-grok-pager-render/src/theme/tokyonight.rs)
(the `palette` block + `Theme::tokyonight()`) vs the theme JSON on
[marketplace](https://marketplace.visualstudio.com/items?itemName=enkia.tokyo-night).
Open them side by side; every grok field below traces back to a VS Code key.
Apply the same trace to any other theme.

| VS Code key (Tokyo Night value) | Grok field(s) |
|---|---|
| `editor.background` — Storm (`#24283b`) | `bg_base` |
| `list.activeSelectionBackground` (`#292e42`) | `bg_light`, `bg_dark`, `bg_highlight` |
| Night editor/`terminal.background` (`#1a1b26`) | `bg_terminal` |
| `editor.foreground` (`#c0caf5`) | `text_primary`, `md_text` |
| `editor.selectionBackground` | `bg_visual` |
| `editorWidget.background` / `tab.inactiveBackground` | `paste_bg`, `md_code_bg`, `scrollbar_bg` |
| `scrollbarSlider.background` | `scrollbar_fg` |
| `editor.hoverHighlightBackground`-family mid-tone | `bg_hover` |
| `terminal.ansiYellow` (`#e0af68`) | `command`, `warning`, `accent_plan` |
| `terminal.ansiBrightYellow` (`#ff9e64`) | `path` |
| `terminal.ansiCyan`/`BrightCyan` | `running`, `md_task_checked`, `accent_model` |
| `terminal.ansiMagenta` (`#bb9af7`) | `accent_assistant`, `accent_running`, `accent_verify`, `md_heading_h6` |
| `terminal.ansiBlue` (`#7aa2f7`) | `accent_user`, `accent_system`, `accent_skill`, `fuzzy_accent`, `link_fg`, `md_heading_h2` |
| `terminal.ansiGreen` (`#9ece6a`) | `accent_success`, `accent_remember`, `md_heading_h5` |
| `terminal.ansiRed` / errorForeground (`#f7768e`) | `accent_error`, `diff_delete_fg` |
| comment tokenColor (`#565f89`) | `gray`, `md_muted`, `diff_equal_fg` |
| `tree.indentGuidesStroke` / line numbers | `gray_dim`, `diff_gutter_fg` |
| bright punctuation gray (`#737aa2`) | `gray_bright`, `accent_tool` |
| dim gutter gray (`#3b4261`) | `accent_thinking`, `paste_dim` |
| `inputOption.activeBorder` / `editorCursor.foreground` | `prompt_border_active`, `accent_user` |
| `focusBorder` / `editorBracketMatch.border` | `selection_border`, `hover_border`*, `prompt_border`* |
| `gitDecoration.*ResourceForeground` | `command` (modified), `diff_insert_fg` (untracked) |

\* Tokyo Night derives border/hover tones from its blues — pick the mid-tone
of the same family rather than copying a single key.

Markdown headings don't exist in VS Code's palette as six steps; both ports
invent a ladder from the accent set (Tokyo Night: teal → blue → orange → red →
green → magenta for h1–h6). Pick any pleasing ramp from the target theme's
token colors and add `_mod = "bold"` where the VS Code theme sets bold.

**Alpha channels:** VS Code themes use 8-digit hex (`#rrggbbaa`). Terminals
don't blend, so composite over `bg_base` yourself per channel:

```
out = round(a × fg + (1 − a) × bg_base)      # a = AA/255
```

Example: Aura's `editor.selectionBackground #3d375e7f` over `#15141b`
becomes `#3d375e`; its scrollbar slider `#a394f033` over `#15141b` becomes
`#312e46`.

### Selecting a theme

- **Slash command:** `/theme my-theme` (Tab completes custom themes). Bare
  `/theme` cycles built-ins.
- **Settings modal:** Settings → Appearance → Theme; custom themes appear
  alongside the built-ins. Arrow keys preview live, Enter commits, Esc reverts.
- **Config:** `[ui].theme = "my-theme"` in `config.toml` — applied at launch.
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

### Example: Aura Dark

Full port of [daltonmenezes/aura-theme](https://github.com/daltonmenezes/aura-theme)
(hexes verbatim from its VS Code / color-palette sources). Save as
`~/.grok/themes/aura.toml`, then `/theme aura`.

<details><summary>aura.toml</summary>

```toml
[meta]
display = "Aura"
description = "Dark purple — aura theme port."
requires_truecolor = true

[theme]
bg_base = "#15141b"
bg_light = "#2e2b38"
bg_dark = "#110f18"
bg_highlight = "#3b334b"
bg_hover = "#3b334b"
bg_terminal = "#15141b"

accent_user = "#a277ff"
accent_assistant = "#f694ff"
accent_thinking = "#6d6d6d"
accent_tool = "#525156"
accent_system = "#82e2ff"
accent_error = "#ff6767"
accent_success = "#61ffca"
accent_running = "#a277ff"
accent_skill = "#f694ff"

text_primary = "#edecee"
text_secondary = "#cdccce"

gray_dim = "#4d4d4d"
gray = "#6d6d6d"
gray_bright = "#adacae"

command = "#ffca85"
path = "#82e2ff"
running = "#61ffca"
warning = "#ffca85"

fuzzy_accent = "#a277ff"
accent_plan = "#ffca85"
accent_verify = "#a277ff"
accent_remember = "#61ffca"

selection_border = "#3d375e"
hover_border = "#3b334b"
prompt_border = "#3b334b"
prompt_border_active = "#a277ff"

accent_model = "#61ffca"

scrollbar_bg = "#121016"
scrollbar_fg = "#312e46"

diff_delete_bg = "#321a25"
diff_delete_fg = "#ff6767"
diff_insert_bg = "#122c29"
diff_insert_fg = "#61ffca"
diff_equal_fg = "#6d6d6d"
diff_gutter_fg = "#4d4d4d"

bg_visual = "#3d375e"

paste_bg = "#121016"
paste_fg = "#cdccce"
paste_dim = "#6d6d6d"

md_heading_h1 = "#a277ff"
md_heading_h1_mod = "bold"
md_heading_h2 = "#f694ff"
md_heading_h2_mod = "bold"
md_heading_h3 = "#82e2ff"
md_heading_h3_mod = "bold"
md_heading_h4 = "#61ffca"
md_heading_h4_mod = "bold"
md_heading_h5 = "#ffca85"
md_heading_h5_mod = "bold"
md_heading_h6 = "#6d6d6d"
md_heading_h6_mod = "bold"
md_code = "#61ffca"
md_task_checked = "#61ffca"
md_task_unchecked = "#cdccce"
md_muted = "#6d6d6d"
md_code_bg = "#121016"
md_text = "#edecee"
link_fg = "#f694ff"
```

</details>

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
