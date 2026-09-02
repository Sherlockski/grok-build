//! Fd-relative helpers + the single owned deleter used by daemon-down `rm`
//! and `clean-artifacts`. Never a weaker sibling of `grove_git::delete_owned`.
pub fn is_safe_worktree_id(id: &str) -> bool {
    if id.is_empty()
        || id.starts_with('.')
        || id.contains('/')
        || id.contains('\\')
        || id.contains('\0')
    {
        return false;
    }
    // Reject whitespace/control and non-alphanumeric/._- to prevent
    // newline/space injection (test expects "wt name\nnewline-deadbeef" invalid).
    if id.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return false;
    }
    true
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
