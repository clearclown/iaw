use crate::error::{AetherError, Result};
use std::path::{Path, PathBuf};

/// Walk up from `start` until a `.jj` directory is found, returning the repo root.
pub fn find_repo_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.canonicalize()?;

    loop {
        if current.join(".jj").is_dir() {
            return Ok(current);
        }

        current = current
            .parent()
            .ok_or_else(|| AetherError::Config("Not in a jj repository".into()))?
            .to_path_buf();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_repo_root_not_in_repo() {
        // /tmp is not a jj repo
        let result = find_repo_root(Path::new("/tmp"));
        assert!(result.is_err());
    }
}
