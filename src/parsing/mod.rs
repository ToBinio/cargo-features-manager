use crate::dependencies::dependency::{
    Dependency, EnabledState, FeatureData, FeatureType, SubFeature,
};
use anyhow::anyhow;
use cargo_metadata::PackageId;
use itertools::Itertools;
use semver::VersionReq;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub mod package;
pub mod workspace;

pub fn toml_document_from_path<P: AsRef<Path>>(dir_path: P) -> anyhow::Result<toml_edit::Document> {
    let file_content = fs::read_to_string(&dir_path)
        .map_err(|_| anyhow!("could not find Cargo.toml at {:?}", dir_path.as_ref()))?;

    Ok(file_content.parse()?)
}

pub fn set_features(
    dependency: &mut Dependency,
    package: &cargo_metadata::Package,
    uses_default_features: bool,
    enabled_features: &Vec<String>,
) -> anyhow::Result<()> {
    let default_features = package.features.get("default").cloned().unwrap_or(vec![]);

    let features = package
        .features
        .iter()
        .filter(|(name, _)| name != &"default")
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
                        .filter(|sub_feature| sub_feature.kind != FeatureType::DependencyFeature)
                        .collect_vec(),
                    is_default: default_features.contains(feature),
                    enabled_state: EnabledState::Normal(false),
                },
            )
        })
        .collect();

    dependency.features = features;

    for feature in enabled_features {
        if Into::<FeatureType>::into(feature.as_str()) == FeatureType::Normal {
            dependency.enable_feature(feature)?;
        }
    }

    if uses_default_features {
        for feature in &default_features {
            if Into::<FeatureType>::into(feature.as_str()) == FeatureType::Normal {
                dependency.enable_feature(feature)?;
            }
        }
    }

    Ok(())
}

pub fn get_package_from_version<'a>(
    name: &str,
    version_req: &VersionReq,
    packages: &'a HashMap<PackageId, cargo_metadata::Package>,
) -> anyhow::Result<&'a cargo_metadata::Package> {
    Ok(packages
        .iter()
        .map(|(_, package)| package)
        .find(|package| package.name == name && version_req.matches(&package.version))
        .unwrap_or_else(|| panic!("could not find version for {} {}", name, version_req)))
}
