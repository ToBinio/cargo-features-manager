use crate::dependencies::dependency::{Dependency, DependencyType};
use crate::parsing::package::Package;
use crate::parsing::{get_package_from_version, set_features, toml_document_from_path};
use cargo_metadata::PackageId;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use console::Emoji;
use semver::VersionReq;
use std::collections::HashMap;
use toml_edit::Item;

pub fn parse_workspace(
    path: &str,
    packages: &HashMap<PackageId, cargo_metadata::Package>,
) -> Result<Option<Package>> {
    let path = format!("{}/Cargo.toml", path);

    let document = toml_document_from_path(&path)?;
    let Some(workspace) = document.get("workspace") else {
        return Ok(None);
    };

    let Some(dependencies) = workspace.get("dependencies") else {
        return Ok(None);
    };

    let dependencies_table = dependencies.as_table_like().ok_or(eyre!(
        "failed to parse workspace.dependencies - not a table"
    ))?;

    let dependencies: Result<Vec<Dependency>> = dependencies_table
        .iter()
        .map(|(name, data)| parse_dependency_from_item(packages, name, data))
        .collect();

    let package = Package {
        dependencies: dependencies?,
        name: format!("{} Workspace", Emoji("🗃️", "")).to_string(),
        manifest_path: path,
    };

    Ok(Some(package))
}

fn parse_dependency_from_item(
    packages: &HashMap<PackageId, cargo_metadata::Package>,
    name: &str,
    data: &Item,
) -> Result<Dependency> {
    let mut version = "*";
    let mut enabled_features = vec![];
    let mut uses_default_features = true;
    let mut rename = None;

    if let Some(data) = data.as_table_like() {
        //parse version
        if let Some(version_data) = data.get("version") {
            version = version_data
                .as_str()
                .ok_or(eyre!("could not parse version"))?;
        }

        //parse enabled features
        if let Some(features) = data.get("features") {
            let features = features
                .as_array()
                .ok_or(eyre!("could not parse features"))?;

            enabled_features = features
                .iter()
                .filter_map(|feature| feature.as_str())
                .map(|feature| feature.to_string())
                .collect();
        }

        //parse uses_default_features
        if let Some(uses_default) = data.get("default-features") {
            let uses_default = uses_default
                .as_bool()
                .ok_or(eyre!("could not parse default-features"))?;

            uses_default_features = uses_default;
        }

        //parse rename - package
        if let Some(package) = data.get("package") {
            let package = package.as_str().ok_or(eyre!("could not parse package"))?;

            rename = Some(package.to_string());
        }
    } else {
        version = data.as_str().ok_or(eyre!("could not parse version"))?;
    }

    let mut dependency = Dependency {
        name: name.to_string(),
        rename,
        version: version.to_string(),
        workspace: false,
        kind: DependencyType::Workspace,
        target: None,
        features: Default::default(),
    };

    let package = get_package_from_version(name, &VersionReq::parse(version)?, packages)?;

    set_features(
        &mut dependency,
        package,
        uses_default_features,
        &enabled_features,
    )?;

    Ok(dependency)
}
