use oxc_resolver::{AliasValue, ResolveOptions, Resolver};
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

mod tests;

pub const EXTENSIONS: [&str; 8] = [".tsx", ".ts", ".jsx", ".js", ".mjs", ".cjs", ".mdx", ".md"];

pub fn resolve_file_path(
  import_path_str: &str,
  source_file_path: &str,
  aliases: &FxHashMap<String, Vec<String>>,
) -> std::io::Result<PathBuf> {
  let resolver_options = ResolveOptions {
    extensions: EXTENSIONS.iter().map(|ext| ext.to_string()).collect(),
    condition_names: vec![
      "node".into(),
      "import".into(),
      "require".into(),
      "default".into(),
    ],
    ..ResolveOptions::default()
  };

  let resolver_without_aliases = Resolver::new(resolver_options.clone());
  let resolver_with_aliases = Resolver::new(ResolveOptions {
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
    ..resolver_options
  });

  let source_directory_path = Path::new(source_file_path)
    .parent()
    .unwrap()
    .to_str()
    .unwrap();

  if import_path_str.starts_with(".")
    && let Ok(resolution) =
      resolver_without_aliases.resolve(source_directory_path, &import_path_str)
  {
    return Ok(resolution.into_path_buf());
  }

  match resolver_with_aliases.resolve(source_file_path, &import_path_str) {
    Ok(resolution) => Ok(resolution.into_path_buf()),
    Err(_) => Err(std::io::Error::new(
      std::io::ErrorKind::NotFound,
      "File not found",
    )),
  }
}
