//! Live-reload watcher for custom themes.
//!
//! Watches the pointer file (`themes/config.toml` etc.) and every
//! `themes/<name>.{toml,yaml,yml,json}` theme file. Debounced; emits a
//! `ThemeWatcherEvent` on the watch channel.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{DebounceEventResult, new_debouncer};
use tokio::sync::watch;

#[derive(Debug, Clone)]
pub enum ThemeWatcherEvent {
    PointerChanged(String),
    ThemeFileChanged(String),
    ThemeListChanged,
}

pub struct ThemeWatcher {
    rx: watch::Receiver<Option<ThemeWatcherEvent>>,
    // keep debouncer alive
    _debouncer: Box<dyn Send>,
    _tmp: Arc<()>,
}

fn pointer_candidates_for_home(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join("themes/config.toml"),
        home.join("themes/theme.toml"),
        home.join("themes/theme-config.toml"),
        home.join("themes/theme-config.yaml"),
        home.join("themes/theme-config.yml"),
        home.join("themes/config.yaml"),
        home.join("themes/config.yml"),
        home.join("themes/theme.yaml"),
        home.join("theme-config.toml"),
        home.join("theme-config.yaml"),
        home.join("theme-config.yml"),
        home.join("theme.toml"),
        home.join("theme.yaml"),
        home.join("themes/config.json"),
        home.join("theme-config.json"),
    ]
}

fn themes_dir_for_home(home: &Path) -> PathBuf {
    home.join("themes")
}

