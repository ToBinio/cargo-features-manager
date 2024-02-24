use crate::dependencies::dependency::{
    Dependency, DependencySource, FeatureData, FeatureType, SubFeature,
};

use cargo_metadata::{CargoOpt, PackageId};

use itertools::Itertools;
use semver::VersionReq;
use std::collections::HashMap;

pub struct Package {
    pub dependencies: Vec<Dependency>,
    pub name: String,
    pub manifest_path: String,
}

pub fn get_packages() -> anyhow::Result<Vec<Package>> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .features(CargoOpt::AllFeatures)
        .exec()?;

    let packages: HashMap<PackageId, cargo_metadata::Package> = metadata
        .packages
        .into_iter()
        .map(|package| (package.id.clone(), package))
        .collect();

    let resolve = metadata.resolve.expect("no resolver found");

    if let Some(root) = resolve.root {
        Ok(vec![parse_package(&root, &packages)?])
    } else {
        metadata
            .workspace_members
            .iter()
            .map(|package| parse_package(package, &packages))
            .collect()
    }
}

pub fn parse_package(
    package: &PackageId,
    packages: &HashMap<PackageId, cargo_metadata::Package>,
) -> anyhow::Result<Package> {
    let package = packages.get(package).unwrap();

    let dependencies: anyhow::Result<Vec<Dependency>> = package
        .dependencies
        .iter()
        .map(|dep| parse_dependency(dep, packages))
        .collect();

    Ok(Package {
        dependencies: dependencies?,
        name: package.name.to_string(),
        manifest_path: package.manifest_path.to_string(),
    })
}

pub fn parse_dependency(
    dependency: &cargo_metadata::Dependency,
    packages: &HashMap<PackageId, cargo_metadata::Package>,
) -> anyhow::Result<Dependency> {
    let package = get_package_from_version(&dependency.name, &dependency.req, packages)?;

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
                    is_enabled: false,
                },
            )
        })
        .collect();

    let dependency_source = dependency
        .path
        .as_ref()
        .map(|source| DependencySource::Local(source.to_string()))
        .unwrap_or(DependencySource::Remote);

    let mut new_dependency = Dependency {
        name: dependency.name.to_string(),
        version: dependency
            .req
            .to_string()
            .trim_start_matches('^')
            .to_owned(),
        source: dependency_source,
        kind: dependency.kind,
        features,
    };

    for feature in &dependency.features {
        if Into::<FeatureType>::into(feature.as_str()) == FeatureType::Normal {
            new_dependency.enable_feature(feature);
        }
    }

    if dependency.uses_default_features {
        for feature in &default_features {
            if Into::<FeatureType>::into(feature.as_str()) == FeatureType::Normal {
                new_dependency.enable_feature(feature);
            }
        }
    }

    Ok(new_dependency)
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
