use std::path::{Path, PathBuf};

pub(crate) fn find_closest_path(path: &Path, target_folder_name: &str) -> Option<PathBuf> {
  let node_modules_path: PathBuf = path.join(target_folder_name);

  if node_modules_path.exists() {
    return Some(node_modules_path);
  }

  match path.parent() {
    Some(parent) => find_closest_path(parent, target_folder_name),
    None => None,
  }
}
