use color_eyre::Result;
use std::collections::HashMap;
use std::fs;

use console::{style, Term};
use std::io::Write;
use std::ops::Not;
use std::path::Path;

use crate::project::dependency::Dependency;
use crate::project::document::Document;
use crate::save::save_dependency;
use crate::util::{get_item_from_doc, toml_document_from_path};
use color_eyre::eyre::{eyre, ContextCompat};
use std::process::{Command, Stdio};

pub fn prune(mut document: Document, is_dry_run: bool) -> Result<()> {
    let mut term = Term::stdout();

    let mut enabled_features = get_enabled_features(&document);

    let base_ignored_features =
        get_ignored_features("./", "workspace.cargo-features-manager.keep")?;
    remove_ignored_features(&document, &base_ignored_features, &mut enabled_features)?;

    prune_features(&mut document, is_dry_run, &mut term, enabled_features)?;

    Ok(())
}

fn get_enabled_features(document: &Document) -> HashMap<String, HashMap<String, Vec<String>>> {
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
    enabled_features: &mut HashMap<String, HashMap<String, Vec<String>>>,
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

            for feature in ignored_features.get(dependency_name).unwrap_or(&vec![]) {
                remove_feature(feature, features, dependency);
            }
            for feature in base_ignored.get(dependency_name).unwrap_or(&vec![]) {
                remove_feature(feature, features, dependency);
            }

            if let Some(index) = features.iter().position(|name| name == "default") {
                features.remove(index);
            }
        }
    }

    //todo remove empty packages / dependencies

    Ok(())
}

fn remove_feature(feature: &String, features: &mut Vec<String>, dependency: &Dependency) {
    let index = features.iter().position(|name| name == feature);

    let Some(index) = index else {
        return;
    };

    features.remove(index);

    if let Some(feature) = dependency.get_feature(&feature) {
        for sub_feature in &feature.sub_features {
            remove_feature(&sub_feature.name, features, dependency);
        }
    }
}

fn prune_features(
    document: &mut Document,
    is_dry_run: bool,
    term: &mut Term,
    features: HashMap<String, HashMap<String, Vec<String>>>,
) -> Result<()> {
    let inset = if features.len() == 1 { 0 } else { 2 };

    for (package_name, dependencies) in features {
        if document.is_workspace() {
            writeln!(term, "{}", package_name)?;
        }

        for (dependency_name, features) in dependencies {
            term.clear_line()?;
            writeln!(term, "{:inset$}{} [0/0]", "", dependency_name)?;

            let mut to_be_disabled = vec![];

            for (id, feature) in features.iter().enumerate() {
                term.clear_line()?;
                writeln!(term, "{:inset$} └ {}", "", feature)?;

                document
                    .get_package_mut(&package_name)?
                    .get_dep_mut(&dependency_name)?
                    .disable_feature(feature)?;

                save_dependency(document, &package_name, &dependency_name)?;

                if check()? {
                    //todo disable parent features and dont test them
                    to_be_disabled.push(feature.to_string());
                }

                //reset to start
                for feature in &features {
                    document
                        .get_package_mut(&package_name)?
                        .get_dep_mut(&dependency_name)?
                        .enable_feature(feature)?;
                }

                save_dependency(document, &package_name, &dependency_name)?;

                term.move_cursor_up(2)?;
                term.clear_line()?;
                writeln!(
                    term,
                    "{:inset$}{} [{}/{}]",
                    "",
                    dependency_name,
                    id + 1,
                    features.len()
                )?;
            }

            let mut disabled_count = style(to_be_disabled.len());

            if to_be_disabled.is_empty().not() {
                disabled_count = disabled_count.red();
            }

            term.move_cursor_up(1)?;
            term.clear_line()?;
            writeln!(
                term,
                "{:inset$}{} [{}/{}]",
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
                    document
                        .get_package_mut(&package_name)?
                        .get_dep_mut(&dependency_name)?
                        .disable_feature(&feature)?;
                }

                save_dependency(document, &package_name, &dependency_name)?;
            }
        }
    }

    Ok(())
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
            let item = get_item_from_doc(&item_path, &document);

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
