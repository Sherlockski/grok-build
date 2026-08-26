//! File-based custom themes.
//!
//! Layout under `$GROK_HOME/themes/` (or `$GROK_HOME` from `xai_grok_home`):
//!   themes/config.toml  -> pointer: `active = "my-theme"` or `theme = "my-theme"`
//!                         aliases checked: `themes/theme.toml`, `themes/theme-config.toml`,
//!                         `theme-config.toml` / `theme.toml` at `$GROK_HOME` root for
//!                         the prompt's `theme-config.yaml` shape (toml/yaml/json all accepted).
//!   themes/<name>.toml|yaml|yml|json  -> Theme definition
//!
//! `<name>.toml` format (sparse — missing keys fall back to GrokNight):
//!   [meta]
//!   display = "My Theme"
//!   description = "Warm amber"
//!   requires_truecolor = true
//!   [theme]
//!   bg_base = "#0f0f0f"
//!   accent_user = "#a277ff"
//!   ... any Theme field name from `tokyonight::Theme` ...
//!   md_heading_h1_mod = "bold"   # comma-separated: bold,italic,underline,dim
//!
//! Supported color literals: `#rrggbb`, `#rgb`, `none`/`reset`, named ANSI
//! (`black`, `white`, `red`, `lightred`, … `darkgray`, `gray`). Hex is case-insensitive.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier};
use serde::Deserialize;

use super::tokyonight::Theme;

const POINTER_KEYS: &[&str] = &["active", "theme", "current", "name"];

// ── Preview guard ───────────────────────────────────────────────────────
//
// While the user is live-previewing themes (picker arrows / slash arg
// navigation), in-flight file events (debounced writes from an earlier
// commit landing 100ms+ late) must NOT re-apply the committed theme —
// that raced previews and snapped them back mid-navigation. Preview
// paths stamp this; the watcher checks it before applying.

use std::sync::atomic::{AtomicU64, Ordering};

static LAST_PREVIEW_MS: AtomicU64 = AtomicU64::new(0);

pub(crate) fn now_millis() -> u64 {    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

const PREVIEW_GUARD_MS: u64 = 750;

/// Stamp "a preview just happened" — watcher suppresses applies for
/// [`PREVIEW_GUARD_MS`].
pub fn mark_preview() {
    LAST_PREVIEW_MS.store(now_millis(), Ordering::Release);
}

/// True while within [`PREVIEW_GUARD_MS`] of the last preview.
pub fn preview_active() -> bool {
    now_millis().saturating_sub(LAST_PREVIEW_MS.load(Ordering::Acquire)) < PREVIEW_GUARD_MS
}

fn resolve_home_raw() -> Option<PathBuf> {
    if let Some(v) = std::env::var_os("GROK_HOME") {
        if !v.is_empty() {
            return Some(PathBuf::from(v));
        }
    }
    dirs::home_dir().map(|h| {
        dunce::canonicalize(&h)
            .unwrap_or(h)
            .join(".grok")
    })
}

fn themes_dir() -> PathBuf {
    if let Some(home) = resolve_home_raw() {
        home.join("themes")
    } else {
        PathBuf::from("themes")
    }
}

fn grok_home() -> Option<PathBuf> {
    resolve_home_raw()
}

fn pointer_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = grok_home() {
        for rel in [
            "themes/config.toml",
            "themes/theme.toml",
            "themes/theme-config.toml",
            "themes/theme-config.yaml",
            "themes/theme-config.yml",
            "themes/config.yaml",
            "themes/config.yml",
            "themes/theme.yaml",
            "theme-config.toml",
            "theme-config.yaml",
            "theme-config.yml",
            "theme.toml",
            "theme.yaml",
        ] {
            out.push(home.join(rel));
        }
        // json variants
        for rel in ["themes/config.json", "theme-config.json"] {
            out.push(home.join(rel));
        }
    }
    out
}

fn is_pointer_file_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "config.toml"
            | "config.yaml"
            | "config.yml"
            | "config.json"
            | "theme.toml"
            | "theme.yaml"
            | "theme.yml"
            | "theme.json"
            | "theme-config.toml"
            | "theme-config.yaml"
            | "theme-config.yml"
            | "theme-config.json"
    )
}

// ---------------------------------------------------------------------------
// Color + modifier parsing
// ---------------------------------------------------------------------------

