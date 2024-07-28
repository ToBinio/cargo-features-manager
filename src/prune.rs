use color_eyre::Result;
use std::collections::HashMap;

use console::{style, Term};
use std::io::Write;
use std::ops::Not;
use std::path::Path;

use crate::project::dependency::Dependency;
use crate::project::document::Document;
use crate::save::save_dependency;
use crate::util::{get_item_from_doc, toml_document_from_path};
use color_eyre::eyre::{eyre, ContextCompat};
use itertools::Itertools;
use std::process::{Command, Stdio};

pub fn prune(mut document: Document, is_dry_run: bool) -> Result<()> {
    let mut term = Term::stdout();

    let mut enabled_features = get_enabled_features(&document);

    let base_ignored_features =
        get_ignored_features("./", "workspace.cargo-features-manager.keep")?;
    remove_ignored_features(&document, &base_ignored_features, &mut enabled_features)?;

    prune_features(
        &mut document,
        is_dry_run,
        &mut term,
        enabled_features,
        known_features()?,
    )?;

    Ok(())
}

//give a map of known features that do not affect completion but remove functionality
pub fn known_features() -> Result<HashMap<String, Vec<String>>> {
    let file = include_str!("../Known-Features.toml");

    let document: toml_edit::DocumentMut = file.parse()?;

    let mut map = HashMap::new();

    for (dependency, features) in document.as_table() {
        let features = features
            .as_array()
            .context("could not parse Known-Features.toml")?;

        let features = features
            .iter()
            .filter_map(|item| item.as_str())
            .map(|name| name.to_string())
            .collect_vec();

        map.insert(dependency.to_string(), features);
    }

    Ok(map)
}

type FeaturesToTest = HashMap<String, HashMap<String, Vec<String>>>;

fn get_enabled_features(document: &Document) -> FeaturesToTest {
    let mut data = HashMap::new();

    for package in document.get_packages() {
        let mut package_data = HashMap::new();

        for dependency in package.get_deps() {
            let enabled_features = dependency
                .features
                .iter()
                .filter(|(_name, data)| data.is_toggleable() && data.is_enabled())
                .map(|(name, _data)| name)
                .cloned()
                .collect::<Vec<String>>();

            if enabled_features.is_empty().not() {
                package_data.insert(dependency.get_name().clone(), enabled_features);
            }
        }

        if package_data.is_empty().not() {
            data.insert(package.name.clone(), package_data);
        }
    }

    data
}

fn remove_ignored_features(
    document: &Document,
    base_ignored: &HashMap<String, Vec<String>>,
    enabled_features: &mut FeaturesToTest,
) -> Result<()> {
    for (package_name, dependencies) in enabled_features {
        let package = document.get_package(package_name)?;

        let ignored_features = get_ignored_features(
            package.manifest_path.trim_end_matches("/Cargo.toml"),
            "cargo-features-manager.keep",
        )?;

        for (dependency_name, features) in dependencies {
            let dependency = package.get_dep(dependency_name)?;

            if dependency.can_use_default() {
                features.push("default".to_string());
            }

            for feature in ignored_features.get(&dependency.name).unwrap_or(&vec![]) {
                remove_feature(feature, features, dependency);
            }
            for feature in base_ignored.get(&dependency.name).unwrap_or(&vec![]) {
                remove_feature(feature, features, dependency);
            }

            if let Some(index) = features.iter().position(|name| name == "default") {
                features.remove(index);
            }
        }
    }

    Ok(())
}

fn remove_feature(feature: &String, features: &mut Vec<String>, dependency: &Dependency) {
    let index = features.iter().position(|name| name == feature);

    let Some(index) = index else {
        return;
    };

    features.remove(index);

    if let Some(feature) = dependency.get_feature(feature) {
        for sub_feature in &feature.sub_features {
            remove_feature(&sub_feature.name, features, dependency);
        }
    }
}

