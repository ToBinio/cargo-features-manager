use crate::dependencies::dependency::{Dependency, DependencyType, FeatureData, FeatureType};

use cargo_metadata::{CargoOpt, PackageId};

use itertools::Itertools;
use semver::VersionReq;
use std::collections::HashMap;
use std::str::FromStr;


pub struct Package {
    pub dependencies: Vec<Dependency>,
    pub name: String,
    pub manifest_path: String,
    pub dependency_type: PackageType,
}

pub enum PackageType {
    Normal,
    Workspace,
}

impl PackageType {
    pub fn key(&self) -> &'static str {
        match self {
            PackageType::Normal => "dependencies",
            PackageType::Workspace => "workspace.dependencies",
        }
    }
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
        Ok(vec![parse_package(&root, &packages, PackageType::Normal)?])
    } else {
        metadata
            .workspace_members
            .iter()
            .map(|package| parse_package(package, &packages, PackageType::Workspace))
            .collect()
    }
}

pub fn parse_package(
    package: &PackageId,
    packages: &HashMap<PackageId, cargo_metadata::Package>,
    package_type: PackageType,
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
        dependency_type: package_type,
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
                        .map(|name| (name.to_string(), FeatureType::from_str(name).unwrap()))
                        .collect_vec(),
                    is_default: default_features.contains(feature),
                    is_enabled: false,
                },
            )
        })
        .collect();

    let dependency_type = dependency
        .source
        .as_ref()
        .map(|source| DependencyType::Local(source.to_string()))
        .unwrap_or(DependencyType::Remote);

    let mut new_dependency = Dependency {
        dep_name: dependency.name.to_string(),
        version: dependency.req.to_string(),
        dep_type: dependency_type,
        features,
    };

    //todo join 2 loops
    for feature in &dependency.features {
        if FeatureType::from_str(feature) == Ok(FeatureType::Normal) {
            new_dependency.enable_feature(feature);
        }
    }

    if dependency.uses_default_features {
        for feature in &default_features {
            if FeatureType::from_str(feature) == Ok(FeatureType::Normal) {
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
        .unwrap_or_else(|| panic!("could not find version for {} {}",
            name, version_req)))
}
