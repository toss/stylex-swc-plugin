use std::{
  env,
  path::{Path, PathBuf},
};

use path_clean::PathClean;

#[allow(dead_code)]
fn get_root_dir(test_path: &Path) -> PathBuf {
  if env::var("original_root_dir").is_err() {
    unsafe { env::set_var("original_root_dir", env::current_dir().unwrap()) };
  }

  let new_cwd = PathBuf::from(env::var("original_root_dir").unwrap())
    .join("fixtures")
    .join(test_path)
    .clean();

  env::set_current_dir(&new_cwd).expect("Failed to set current directory");

  new_cwd
}

#[allow(dead_code)]
fn fixture(test_path: &PathBuf, part: &str) -> PathBuf {
  PathBuf::from(
    env::var("original_root_dir").unwrap_or(env::current_dir().unwrap().display().to_string()),
  )
  .join("fixtures")
  .join(test_path)
  .join(part)
  .clean()
}

#[cfg(test)]
mod resolve_path_application_pnpm_tests {
  use path_clean::PathClean;
  use rustc_hash::FxHashMap;

  use crate::resolvers::{resolve_file_path, tests::get_root_dir};

  use std::path::PathBuf;

  #[test]
  fn resolve_regular_local_import_from_src() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "../colors.stylex.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = Default::default();

    let expected_result = format!("{}/{}", root_path, "src/colors.stylex.js");

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_regular_local_import_from_same_level_directory() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "../components/button.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = Default::default();

