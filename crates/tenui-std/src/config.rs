//! Poll-based config/theme hot-reload.
//!
//! A step toward the spec's hot-loaded `config.lua` hooks: watch a file and surface its
//! contents whenever they change, so the app can live-swap the active `ThemePalette` (or
//! any serialized config, e.g. a workspace `.ron`) without a restart. Change detection is
//! content-hash based (robust against coarse filesystem mtime resolution). Poll it once per
//! frame or on an interval.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

/// Watches a single config file for content changes.
#[derive(Debug, Clone)]
pub struct ConfigWatcher {
    path: PathBuf,
    last_hash: Option<u64>,
}

impl ConfigWatcher {
    /// Creates a watcher for `path`. The first [`poll`](Self::poll) returns the current
    /// contents if the file exists.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            last_hash: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the file's contents if they changed since the last poll (or on first poll),
    /// else `None`. Returns `None` when the file is missing or unreadable.
    pub fn poll(&mut self) -> Option<String> {
        let contents = std::fs::read_to_string(&self.path).ok()?;
        let mut hasher = DefaultHasher::new();
        contents.hash(&mut hasher);
        let hash = hasher.finish();
        if self.last_hash == Some(hash) {
            return None;
        }
        self.last_hash = Some(hash);
        Some(contents)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("tenui_cfg_{}_{}.ron", tag, nanos))
    }

    #[test]
    fn test_watcher_detects_changes_only() {
        let path = temp_path("watch");
        std::fs::write(&path, "theme: \"nord\"").unwrap();

        let mut w = ConfigWatcher::new(&path);
        assert_eq!(w.poll().as_deref(), Some("theme: \"nord\"")); // first read
        assert_eq!(w.poll(), None); // unchanged

        std::fs::write(&path, "theme: \"gruvbox\"").unwrap();
        assert_eq!(w.poll().as_deref(), Some("theme: \"gruvbox\"")); // changed
        assert_eq!(w.poll(), None);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_missing_file_is_none() {
        let mut w = ConfigWatcher::new(temp_path("missing"));
        assert_eq!(w.poll(), None);
    }
}