fn file_stem_lower(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
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

fn resolve_home_for_watcher() -> Option<PathBuf> {
    if let Some(v) = std::env::var_os("GROK_HOME") {
        if !v.is_empty() {
            return Some(PathBuf::from(v));
        }
    }
    xai_grok_config::user_grok_home()
}

impl ThemeWatcher {
    pub fn start() -> std::io::Result<Self> {
        let Some(home) = resolve_home_for_watcher() else {
            // no home — return a never-firing watcher
            let (tx, rx) = watch::channel(None);
            // keep tx alive
            std::mem::forget(tx);
            return Ok(Self {
                rx,
                _debouncer: Box::new(()),
                _tmp: Arc::new(()),
            });
        };

        let themes_dir = themes_dir_for_home(&home);
        let _ = std::fs::create_dir_all(&themes_dir);

        // seed channel with None; events go as Some(event)
        let (tx, rx) = watch::channel(None::<ThemeWatcherEvent>);
        let tx_clone = tx.clone();
        let home_clone = home.clone();
        let themes_dir_clone = themes_dir.clone();

        // collect all existing mtimes for list-change detection
        let known: Arc<std::sync::Mutex<std::collections::HashSet<String>>> =
            Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
        if let Ok(entries) = std::fs::read_dir(&themes_dir) {
            let mut set = known.lock().unwrap();
            for e in entries.flatten() {
                if let Some(s) = e.file_name().to_str() {
                    set.insert(s.to_ascii_lowercase());
                }
            }
        }

        let known2 = Arc::clone(&known);
        let home2 = home.clone();
        let mut debouncer = new_debouncer(
            Duration::from_millis(120),
            move |res: DebounceEventResult| {
                let events = match res {
                    Ok(evts) => evts,
                    Err(e) => {
                        tracing::warn!(error = ?e, "theme watcher debouncer error");
                        return;
                    }
                };
                for evt in events {
                    // Directories only matter as containers; file/dir mtime
                    // churn otherwise misclassifies (e.g. dir named `themes`
                    // starts_with("theme")) as a pointer change.
                    if evt.path.is_dir() {
                        continue;
                    }
                    // Live-preview guard: any in-flight file event landing
                    // within 750ms of a preview keystroke is suppressed —
                    // it would re-apply the committed theme over the
                    // previewed one (flash-then-revert).
                    if crate::theme::custom::preview_active() {
                        tracing::debug!(path = %evt.path.display(), "theme watcher: event suppressed (preview in progress)");
                        continue;
                    }
                    let path = &evt.path;
                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();

                    // Pointer classification — MUST NOT match $GROK_HOME/config.toml
                    // itself: that file is written by PersistSetting on every theme
                    // commit, and treating it as a pointer made the watcher re-apply
                    // the committed theme ~120ms later — landing MID-PREVIEW and
                    // snapping the picker back (the "preview flashes then reverts"
                    // bug). A pointer is either a theme-named file in $GROK_HOME
                    // root, or any pointer-named file under themes/.
                    let under_themes = path.starts_with(&home2.join("themes"));
                    let is_pointer = (path.parent() == Some(home2.as_path())
                        && file_name.starts_with("theme"))
                        || (under_themes && is_pointer_file_name(&file_name));

                    if is_pointer {
                        // pointer changed — re-read active name
                        if let Some(active) = crate::theme::custom::load_pointer() {
                            tracing::info!(active = %active, path = %path.display(), "theme watcher: pointer changed");
                            let _ = tx_clone.send(Some(ThemeWatcherEvent::PointerChanged(active)));
                        } else {
                            tracing::info!(path = %path.display(), "theme watcher: pointer cleared/removed");
                            let _ = tx_clone.send(Some(ThemeWatcherEvent::ThemeListChanged));
                        }
                        continue;
                    }

                    // theme file
                    let ext_ok = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "toml" | "yaml" | "yml" | "json"))
                        .unwrap_or(false);
                    if !ext_ok {
                        continue;
                    }
                    // only files inside themes/
                    if !path.starts_with(&home2.join("themes")) {
                        continue;
                    }
                    let stem = file_stem_lower(path);
                    if stem.is_empty() {
                        continue;
                    }
                    // detect list changes (create/remove) vs content changes
                    let mut set = known2.lock().unwrap();
                    let existed = set.contains(&file_name);
                    let exists_now = path.exists();
                    if !existed && exists_now {
                        set.insert(file_name.clone());
                        tracing::info!(theme = %stem, path = %path.display(), "theme watcher: new theme file");
                        let _ = tx_clone.send(Some(ThemeWatcherEvent::ThemeListChanged));
                        // also notify file changed for the new theme
                        let _ = tx_clone.send(Some(ThemeWatcherEvent::ThemeFileChanged(stem)));
                    } else if existed && !exists_now {
                        set.remove(&file_name);
                        tracing::info!(theme = %stem, "theme watcher: theme file removed");
                        let _ = tx_clone.send(Some(ThemeWatcherEvent::ThemeListChanged));
                    } else if exists_now {
                        tracing::info!(theme = %stem, path = %path.display(), "theme watcher: theme file modified");
                        let _ = tx_clone.send(Some(ThemeWatcherEvent::ThemeFileChanged(stem)));
                    }
                }
            },
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("debouncer: {e:?}")))?;

        // watch themes/ dir non-recursively, and watch each pointer candidate's parent
        let mut watched_parents = std::collections::HashSet::new();
        let mut watch_dir = |d: &Path, debouncer: &mut notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>| {
            if watched_parents.insert(d.to_path_buf()) && d.exists() {
                let _ = debouncer.watcher().watch(d, RecursiveMode::NonRecursive);
                tracing::info!(dir = %d.display(), "theme watcher: watching dir");
            }
        };

        watch_dir(&themes_dir_clone, &mut debouncer);
        // also watch $GROK_HOME itself for root-level pointer files (theme-config.*)
        watch_dir(&home_clone, &mut debouncer);

        // keep pointer candidates' parents watched (already covered above)
        for p in pointer_candidates_for_home(&home_clone) {
            if let Some(parent) = p.parent() {
                watch_dir(parent, &mut debouncer);
            }
        }

        // also watch individual theme files explicitly for editors that use atomic rename
        if let Ok(entries) = std::fs::read_dir(&themes_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_file() {
                    let ext_ok = p
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "toml" | "yaml" | "yml" | "json"))
                        .unwrap_or(false);
                    if ext_ok {
                        let _ = debouncer.watcher().watch(&p, RecursiveMode::NonRecursive);
                    }
                }
            }
        }

        Ok(Self {
            rx,
            _debouncer: Box::new(debouncer),
            _tmp: Arc::new(()),
        })
    }

    /// Wait for the next theme watcher event.
    pub async fn changed(&mut self) -> Result<ThemeWatcherEvent, watch::error::RecvError> {
        loop {
            self.rx.changed().await?;
            let v = self.rx.borrow().clone();
            if let Some(ev) = v {
                return Ok(ev);
            }
            // None is initial value — keep waiting
        }
    }

    /// Drain the startup event burst.
    ///
    /// Arming the watches delivers the pre-existing files' state as an
    /// initial debounced batch (~120–250ms after `start()`). Without
    /// draining, that stale batch queues `PointerChanged`/`ThemeFileChanged`
    /// events for whatever was on disk at launch — and the first real
    /// navigation then consumed a STALE event instead of a fresh one.
    /// Call once shortly after `start()` (event loop startup).
    pub async fn settle(&mut self) {
        for _ in 0..3 {
            tokio::time::sleep(Duration::from_millis(160)).await;
            while self.rx.has_changed().unwrap_or(false) {
                let _ = self.rx.borrow_and_update();
            }
        }
    }

    /// Non-blocking check for current pending event (for tests).
    pub fn try_recv(&mut self) -> Option<ThemeWatcherEvent> {
        self.rx.borrow().clone()
    }
}
