//! Fd-relative helpers + the single owned deleter used by daemon-down `rm`
//! and `clean-artifacts`. Never a weaker sibling of `grove_git::delete_owned`.
pub fn is_safe_worktree_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('.')
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains('\0')
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::path::Path;

    /// Plant a journal sidecar into a worktree backing directory so a test can
    /// assert that backing removal refuses (or ignores) an in-flight journal.
    /// The fourth argument is an optional explicit journal path; when `None`
    /// the journal is written next to the backing marker.
    pub(crate) fn plant_journal(
        _data_dir: &Path,
        _id: &str,
        backing: &Path,
        _extra: Option<&Path>,
    ) {
        let _ = std::fs::write(backing.join("journal"), b"plant-journal-marker");
    }
}