fn parse_color(s: &str) -> Option<Color> {
    let t = s.trim();
    if t.eq_ignore_ascii_case("none") || t.eq_ignore_ascii_case("reset") {
        return Some(Color::Reset);
    }
    // named ANSI
    match t.to_ascii_lowercase().as_str() {
        "black" => return Some(Color::Black),
        "red" => return Some(Color::Red),
        "green" => return Some(Color::Green),
        "yellow" => return Some(Color::Yellow),
        "blue" => return Some(Color::Blue),
        "magenta" => return Some(Color::Magenta),
        "cyan" => return Some(Color::Cyan),
        "white" => return Some(Color::White),
        "gray" | "grey" | "silver" => return Some(Color::Gray),
        "darkgray" | "dark_gray" | "darkgrey" | "brightblack" => return Some(Color::DarkGray),
        "lightred" | "light_red" | "brightred" => return Some(Color::LightRed),
        "lightgreen" | "light_green" | "brightgreen" => return Some(Color::LightGreen),
        "lightyellow" | "light_yellow" | "brightyellow" => return Some(Color::LightYellow),
        "lightblue" | "light_blue" | "brightblue" => return Some(Color::LightBlue),
        "lightmagenta" | "light_magenta" | "brightmagenta" => return Some(Color::LightMagenta),
        "lightcyan" | "light_cyan" | "brightcyan" => return Some(Color::LightCyan),
        "lightgray" | "light_gray" => return Some(Color::Gray),
        _ => {}
    }
    // indexed: "idx:123" or "indexed:123"
    if let Some(rest) = t.strip_prefix("idx:").or_else(|| t.strip_prefix("indexed:")) {
        if let Ok(n) = rest.trim().parse::<u8>() {
            return Some(Color::Indexed(n));
        }
    }
    // hex
    let hex = t.strip_prefix('#').unwrap_or(t);
    if hex.len() == 3 {
        let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
        return Some(Color::Rgb(r, g, b));
    }
    if hex.len() == 6 || hex.len() == 8 {
        // #RRGGBBAA: alpha channel is ignored (terminals don't blend SGR);
        // pre-composite over the theme bg in the .toml instead.
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some(Color::Rgb(r, g, b));
    }
    None
}

fn parse_modifier(s: &str) -> Modifier {
    let mut m = Modifier::empty();
    for part in s.split(|c| c == ',' || c == '|' || c == '+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "bold" => m |= Modifier::BOLD,
            "dim" => m |= Modifier::DIM,
            "italic" => m |= Modifier::ITALIC,
            "underlined" | "underline" => m |= Modifier::UNDERLINED,
            "slow_blink" | "slowblink" | "blink" => m |= Modifier::SLOW_BLINK,
            "rapid_blink" => m |= Modifier::RAPID_BLINK,
            "reversed" | "reverse" => m |= Modifier::REVERSED,
            "hidden" => m |= Modifier::HIDDEN,
            "crossed_out" | "crossedout" | "strike" => m |= Modifier::CROSSED_OUT,
            "" | "none" | "empty" => {}
            _ => {}
        }
    }
    m
}

