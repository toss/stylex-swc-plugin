use std::path::{Path, PathBuf};

use path_clean::PathClean;

pub fn relative_path(file_path: &Path, root: &Path) -> PathBuf {
  pathdiff::diff_paths(file_path, root)
    .unwrap_or_else(|| {
      panic!(
        "Failed to get relative path for file {} based on root {}",
        file_path.display(),
        root.display()
      )
    })
    .clean()
}
