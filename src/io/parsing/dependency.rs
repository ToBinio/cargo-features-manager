use crate::io::parsing::package::get_package_from_version;
use crate::io::util::get_item_from_doc;
use crate::project::dependency::feature::{EnabledState, FeatureData, SubFeature, SubFeatureType};
use crate::project::dependency::util::get_path;
use crate::project::dependency::{Dependency, DependencyType};
use cargo_metadata::PackageId;
use color_eyre::eyre::{eyre, ContextCompat};
use itertools::Itertools;
use semver::VersionReq;
use std::collections::HashMap;
use toml_edit::Item;

pub fn parse_dependency(
    dependency: &cargo_metadata::Dependency,
    packages: &HashMap<PackageId, cargo_metadata::Package>,
    document: &toml_edit::DocumentMut,
) -> color_eyre::Result<Dependency> {
    let package = get_package_from_version(&dependency.name, &dependency.req, packages)?;

    let kind: DependencyType = dependency.kind.into();
    let mut workspace = false;

    let deps = get_item_from_doc(&get_path(&kind, &dependency.target), document)?;

    let deps = deps.as_table().context(format!(
        "could not parse dependencies as a table - {}",
        package.name
    ))?;

    let dep = if let Some(name) = &dependency.rename {
        deps.get(name).ok_or(eyre!(
            "could not find - dep:{} - {} - {:?}",
            name,
            deps,
            kind
        ))?
    } else {
        deps.get(&dependency.name).ok_or(eyre!(
            "could not find - dep:{} - {} - {:?}",
            dependency.name,
            deps,
            kind
        ))?
    };

    if let Some(dep) = dep.as_table_like() {
        if let Some(workspace_item) = dep.get("workspace") {
            workspace = workspace_item.as_bool().unwrap_or(false);
        }
    }

    let mut new_dependency = Dependency {
        name: dependency.name.to_string(),
        rename: dependency.rename.clone(),
        target: dependency.target.clone(),
        version: dependency
            .req
            .to_string()
            .trim_start_matches('^')
            .to_owned(),
        kind,
        workspace,
        features: HashMap::new(),
        comment: None,
    };

    set_features(
        &mut new_dependency,
        package,
        dependency.uses_default_features,
        &dependency.features,
    )?;

    Ok(new_dependency)
}

pub fn parse_dependency_from_item(
    packages: &HashMap<PackageId, cargo_metadata::Package>,
    name: &str,
    data: &Item,
) -> color_eyre::Result<Dependency> {
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
        comment: None,
        version: version.to_string(),
        workspace: false,
        kind: DependencyType::Workspace,
        target: None,
        features: Default::default(),
    };

    if let Ok(package) = get_package_from_version(name, &VersionReq::parse(version)?, packages) {
        set_features(
            &mut dependency,
            package,
            uses_default_features,
            &enabled_features,
        )?;
    } else {
        dependency.comment = Some("unused".to_string());
    }

    Ok(dependency)
}

pub fn set_features(
    dependency: &mut Dependency,
    package: &cargo_metadata::Package,
    uses_default_features: bool,
    enabled_features: &Vec<String>,
) -> color_eyre::Result<()> {
    let default_features = package.features.get("default").cloned().unwrap_or(vec![]);

    let features = package
        .features
        .iter()
        .map(|(feature, sub_features)| {
            (
                feature.to_string(),
                FeatureData {
                    sub_features: sub_features
                        .iter()
                        .map(|name| SubFeature {
                            name: name.to_string(),
                            kind: name.as_str().into(),
                        })
                        .filter(|sub_feature| sub_feature.kind != SubFeatureType::DependencyFeature)
                        .collect_vec(),
                    is_default: default_features.contains(feature),
                    enabled_state: EnabledState::Normal(false),
                },
            )
        })
        .collect();

    dependency.features = features;

    for feature in enabled_features {
        if Into::<SubFeatureType>::into(feature.as_str()) == SubFeatureType::Normal {
            dependency.enable_feature(feature)?;
        }
    }

    if uses_default_features {
        for feature in &default_features {
            if Into::<SubFeatureType>::into(feature.as_str()) == SubFeatureType::Normal {
                dependency.enable_feature(feature)?;
            }
        }
    }

    Ok(())
}