// ---------------------------------------------------------------------------
// File shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default, Clone)]
struct Meta {
    display: Option<String>,
    description: Option<String>,
    requires_truecolor: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct PointerFile {
    active: Option<String>,
    theme: Option<String>,
    current: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawFile {
    meta: Option<Meta>,
    #[serde(default)]
    theme: HashMap<String, String>,
    // allow top-level flat colors as fallback when no [theme] table
    #[serde(flatten)]
    flat: HashMap<String, String>,
}

fn read_pointer_from_path(path: &Path) -> Option<String> {
    let data = std::fs::read_to_string(path).ok()?;
    // try toml first, then yaml, then json
    if path.extension().is_some_and(|e| e == "json") {
        if let Ok(v) = serde_json::from_str::<PointerFile>(&data) {
            for k in [v.active, v.theme, v.current, v.name] {
                if let Some(s) = k {
                    let t = s.trim().to_string();
                    if !t.is_empty() {
                        return Some(t);
                    }
                }
            }
        }
        if let Ok(v) = serde_json::from_str::<HashMap<String, String>>(&data) {
            for key in POINTER_KEYS {
                if let Some(s) = v.get(*key) {
                    let t = s.trim().to_string();
                    if !t.is_empty() {
                        return Some(t);
                    }
                }
            }
        }
        return None;
    }
    // toml
    if let Ok(v) = toml::from_str::<PointerFile>(&data) {
        for k in [v.active.clone(), v.theme.clone(), v.current.clone(), v.name.clone()] {
            if let Some(s) = k {
                let t = s.trim().to_string();
                if !t.is_empty() {
                    return Some(t);
                }
            }
        }
    }
    if let Ok(v) = toml::from_str::<HashMap<String, String>>(&data) {
        for key in POINTER_KEYS {
            if let Some(s) = v.get(*key) {
                let t = s.trim().to_string();
                if !t.is_empty() {
                    return Some(t);
                }
            }
        }
    }
    // yaml fallback for .yaml pointer files written as yaml
    if let Ok(v) = serde_yaml::from_str::<PointerFile>(&data) {
        for k in [v.active, v.theme, v.current, v.name] {
            if let Some(s) = k {
                let t = s.trim().to_string();
                if !t.is_empty() {
                    return Some(t);
                }
            }
        }
    }
    None
}

/// Active theme name from the pointer file, if present.
pub fn load_pointer() -> Option<String> {
    for p in pointer_candidates() {
        if p.exists() {
            if let Some(v) = read_pointer_from_path(&p) {
                let t = v.trim().to_string();
                if !t.is_empty() {
                    tracing::info!(pointer = %p.display(), active = %t, "custom theme pointer loaded");
                    return Some(t);
                }
            }
        }
    }
    None
}

/// Path to the current pointer file (existing or default to create).
pub fn pointer_path() -> PathBuf {
    for p in pointer_candidates() {
        if p.exists() {
            return p;
        }
    }
    // default create location
    if let Some(home) = grok_home() {
        home.join("themes/config.toml")
    } else {
        PathBuf::from("themes/config.toml")
    }
}

/// Persist the pointer file (`active = "<name>"` as TOML). Best-effort.
pub fn write_pointer(name: &str) {
    let path = pointer_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let content = format!("active = \"{}\"\n", name.replace('"', "\\\""));
    match std::fs::write(&path, content) {
        Ok(()) => tracing::info!(pointer = %path.display(), active = %name, "custom theme pointer written"),
        Err(e) => tracing::warn!(pointer = %path.display(), error = %e, "failed to write theme pointer"),
    }
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CustomMeta {
    pub name: String,
    pub display: String,
    pub description: String,
    pub requires_truecolor: bool,
    pub path: PathBuf,
}

fn title_case(name: &str) -> String {
    name.split(|c| c == '-' || c == '_' || c == ' ')
        .filter(|s| !s.is_empty())
        .map(|w| {
            let mut ch = w.chars();
            match ch.next() {
                None => String::new(),
                Some(f) => f.to_ascii_uppercase().to_string() + ch.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_raw_file(path: &Path) -> Option<RawFile> {
    let data = std::fs::read_to_string(path).ok()?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    // try format matching extension first, then fallbacks
    let candidates: Vec<fn(&str) -> Option<RawFile>> = match ext.as_str() {
        "toml" => vec![parse_toml, parse_yaml, parse_json],
        "yaml" | "yml" => vec![parse_yaml, parse_toml, parse_json],
        "json" => vec![parse_json, parse_toml, parse_yaml],
        _ => vec![parse_toml, parse_yaml, parse_json],
    };
    for f in candidates {
        if let Some(v) = f(&data) {
            return Some(v);
        }
    }
    None
}

fn parse_toml(s: &str) -> Option<RawFile> {
    toml::from_str::<RawFile>(s).ok()
}
fn parse_yaml(s: &str) -> Option<RawFile> {
    serde_yaml::from_str::<RawFile>(s).ok()
}
fn parse_json(s: &str) -> Option<RawFile> {
    serde_json::from_str::<RawFile>(s).ok()
}

/// List available custom themes under `$GROK_HOME/themes/`.
///
/// Grok ships **no** file themes — every entry here is user-supplied.
pub fn discover() -> Vec<CustomMeta> {
    let mut out: Vec<CustomMeta> = Vec::new();

    let dir = themes_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => {
            out.sort_by(|a, b| a.name.cmp(&b.name));
            return out;
        }
    };
    for ent in entries.flatten() {
        let path = ent.path();
        if !path.is_file() {
            continue;
        }
        let Some(name_os) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if is_pointer_file_name(name_os) {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "toml" | "yaml" | "yml" | "json") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if stem.is_empty() {
            continue;
        }
        let raw = parse_raw_file(&path);
        let display = raw
            .as_ref()
            .and_then(|r| r.meta.as_ref().and_then(|m| m.display.clone()))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| title_case(&stem));
        let description = raw
            .as_ref()
            .and_then(|r| r.meta.as_ref().and_then(|m| m.description.clone()))
            .unwrap_or_default();
        let requires_truecolor = raw
            .as_ref()
            .and_then(|r| r.meta.as_ref().and_then(|m| m.requires_truecolor))
            .unwrap_or(true);
        out.push(CustomMeta {
            name: stem,
            display,
            description,
            requires_truecolor,
            path,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn is_known(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    discover().iter().any(|m| m.name == lower)
}

fn find_path_for_name(name: &str) -> Option<PathBuf> {
    let lower = name.to_ascii_lowercase();
    for m in discover() {
        if m.name == lower {
            return Some(m.path);
        }
    }
    // also try direct file probe
    let dir = themes_dir();
    for ext in ["toml", "yaml", "yml", "json"] {
        let p = dir.join(format!("{lower}.{ext}"));
        if p.exists() {
            return Some(p);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Theme building
// ---------------------------------------------------------------------------

fn apply_color_map(theme: &mut Theme, map: &HashMap<String, String>) {
    for (k, v) in map {
        let key = k.trim().to_ascii_lowercase();
        // skip meta keys if they leaked into flat
        if matches!(key.as_str(), "meta" | "display" | "description" | "requires_truecolor") {
            continue;
        }
        // modifier keys handled separately
        if key.ends_with("_mod") || key.ends_with("_modifier") {
            continue;
        }
        let Some(color) = parse_color(v) else {
            tracing::warn!(key = %k, value = %v, "custom theme: invalid color, skipping");
            continue;
        };
        match key.as_str() {
            "bg_base" => theme.bg_base = color,
            "bg_light" => theme.bg_light = color,
            "bg_dark" => theme.bg_dark = color,
            "bg_highlight" => theme.bg_highlight = color,
            "bg_hover" => theme.bg_hover = color,
            "bg_terminal" => theme.bg_terminal = color,
            "accent_user" => theme.accent_user = color,
            "accent_assistant" => theme.accent_assistant = color,
            "accent_thinking" => theme.accent_thinking = color,
            "accent_tool" => theme.accent_tool = color,
            "accent_system" => theme.accent_system = color,
            "accent_error" => theme.accent_error = color,
            "accent_success" => theme.accent_success = color,
            "accent_running" => theme.accent_running = color,
            "accent_skill" => theme.accent_skill = color,
            "text_primary" => theme.text_primary = color,
            "text_secondary" => theme.text_secondary = color,
            "gray_dim" => theme.gray_dim = color,
            "gray" => theme.gray = color,
            "gray_bright" => theme.gray_bright = color,
            "command" => theme.command = color,
            "path" => theme.path = color,
            "running" => theme.running = color,
            "warning" => theme.warning = color,
            "fuzzy_accent" => theme.fuzzy_accent = color,
            "accent_plan" => theme.accent_plan = color,
            "accent_verify" => theme.accent_verify = color,
            "accent_remember" => theme.accent_remember = color,
            "selection_border" => theme.selection_border = color,
            "hover_border" => theme.hover_border = color,
            "prompt_border" => theme.prompt_border = color,
            "prompt_border_active" => theme.prompt_border_active = color,
            "accent_model" => theme.accent_model = color,
            "scrollbar_bg" => theme.scrollbar_bg = color,
            "scrollbar_fg" => theme.scrollbar_fg = color,
            "diff_delete_bg" => theme.diff_delete_bg = color,
            "diff_delete_fg" => theme.diff_delete_fg = color,
            "diff_insert_bg" => theme.diff_insert_bg = color,
            "diff_insert_fg" => theme.diff_insert_fg = color,
            "diff_equal_fg" => theme.diff_equal_fg = color,
            "diff_gutter_fg" => theme.diff_gutter_fg = color,
            "bg_visual" => theme.bg_visual = color,
            "paste_bg" => theme.paste_bg = color,
            "paste_fg" => theme.paste_fg = color,
            "paste_dim" => theme.paste_dim = color,
            "md_heading_h1" => theme.md_heading_h1 = color,
            "md_heading_h2" => theme.md_heading_h2 = color,
            "md_heading_h3" => theme.md_heading_h3 = color,
            "md_heading_h4" => theme.md_heading_h4 = color,
            "md_heading_h5" => theme.md_heading_h5 = color,
            "md_heading_h6" => theme.md_heading_h6 = color,
            "md_code" => theme.md_code = color,
            "md_task_checked" => theme.md_task_checked = color,
            "md_task_unchecked" => theme.md_task_unchecked = color,
            "md_muted" => theme.md_muted = color,
            "md_code_bg" => theme.md_code_bg = color,
            "md_text" => theme.md_text = color,
            "link_fg" => theme.link_fg = color,
            _ => {
                // unknown key — ignore but log at debug
                tracing::debug!(key = %k, "custom theme: unknown theme key, skipping");
            }
        }
    }
}

fn apply_modifier_map(theme: &mut Theme, map: &HashMap<String, String>) {
    for (k, v) in map {
        let key = k.trim().to_ascii_lowercase();
        if !(key.ends_with("_mod") || key.ends_with("_modifier")) {
            continue;
        }
        let base = key
            .strip_suffix("_mod")
            .or_else(|| key.strip_suffix("_modifier"))
            .unwrap_or(&key);
        let m = parse_modifier(v);
        match base {
            "md_heading_h1" => theme.md_heading_h1_mod = m,
            "md_heading_h2" => theme.md_heading_h2_mod = m,
            "md_heading_h3" => theme.md_heading_h3_mod = m,
            "md_heading_h4" => theme.md_heading_h4_mod = m,
            "md_heading_h5" => theme.md_heading_h5_mod = m,
            "md_heading_h6" => theme.md_heading_h6_mod = m,
            _ => {}
        }
    }
}

/// Load a custom theme by name (case-insensitive). Returns the built `Theme`.
pub fn load(name: &str) -> Option<Theme> {
    let path = find_path_for_name(name)?;
    load_from_path(&path)
}

fn load_from_path(path: &Path) -> Option<Theme> {
    let raw = parse_raw_file(path)?;
    let mut theme = Theme::groknight();
    // collect all color entries: [theme] table + flat top-level
    let mut map = raw.theme.clone();
    for (k, v) in &raw.flat {
        let lk = k.to_ascii_lowercase();
        if lk == "meta" || lk == "theme" {
            continue;
        }
        // flat keys already in map win via [theme] table; don't overwrite
        map.entry(k.clone()).or_insert_with(|| v.clone());
    }
    apply_color_map(&mut theme, &map);
    apply_modifier_map(&mut theme, &map);
    tracing::info!(path = %path.display(), "custom theme loaded");
    Some(theme)
}

/// All known theme names (lowercase) — for error messages / picker.
pub fn all_names() -> Vec<String> {
    discover().into_iter().map(|m| m.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn parse_color_hex() {
        assert_eq!(parse_color("#0f0f0f"), Some(Color::Rgb(15, 15, 15)));
        assert_eq!(parse_color("#fff"), Some(Color::Rgb(255, 255, 255)));
        assert_eq!(parse_color("none"), Some(Color::Reset));
    }

    #[test]
    fn parse_modifier_bold() {
        assert!(parse_modifier("bold").contains(Modifier::BOLD));
        assert!(parse_modifier("bold,italic").contains(Modifier::ITALIC));
    }

    struct GrokHomeGuard(Option<std::ffi::OsString>);
    impl GrokHomeGuard {
        fn set(path: &std::path::Path) -> Self {
            let prev = std::env::var_os("GROK_HOME");
            unsafe { std::env::set_var("GROK_HOME", path) };
            Self(prev)
        }
    }
    impl Drop for GrokHomeGuard {
        fn drop(&mut self) {
            match &self.0 {
                Some(v) => unsafe { std::env::set_var("GROK_HOME", v) },
                None => unsafe { std::env::remove_var("GROK_HOME") },
            }
        }
    }

    #[test]
    #[serial]
    fn discover_and_load_with_grok_home() {
        let tmp = tempfile::tempdir().unwrap();
        let themes = tmp.path().join("themes");
        std::fs::create_dir_all(&themes).unwrap();
        std::fs::write(
            themes.join("neon.toml"),
            "[meta]\ndisplay = \"Neon\"\n[theme]\nbg_base = \"#0a0a12\"\naccent_user = \"#ff00ff\"\n",
        )
        .unwrap();
        std::fs::write(themes.join("config.toml"), "active = \"neon\"\n").unwrap();
        std::fs::write(
            themes.join("sunset.yaml"),
            "meta:\n  display: Sunset\ntheme:\n  bg_base: \"#1a0f0a\"\n",
        )
        .unwrap();
        let _guard = GrokHomeGuard::set(tmp.path());
        let metas = discover();
        assert!(metas.iter().any(|m| m.name == "neon"), "neon must be discovered: {:?}", metas);
        assert!(metas.iter().any(|m| m.name == "sunset"), "sunset yaml must be discovered");
        let pointer = load_pointer();
        assert_eq!(pointer.as_deref(), Some("neon"));
        let t = load("neon").expect("load neon");
        assert_eq!(t.bg_base, Color::Rgb(0x0a, 0x0a, 0x12));
    }

    /// The canonical Aura palette as a user-supplied theme file. Grok ships
    /// no file themes — this content is what a user puts in
    /// `~/.grok/themes/aura.toml`. Hexes are verbatim from upstream
    /// aura-dark-color-theme.json / aura-colors.css.
    const AURA_USER_TOML: &str = "[meta]\n\
display = \"Aura\"\ndescription = \"Dark purple\"\nrequires_truecolor = true\n\n\
[theme]\n\
bg_base = \"#15141b\"\nbg_light = \"#2e2b38\"\nbg_dark = \"#110f18\"\n\
bg_highlight = \"#3b334b\"\nbg_hover = \"#3b334b\"\nbg_terminal = \"#15141b\"\n\
accent_user = \"#a277ff\"\naccent_assistant = \"#f694ff\"\n\
accent_thinking = \"#6d6d6d\"\naccent_tool = \"#525156\"\n\
accent_system = \"#82e2ff\"\naccent_error = \"#ff6767\"\n\
accent_success = \"#61ffca\"\naccent_running = \"#a277ff\"\n\
accent_skill = \"#f694ff\"\ntext_primary = \"#edecee\"\n\
text_secondary = \"#cdccce\"\ngray_dim = \"#4d4d4d\"\ngray = \"#6d6d6d\"\n\
gray_bright = \"#adacae\"\ncommand = \"#ffca85\"\npath = \"#82e2ff\"\n\
running = \"#61ffca\"\nwarning = \"#ffca85\"\nfuzzy_accent = \"#a277ff\"\n\
accent_plan = \"#ffca85\"\naccent_verify = \"#a277ff\"\n\
accent_remember = \"#61ffca\"\nselection_border = \"#3d375e\"\n\
hover_border = \"#3b334b\"\nprompt_border = \"#3b334b\"\n\
prompt_border_active = \"#a277ff\"\naccent_model = \"#61ffca\"\n\
scrollbar_bg = \"#121016\"\nscrollbar_fg = \"#312e46\"\n\
diff_delete_bg = \"#321a25\"\ndiff_delete_fg = \"#ff6767\"\n\
diff_insert_bg = \"#122c29\"\ndiff_insert_fg = \"#61ffca\"\n\
diff_equal_fg = \"#6d6d6d\"\ndiff_gutter_fg = \"#4d4d4d\"\n\
bg_visual = \"#3d375e\"\npaste_bg = \"#121016\"\npaste_fg = \"#cdccce\"\n\
paste_dim = \"#6d6d6d\"\nmd_code_bg = \"#121016\"\nmd_text = \"#edecee\"\n\
link_fg = \"#f694ff\"\n";

    #[test]
    #[serial]
    fn user_supplied_aura_theme_loads_with_upstream_hexes() {
        // Aura is strictly user-supplied: write it into themes/ like a
        // user would, and verify the parsed hexes match upstream.
        let tmp = tempfile::tempdir().unwrap();
        let themes = tmp.path().join("themes");
        std::fs::create_dir_all(&themes).unwrap();
        std::fs::write(themes.join("aura.toml"), AURA_USER_TOML).unwrap();
        let _guard = GrokHomeGuard::set(tmp.path());
        let t = load("aura").expect("user-supplied aura.toml must load");
        assert_eq!(t.bg_base, Color::Rgb(0x15, 0x14, 0x1b)); // editor.background #15141b
        assert_eq!(t.bg_terminal, Color::Rgb(0x15, 0x14, 0x1b));
        assert_eq!(t.bg_dark, Color::Rgb(0x11, 0x0f, 0x18)); // sideBar.background #110f18
        assert_eq!(t.accent_user, Color::Rgb(0xa2, 0x77, 0xff)); // purple #a277ff
        assert_eq!(t.accent_assistant, Color::Rgb(0xf6, 0x94, 0xff)); // pink #f694ff
        assert_eq!(t.accent_system, Color::Rgb(0x82, 0xe2, 0xff)); // blue #82e2ff
        assert_eq!(t.text_primary, Color::Rgb(0xed, 0xec, 0xee)); // foreground #edecee
        assert_eq!(t.text_secondary, Color::Rgb(0xcd, 0xcc, 0xce)); // dropdown.foreground
        let metas = discover();
        let aura = metas.iter().find(|m| m.name == "aura").expect("aura in discover");
        assert_eq!(aura.display, "Aura");
    }

    #[test]
    #[serial]
    fn pointer_seeds_cache_and_current_serves_aura_background() {
        // End-to-end: themes/config.toml active="aura" -> cache custom overlay
        // -> Theme::current() renders the file theme's bg_base (#15141b),
        // not GrokNight's #141414. Regression for "background never changes".
        let tmp = tempfile::tempdir().unwrap();
        let themes = tmp.path().join("themes");
        std::fs::create_dir_all(&themes).unwrap();
        std::fs::write(themes.join("aura.toml"), AURA_USER_TOML).unwrap();
        std::fs::write(themes.join("config.toml"), "active = \"aura\"\n").unwrap();
        let _guard = GrokHomeGuard::set(tmp.path());
        crate::theme::cache::reset_for_test();
        let t = crate::theme::Theme::current();
        assert!(
            matches!(t.bg_base, Color::Rgb(0x15, 0x14, 0x1b)),
            "Theme::current().bg_base must be aura #15141b, got {:?}",
            t.bg_base
        );
        assert_eq!(crate::theme::cache::current_name(), "aura");
        assert!(crate::theme::cache::is_custom());
        crate::theme::cache::reset_for_test();
    }

    #[tokio::test]
    #[serial]
    async fn watcher_pointer_live_reload() {
        let tmp = tempfile::tempdir().unwrap();
        let themes = tmp.path().join("themes");
        std::fs::create_dir_all(&themes).unwrap();
        std::fs::write(themes.join("a.toml"), "[theme]\nbg_base = \"#111111\"\n").unwrap();
        std::fs::write(themes.join("b.toml"), "[theme]\nbg_base = \"#222222\"\n").unwrap();
        std::fs::write(themes.join("config.toml"), "active = \"a\"\n").unwrap();
        let _guard = GrokHomeGuard::set(tmp.path());
        let mut w = crate::theme::watcher::ThemeWatcher::start().expect("watcher start");
        w.settle().await;
        std::fs::write(themes.join("config.toml"), "active = \"b\"\n").unwrap();
        eprintln!("[watcher-test] t={} wrote b", now_millis());
        let ev = tokio::time::timeout(std::time::Duration::from_secs(4), w.changed())
            .await
            .expect("watcher timeout")
            .expect("watcher error");
        match ev {
            crate::theme::watcher::ThemeWatcherEvent::PointerChanged(name) => {
                assert_eq!(name, "b");
            }
            other => panic!("expected PointerChanged, got {:?}", other),
        }
    }

    #[tokio::test]
    #[serial]
    async fn watcher_file_content_live_reload() {
        let tmp = tempfile::tempdir().unwrap();
        let themes = tmp.path().join("themes");
        std::fs::create_dir_all(&themes).unwrap();
        std::fs::write(themes.join("live.toml"), "[theme]\nbg_base = \"#111111\"\n").unwrap();
        std::fs::write(themes.join("config.toml"), "active = \"live\"\n").unwrap();
        let _guard = GrokHomeGuard::set(tmp.path());
        let mut w = crate::theme::watcher::ThemeWatcher::start().expect("watcher start");
        w.settle().await;
        std::fs::write(themes.join("live.toml"), "[theme]\nbg_base = \"#ff0000\"\n").unwrap();
        let mut saw = false;
        for _ in 0..6 {
            let ev = tokio::time::timeout(std::time::Duration::from_secs(3), w.changed()).await;
            if let Ok(Ok(crate::theme::watcher::ThemeWatcherEvent::ThemeFileChanged(n))) = ev {
                if n == "live" {
                    saw = true;
                    let t = load("live").unwrap();
                    assert_eq!(t.bg_base, Color::Rgb(0xff, 0x00, 0x00));
                    break;
                }
            }
        }
        assert!(saw, "expected ThemeFileChanged for live");
    }
}