fn prune_features(
    document: &mut Document,
    is_dry_run: bool,
    term: &mut Term,
    features: FeaturesToTest,
    known_features: HashMap<String, Vec<String>>,
) -> Result<()> {
    let feature_count = features
        .values()
        .flat_map(|dependencies| dependencies.values())
        .flatten()
        .count();

    let mut has_known_features_enabled = false;

    let mut checked_features_count = 0;

    writeln!(
        term,
        "workspace [{}/{}]",
        checked_features_count, feature_count
    )?;

    let mut offset_to_top = 1;

    let package_inset = if features.len() == 1 { 0 } else { 2 };
    let dependency_inset = if features.len() == 1 { 2 } else { 4 };

    for (package_name, dependencies) in features
        .into_iter()
        .sorted_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b))
    {
        if dependencies.is_empty() {
            continue;
        }

        let package_feature_count = dependencies.values().flatten().count();
        let mut package_checked_features_count = 0;
        let mut package_offset_to_top = 1;

        if document.is_workspace() {
            term.clear_line()?;
            writeln!(term)?;
            writeln!(
                term,
                "{:package_inset$}{} [{}/{}]",
                "", package_name, package_checked_features_count, package_feature_count
            )?;
            offset_to_top += 2;
        }

        for (dependency_name, features) in dependencies
            .into_iter()
            .sorted_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b))
        {
            if features.is_empty() {
                continue;
            }

            let mut known_features_list = vec![];
            let dependency = document
                .get_package(&package_name)?
                .get_dep(&dependency_name)?;

            for feature_name in known_features.get(&dependency_name).unwrap_or(&vec![]) {
                set_features_to_be_keept(
                    dependency,
                    feature_name.to_string(),
                    &mut known_features_list,
                )
            }

            let mut to_be_disabled = vec![];
            to_be_disabled.append(&mut known_features_list.clone());

            for (id, feature) in features.iter().enumerate() {
                term.clear_line()?;
                writeln!(
                    term,
                    "{:dependency_inset$}{} [{}/{}]",
                    "",
                    dependency_name,
                    id,
                    features.len()
                )?;
                term.clear_line()?;
                writeln!(term, "{:dependency_inset$} └ {}", "", feature)?;

                term.move_cursor_up(2)?;

                document
                    .get_package_mut(&package_name)?
                    .get_dep_mut(&dependency_name)?
                    .disable_feature(feature)?;

                save_dependency(document, &package_name, &dependency_name)?;

                if !to_be_disabled.contains(feature) && check()? {
                    set_features_to_be_disabled(
                        document
                            .get_package(&package_name)?
                            .get_dep(&dependency_name)?,
                        feature.to_string(),
                        &mut to_be_disabled,
                    );
                }

                //reset to start
                for feature in &features {
                    document
                        .get_package_mut(&package_name)?
                        .get_dep_mut(&dependency_name)?
                        .enable_feature(feature)?;
                }

                save_dependency(document, &package_name, &dependency_name)?;

                checked_features_count += 1;
                package_checked_features_count += 1;

                term.move_cursor_up(offset_to_top)?;
                writeln!(
                    term,
                    "workspace [{}/{}]",
                    checked_features_count, feature_count
                )?;
                term.move_cursor_down(offset_to_top - 1)?;

                if document.is_workspace() {
                    term.move_cursor_up(package_offset_to_top)?;
                    writeln!(
                        term,
                        "{:package_inset$}{} [{}/{}]",
                        "", package_name, package_checked_features_count, package_feature_count
                    )?;
                    term.move_cursor_down(package_offset_to_top - 1)?;
                }
            }

            offset_to_top += 1;
            package_offset_to_top += 1;

            let mut disabled_count = style(
                features
                    .iter()
                    .filter(|feature| to_be_disabled.contains(feature))
                    .map(|feature| {
                        if known_features_list.contains(feature) {
                            has_known_features_enabled = true;
                            style(feature).color256(7).to_string()
                        } else {
                            style(format!("-{}", feature)).red().to_string()
                        }
                    })
                    .join(","),
            );

            if to_be_disabled.is_empty() {
                disabled_count = style("0".to_string());
            }

            term.clear_line()?;
            writeln!(
                term,
                "{:dependency_inset$}{} [{}/{}]",
                "",
                dependency_name,
                disabled_count,
                features.len()
            )?;

            if is_dry_run {
                continue;
            }

            if to_be_disabled.is_empty().not() {
                for feature in to_be_disabled {
                    if known_features_list.contains(&feature) {
                        continue;
                    }

                    document
                        .get_package_mut(&package_name)?
                        .get_dep_mut(&dependency_name)?
                        .disable_feature(&feature)?;
                }

                save_dependency(document, &package_name, &dependency_name)?;
            }
        }
    }

    if has_known_features_enabled {
        term.clear_line()?;
        writeln!(term)?;
        writeln!(term, "Some features that do not affect compilation but can limit functionally where found. For more information refer to https://github.com/ToBinio/cargo-features-manager?tab=readme-ov-file#prune")?;
    }

    Ok(())
}

fn set_features_to_be_disabled(
    dependency: &Dependency,
    feature: String,
    to_be_disabled: &mut Vec<String>,
) {
    if to_be_disabled.contains(&feature) {
        return;
    }

    to_be_disabled.push(feature.clone());

    dependency
        .features
        .iter()
        .filter(|(_, data)| {
            data.sub_features
                .iter()
                .any(|sub_feature| sub_feature.name == feature)
        })
        .for_each(|(name, _)| {
            set_features_to_be_disabled(dependency, name.to_string(), to_be_disabled);
        });
}

fn set_features_to_be_keept(
    dependency: &Dependency,
    feature: String,
    to_be_disabled: &mut Vec<String>,
) {
    if to_be_disabled.contains(&feature) {
        return;
    }

    to_be_disabled.push(feature.clone());

    if let Some(feature) = dependency.get_feature(&feature) {
        for sub_feature in &feature.sub_features {
            set_features_to_be_keept(dependency, sub_feature.name.clone(), to_be_disabled);
        }
    }
}

fn check() -> Result<bool> {
    if !build()? {
        return Ok(false);
    }

    if !test()? {
        return Ok(false);
    }

    Ok(true)
}

fn build() -> Result<bool> {
    let mut child = Command::new("cargo")
        .arg("build")
        .arg("--all-targets")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let code = child.wait()?.code().ok_or(eyre!("Could not build"))?;

    Ok(code == 0)
}

fn test() -> Result<bool> {
    let mut child = Command::new("cargo")
        .arg("test")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let code = child.wait()?.code().ok_or(eyre!("Could not test"))?;

    Ok(code == 0)
}

fn get_ignored_features<P: AsRef<Path>>(
    file_path: P,
    item_path: &str,
) -> Result<HashMap<String, Vec<String>>> {
    let result = toml_document_from_path(file_path.as_ref().join("Cargo.toml"));

    match result {
        Ok(document) => {
            let item = get_item_from_doc(item_path, &document);

            let Ok(item) = item else {
                return Ok(HashMap::new());
            };

            let table = item.as_table_like().context(format!(
                "could not parse {} in {:?}",
                item_path,
                file_path.as_ref()
            ))?;

            let mut map = HashMap::new();

            for (key, value) in table.iter() {
                map.insert(
                    key.to_string(),
                    value
                        .as_array()
                        .ok_or(eyre!("Invalid format to keep features"))?
                        .iter()
                        .filter_map(|value| value.as_str())
                        .map(|value| value.to_string())
                        .collect(),
                );
            }

            Ok(map)
        }
        Err(_) => Ok(HashMap::new()),
    }
}
