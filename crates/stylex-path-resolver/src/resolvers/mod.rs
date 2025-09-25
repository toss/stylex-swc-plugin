use log::debug;
use once_cell::sync::Lazy;
use oxc_resolver::{AliasValue, ResolveOptions, Resolver};
use path_clean::PathClean;
use regex::Regex;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

use std::fs;

mod tests;

pub const EXTENSIONS: [&str; 8] = [".tsx", ".ts", ".jsx", ".js", ".mjs", ".cjs", ".mdx", ".md"];

pub static FILE_PATTERN: Lazy<Regex> =
  Lazy::new(|| Regex::new(r#"\.(jsx?|tsx?|mdx?|mjs|cjs)$"#).unwrap());

pub fn resolve_file_path(
  import_path_str: &str,
  source_file_path: &str,
  root_path: &str,
  aliases: &FxHashMap<String, Vec<String>>,
) -> std::io::Result<PathBuf> {
  let source_file_dir = Path::new(source_file_path).parent().unwrap();
  let root_path = Path::new(root_path);

  let cwd_path = Path::new(root_path);

  let resolved_file_paths = {
    let resolver_options = ResolveOptions {
      alias: aliases
        .iter()
        .map(|(alias, values)| {
          (
            alias.clone(),
            values
              .iter()
              .map(|value| AliasValue::from(value.clone()))
              .collect(),
          )
        })
        .collect(),
      extensions: EXTENSIONS.iter().map(|ext| ext.to_string()).collect(),
      condition_names: vec![
        "node".into(),
        "import".into(),
        "require".into(),
        "default".into(),
      ],
      ..ResolveOptions::default()
    };

    match Resolver::new(resolver_options).resolve(source_file_dir, &import_path_str) {
      Err(err) => {
        println!("err: {:?}", err);
        vec![]
      }
      Ok(resolution) => {
        println!("resolution: {:?}", resolution);
        vec![resolution.into_path_buf()]
      }
    }
  };

  let resolved_potential_file_paths = resolved_file_paths
    .iter()
    .filter(|path| path.as_path() != Path::new("."))
    .collect::<Vec<&PathBuf>>();

  debug!(
    "Resolved potential paths: {:?} for import `{}`",
    resolved_potential_file_paths, import_path_str
  );

  for ext in EXTENSIONS.iter() {
    for resolved_file_path in resolved_potential_file_paths.iter() {
      let mut resolved_file_path = resolved_file_path.clean();

      if let Some(extension) = resolved_file_path.extension() {
        let subpath = extension.to_string_lossy();
        if EXTENSIONS
          .iter()
          .all(|ext| !ext.ends_with(subpath.as_ref()))
        {
          resolved_file_path.set_extension(format!("{}{}", subpath, ext));
        }
      } else {
        resolved_file_path.set_extension(ext.trim_start_matches("."));
      }

      let cleaned_path = resolved_file_path.to_string_lossy().to_string();

      let path_to_check: PathBuf;
      let node_modules_path_to_check: PathBuf;

      if !cleaned_path.contains(root_path.to_str().expect("root path is not valid")) {
        if !cleaned_path.starts_with("node_modules") {
          node_modules_path_to_check = cwd_path.join("node_modules").join(&cleaned_path);
        } else {
          node_modules_path_to_check = cwd_path.join(&cleaned_path);
        }
        path_to_check = cwd_path.join(cleaned_path);
      } else {
        path_to_check = PathBuf::from(&cleaned_path);
        node_modules_path_to_check = path_to_check.clone();
      }

      if fs::metadata(&path_to_check).is_ok() {
        return Ok(path_to_check.to_path_buf().clean());
      }

      if fs::metadata(&node_modules_path_to_check).is_ok() {
        return Ok(node_modules_path_to_check.to_path_buf().clean());
      }
    }
  }

  Err(std::io::Error::new(
    std::io::ErrorKind::NotFound,
    "File not found",
  ))
}
