use core::panic;
use log::{debug, warn};
use once_cell::sync::Lazy;
use oxc_resolver::{AliasValue, ResolveOptions, Resolver};
use path_clean::PathClean;
use regex::Regex;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use swc_core::{
  common::FileName,
  ecma::loader::{
    resolve::{Resolution, Resolve},
    resolvers::node::NodeModulesResolver,
  },
};

use std::fs;

use crate::{
  enums::ExportsType,
  file_system::get_directory_path_recursive,
  package_json::{
    PackageJsonExtended, find_closest_node_modules, find_closest_package_json_folder,
    get_package_json, get_package_json_with_deps,
  },
  utils::{contains_subpath, relative_path, sort_export_paths_by_priority},
};

mod tests;

pub const EXTENSIONS: [&str; 8] = [".tsx", ".ts", ".jsx", ".js", ".mjs", ".cjs", ".mdx", ".md"];

pub static FILE_PATTERN: Lazy<Regex> =
  Lazy::new(|| Regex::new(r#"\.(jsx?|tsx?|mdx?|mjs|cjs)$"#).unwrap());

pub fn resolve_path(
  processing_file: &Path,
  root_dir: &Path,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> String {
  if !FILE_PATTERN.is_match(processing_file.to_str().unwrap()) {
    let processing_path = if cfg!(test) {
      processing_file
        .strip_prefix(root_dir.parent().unwrap().parent().unwrap())
        .unwrap()
        .to_path_buf()
    } else {
      processing_file.to_path_buf()
    };

    panic!(
      r#"Resolve path must be a file, but got: {}"#,
      processing_path.display()
    );
  }

  let cwd = if cfg!(test) {
    root_dir.to_path_buf()
  } else {
    "cwd".into()
  };

  let mut path_by_package_json =
    match resolve_from_package_json(processing_file, root_dir, &cwd, package_json_seen) {
      Ok(value) => value,
      Err(value) => return value,
    };

  if path_by_package_json.starts_with(&cwd) {
    path_by_package_json = path_by_package_json
      .strip_prefix(cwd)
      .unwrap()
      .to_path_buf();
  }

  let resolved_path_by_package_name = path_by_package_json.clean().display().to_string();

  if cfg!(test) {
    let cwd_resolved_path = format!("{}/{}", root_dir.display(), resolved_path_by_package_name);

    assert!(
      fs::metadata(&cwd_resolved_path).is_ok(),
      "Path resolution failed: {}",
      resolved_path_by_package_name
    );
  }

  resolved_path_by_package_name
}

fn resolve_from_package_json(
  processing_file: &Path,
  root_dir: &Path,
  cwd: &Path,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> Result<PathBuf, String> {
  let resolved_path = match processing_file.strip_prefix(root_dir) {
    Ok(stripped) => stripped.to_path_buf(),
    Err(_) => {
      let processing_file_str = processing_file.to_string_lossy();

      if let Some(node_modules_index) = processing_file_str.rfind("node_modules") {
        // NOTE: This is a workaround for the case when the file is located in the node_modules directory and pnpm is package manager

        let resolved_path_from_node_modules = processing_file_str
          .split_at(node_modules_index)
          .1
          .to_string();

        if !resolved_path_from_node_modules.is_empty() {
          return Err(resolved_path_from_node_modules);
        }
      }

      let relative_package_path = relative_path(processing_file, root_dir);

      get_package_path_by_package_json(cwd, &relative_package_path, package_json_seen)
    }
  };

  Ok(resolved_path)
}

fn get_package_path_by_package_json(
  cwd: &Path,
  relative_package_path: &Path,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> PathBuf {
  let (resolver, package_dependencies) = get_package_json_with_deps(cwd, package_json_seen);

  let mut potential_package_path: PathBuf = PathBuf::default();

  for (name, _) in package_dependencies.iter() {
    let file_name = FileName::Real(cwd.to_path_buf());

    let potential_path_section = name.split("/").last().unwrap_or_default();

    if contains_subpath(relative_package_path, Path::new(&potential_path_section)) {
      let relative_package_path_str = relative_package_path.display().to_string();

      let potential_file_path = relative_package_path_str
        .split(potential_path_section)
        .last()
        .unwrap_or_default();

      if !potential_file_path.is_empty()
        || relative_package_path_str.ends_with(format!("/{}", potential_path_section).as_str())
      {
        let resolved_node_modules_path =
          get_node_modules_path(&resolver, &file_name, name, package_json_seen);

        if let Some(resolved_node_modules_path) = resolved_node_modules_path
          && let FileName::Real(real_resolved_node_modules_path) =
            resolved_node_modules_path.filename
        {
          potential_package_path = resolve_exports_path(
            &real_resolved_node_modules_path,
            Path::new(potential_file_path),
            package_json_seen,
          );
        }

        if potential_package_path.as_os_str().is_empty() {
          potential_package_path =
            PathBuf::from(format!("node_modules/{}{}", name, potential_file_path));
        }

        break;
      }
    }
  }

  potential_package_path
}

fn resolve_exports_path(
  real_resolved_node_modules_path: &Path,
  potential_file_path: &Path,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> PathBuf {
  let (potential_package_json, _) =
    get_package_json(real_resolved_node_modules_path, package_json_seen);

  match &potential_package_json.exports {
    Some(exports) => resolve_package_json_exports(
      potential_file_path,
      exports,
      real_resolved_node_modules_path,
    ),
    None => {
      let node_modules_regex = Regex::new(r".*node_modules").unwrap();

      node_modules_regex
        .replace(
          real_resolved_node_modules_path
            .display()
            .to_string()
            .as_str(),
          "node_modules",
        )
        .to_string()
        .into()
    }
  }
}

pub(crate) fn get_node_modules_path(
  resolver: &NodeModulesResolver,
  file_name: &FileName,
  name: &str,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> Option<swc_core::ecma::loader::resolve::Resolution> {
  {
    match resolver.resolve(file_name, name) {
      Ok(resolution) => {
        if let FileName::Real(real_filename) = &resolution.filename
          && real_filename.to_string_lossy().contains("node_modules/")
        {
          return Some(resolution);
        }
        None
      }
      Err(_) => get_potential_node_modules_path(file_name, name, package_json_seen),
    }
  }
}

fn get_potential_node_modules_path(
  file_name: &FileName,
  name: &str,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> Option<Resolution> {
  let file_name_real = if let FileName::Real(real_filename) = file_name {
    real_filename
  } else {
    return None;
  };

  let potential_package_path = PathBuf::from(format!(
    "{}/{}",
    find_closest_node_modules(file_name_real)
      .unwrap_or(file_name_real.clone())
      .to_string_lossy(),
    name
  ));

  if let Some(resolved_potential_package_path) =
    get_directory_path_recursive(&potential_package_path)
  {
    let (potential_package_json, _) =
      get_package_json(&resolved_potential_package_path, package_json_seen);

    let package_name = potential_package_json
      .name
      .unwrap_or_else(|| panic!("Package name is not found in package.json of '{}'", name))
      .clone();

    let potential_import_path_segment = name.split(&package_name).last().unwrap_or_default();

    let potential_package_path = resolve_exports_path(
      &resolved_potential_package_path,
      Path::new(potential_import_path_segment),
      package_json_seen,
    );

    let file_name_real_lossy = file_name_real.to_string_lossy();
    let root_subst_file_name = file_name_real_lossy.split("node_modules").next().unwrap();

    let path = Path::new(&potential_package_path);

    let stripped_path = path.strip_prefix(root_subst_file_name).unwrap_or(path);

    return Some(Resolution {
      filename: FileName::Real(stripped_path.to_path_buf()),
      slug: None,
    });
  }

  None
}

fn resolve_package_json_exports(
  potential_import_segment_path: &Path,
  exports: &FxHashMap<String, ExportsType>,
  resolved_node_modules_path: &Path,
) -> PathBuf {
  let mut result: PathBuf = PathBuf::default();

  let mut import_segment_path_without_extension = PathBuf::from(potential_import_segment_path)
    .with_extension("")
    .display()
    .to_string();

  if import_segment_path_without_extension.is_empty() {
    import_segment_path_without_extension = String::from("index.");
  }

  let mut exports_values: Vec<&String> = exports
    .iter()
    .flat_map(|(_, values)| match values {
      ExportsType::Simple(path) => vec![path],
      ExportsType::Complex(map) => map.values().collect(),
    })
    .collect();

  exports_values.sort_by(sort_export_paths_by_priority);

  let resolved_package_path = find_closest_package_json_folder(resolved_node_modules_path)
    .unwrap_or_else(|| {
      panic!(
        "package.json not found near: {}",
        resolved_node_modules_path.display()
      )
    });

  for export_value in exports_values {
    if export_value.contains(&import_segment_path_without_extension) {
      result = resolved_package_path.join(export_value);

      break;
    }
  }

  if result.components().count() == 0 {
    let mut keys: Vec<&String> = exports.keys().collect();

    keys.sort_by_key(|k| (k.to_string(), k.len()));

    for key in keys {
      if key.contains(&import_segment_path_without_extension) {
        let mut export_paths: Vec<&String> = exports
          .get(key)
          .iter()
          .flat_map(|values| match values {
            ExportsType::Simple(path) => vec![path],
            ExportsType::Complex(map) => map.values().collect(),
          })
          .collect();

        export_paths.sort_by(sort_export_paths_by_priority);

        if let Some(export_path) = export_paths.first() {
          result = resolved_package_path.join(export_path);
        }
      }
    }
  }

  if result.components().count() == 0 {
    warn!(
      "Unfortunatly, the exports field is not yet fully supported, so path resolving may work not as expected"
    );
    // TODO: implement exports field resolution
  }

  result
}

pub fn resolve_file_path(
  import_path_str: &str,
  source_file_path: &str,
  root_path: &str,
  aliases: &FxHashMap<String, Vec<String>>,
  package_json_seen: &mut FxHashMap<String, PackageJsonExtended>,
) -> std::io::Result<PathBuf> {
  let source_file_dir = Path::new(source_file_path).parent().unwrap();
  let root_path = Path::new(root_path);

  let cwd_path = Path::new(root_path);

  let resolved_file_paths = if import_path_str.starts_with('.') {
    if FILE_PATTERN.is_match(import_path_str) {
      vec![PathBuf::from(resolve_path(
        &source_file_dir.join(import_path_str),
        root_path,
        package_json_seen,
      ))]
    } else {
      EXTENSIONS
        .iter()
        .map(|ext| {
          let import_path_str_with_ext = format!("{}{}", import_path_str, ext);

          PathBuf::from(resolve_path(
            &source_file_dir.join(import_path_str_with_ext),
            root_path,
            package_json_seen,
          ))
        })
        .collect()
    }
  } else if import_path_str.starts_with('/') {
    vec![root_path.join(import_path_str)]
  } else {
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
      ..ResolveOptions::default()
    };

    match Resolver::new(resolver_options).resolve(source_file_dir, &import_path_str) {
      Err(_) => vec![],
      Ok(resolution) => vec![resolution.into_path_buf()],
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
