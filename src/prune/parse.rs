use color_eyre::Result;

use crate::io::util::{get_item_from_doc, toml_document_from_path};
use crate::project::dependency::Dependency;
use crate::project::document::Document;
use crate::prune::FeaturesMap;
use color_eyre::eyre::{ContextCompat, eyre};
use std::collections::HashMap;
use std::ops::Not;
use std::path::Path;

pub fn get_features_to_test(
    document: &Document,
    only_dependency_features: bool,
) -> Result<FeaturesMap> {
    let base_ignored_features =
        get_ignored_features("./", "workspace.cargo-features-manager.keep")?;

    let mut enabled_features = get_enabled_features(document);

    if only_dependency_features {
        remove_non_dependency_features(document, &mut enabled_features)?;
    }

    remove_ignored_features(document, &base_ignored_features, &mut enabled_features)?;

    Ok(enabled_features)
}

fn get_enabled_features(document: &Document) -> FeaturesMap {
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

fn remove_non_dependency_features(
    document: &Document,
    enabled_features: &mut FeaturesMap,
) -> Result<()> {
    for (package_name, dependencies) in enabled_features {
        let package = document.get_package(package_name)?;

        for (dependency_name, features) in dependencies {
            let dependency = package.get_dep(dependency_name)?;

            for feature_name in &features.clone() {
                let Some(feature) = dependency.get_feature(feature_name) else {
                    continue;
                };

                if feature.has_dependency_features().not() {
                    remove_feature(feature_name, features, dependency);
                }
            }
        }
    }

    Ok(())
}

fn remove_ignored_features(
    document: &Document,
    base_ignored: &HashMap<String, Vec<String>>,
    enabled_features: &mut FeaturesMap,
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