    let expected_result = format!("{}/{}", root_path, "src/components/button.js");

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_regular_local_import_from_alias() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "@/components/button.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert("@/*".to_string(), vec![format!("{}/src/*", root_path)]);

    let expected_result = format!("{}/{}", root_path, "src/components/button.js");

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_regular_local_import_from_workspace_alias() {
    let test_path = PathBuf::from("workspace-pnpm");

    let import_path_str = "@/components/button";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert(
      "@/*".to_string(),
      vec![format!(
        "{}",
        PathBuf::from(&root_path)
          .join("../application-pnpm/src/*")
          .clean()
          .to_string_lossy()
      )],
    );

    let expected_result = format!(
      "{}",
      PathBuf::from(&root_path)
        .join("../application-pnpm/src/components/button.js")
        .clean()
        .to_string_lossy(),
    );

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_regular_external_import() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "stylex-lib-dist-main";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-main/dist/index.jsx"
    );

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_regular_external_import_with_exports_dist() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "stylex-lib-dist-exports-with-main/colors.stylex";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-with-main/dist/colors.stylex.js",
    );
    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_package_with_pnpm_path() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "stylex-lib-pnpm";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path,
      "node_modules/.pnpm/stylex-lib-pnpm@0.1.0/node_modules/stylex-lib-pnpm/dist/index.jsx"
    );

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_organisation_package_with_pnpm_path() {
    let test_path = PathBuf::from("application-pnpm");

    let import_path_str = "@stylex/lib-exports-pnpm/colors.stylex";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path,
      "node_modules/.pnpm/@stylex+lib-exports-pnpm@0.1.0/node_modules/@stylex/lib-exports-pnpm/dist/colors.stylex.js"
    );

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }
}

#[cfg(test)]
mod resolve_path_application_npm_tests {
  use path_clean::PathClean;
  use rustc_hash::FxHashMap;

  use crate::resolvers::{resolve_file_path, tests::get_root_dir};

  use std::path::PathBuf;

  #[test]
  fn resolve_regular_local_import_from_src() {
    let test_path = PathBuf::from("application-npm/apps/web");

    let import_path_str = "../colors.stylex.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = Default::default();

    let expected_result = format!("{}/{}", root_path, "src/colors.stylex.js");

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_regular_local_import_from_same_level_directory() {
    let test_path = PathBuf::from("application-npm/apps/web");

    let import_path_str = "../components/button.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = Default::default();

    let expected_result = format!("{}/{}", root_path, "src/components/button.js");

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_regular_local_import_from_alias() {
    let test_path = PathBuf::from("application-npm/apps/web");

    let import_path_str = "@/components/button.js";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert("@/*".to_string(), vec![format!("{}/src/*", root_path)]);

    let expected_result = format!("{}/{}", root_path, "src/components/button.js");

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_regular_local_import_from_workspace_alias() {
    let test_path = PathBuf::from("workspace-npm/apps/web");

    let import_path_str = "@/components/button";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let mut aliases = FxHashMap::default();
    aliases.insert(
      "@/*".to_string(),
      vec![format!(
        "{}",
        PathBuf::from(&root_path)
          .join("../../../application-npm/apps/web/src/*")
          .clean()
          .to_string_lossy()
      )],
    );

    let expected_result = format!(
      "{}",
      PathBuf::from(&root_path)
        .join("../../../application-npm/apps/web/src/components/button.js")
        .clean()
        .to_string_lossy(),
    );

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_regular_external_import() {
    let test_path = PathBuf::from("application-npm/apps/web");

    let import_path_str = "stylex-lib-dist-main";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path).display().to_string();
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path.replace("/apps/web", ""),
      "node_modules/stylex-lib-dist-main/dist/index.jsx"
    );

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_regular_external_import_with_exports_dist() {
    let test_path = PathBuf::from("application-npm/apps/web");

    let import_path_str = "stylex-lib-dist-exports-with-main/colors.stylex";
    let source_file_path = format!(
      "{}/src/pages/home.js",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path)
      .display()
      .to_string()
      .replace("/apps/web", "");
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-with-main/dist/colors.stylex.js",
    );

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }
}
#[cfg(test)]
mod resolve_nested_external_imports_tests {
  use rustc_hash::FxHashMap;

  use crate::{
    package_json::find_closest_node_modules,
    resolvers::{resolve_file_path, tests::get_root_dir},
  };

  use std::path::PathBuf;

  #[test]
  fn resolve_regular_nested_import() {
    let test_path = PathBuf::from("exports/node_modules/stylex-lib-dist-main");

    let import_path_str = "stylex-lib-dist-exports-with-main/colors.stylex";
    let source_file_path = format!(
      "{}/dist/index.jsx",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path)
      .display()
      .to_string()
      .replace("/node_modules/stylex-lib-dist-main", "");
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-with-main/dist/colors.stylex.js"
    );

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );
  }

  #[test]
  fn resolve_nested_import_with_exports_and_nested_node_modules() {
    let test_path = PathBuf::from("exports/node_modules/stylex-lib-dist-main");

    let import_path_str = "stylex-lib-dist-exports/colors.stylex";
    let source_file_path = format!(
      "{}/dist/index.jsx",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path)
      .display()
      .to_string()
      .replace("/node_modules/stylex-lib-dist-main", "");
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports/dist/colors.stylex.js"
    );

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );

    let test_nested_package_path = &get_root_dir(&test_path);

    let closest_node_modules = find_closest_node_modules(test_nested_package_path);

    assert_eq!(
      closest_node_modules
        .unwrap_or_default()
        .display()
        .to_string(),
      test_nested_package_path
        .join("node_modules")
        .to_string_lossy()
    );
  }

  #[test]
  fn resolve_commonjs_exports() {
    let test_path = PathBuf::from("exports");

    let import_path_str = "stylex-lib-dist-exports-commonjs-esm/colors.stylex";
    let source_file_path = format!(
      "{}/dist/index.jsx",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path)
      .display()
      .to_string()
      .replace("/node_modules/stylex-lib-dist-exports-commonjs-esm", "");
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-commonjs-esm/dist/colors.stylex.cjs"
    );

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );

    let test_nested_package_path = &get_root_dir(&test_path);

    let closest_node_modules = find_closest_node_modules(test_nested_package_path);

    assert_eq!(
      closest_node_modules
        .unwrap_or_default()
        .display()
        .to_string(),
      test_nested_package_path
        .join("node_modules")
        .to_string_lossy()
    );
  }

  #[test]
  fn resolve_esm_exports() {
    let test_path = PathBuf::from("exports");

    let import_path_str = "stylex-lib-dist-exports-commonjs-esm";
    let source_file_path = format!(
      "{}/dist/index.jsx",
      get_root_dir(&test_path).as_path().display()
    );
    let root_path = get_root_dir(&test_path)
      .display()
      .to_string()
      .replace("/node_modules/stylex-lib-dist-exports-commonjs-esm", "");
    let aliases = FxHashMap::default();

    let expected_result = format!(
      "{}/{}",
      root_path, "node_modules/stylex-lib-dist-exports-commonjs-esm/dist/index.js"
    );

    assert_eq!(
      resolve_file_path(import_path_str, source_file_path.as_str(), &aliases)
        .unwrap_or_default()
        .display()
        .to_string(),
      expected_result
    );

    let test_nested_package_path = &get_root_dir(&test_path);

    let closest_node_modules = find_closest_node_modules(test_nested_package_path);

    assert_eq!(
      closest_node_modules
        .unwrap_or_default()
        .display()
        .to_string(),
      test_nested_package_path
        .join("node_modules")
        .to_string_lossy()
    );
  }
}
