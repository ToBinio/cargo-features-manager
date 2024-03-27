use crate::dependencies::dependency::{
    Dependency, DependencySource, FeatureData, FeatureType, SubFeature,
};

use cargo_metadata::{CargoOpt, PackageId};

use crate::parsing::workspace::parse_workspace;
use crate::parsing::{get_package_from_version, set_features};
use anyhow::{bail, Context};
use clap::builder::Str;
use itertools::Itertools;
use semver::VersionReq;
use std::collections::HashMap;

pub struct Package {
    pub dependencies: Vec<Dependency>,
    pub name: String,
    // path include the Cargo.toml
    pub manifest_path: String,
}

pub fn get_packages() -> anyhow::Result<Vec<Package>> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .features(CargoOpt::AllFeatures)
        .exec()?;

    let metadata_packages: HashMap<PackageId, cargo_metadata::Package> = metadata
        .packages
        .into_iter()
        .map(|package| (package.id.clone(), package))
        .collect();

    let mut packages = metadata
        .workspace_members
        .iter()
        .map(|package| parse_package(package, &metadata_packages))
        .collect::<anyhow::Result<Vec<Package>>>()?;

    if let Some(workspace_package) =
        parse_workspace(metadata.workspace_root.as_str(), &metadata_packages)?
    {
        packages.push(workspace_package);
    }

    Ok(packages)
}

pub fn parse_package(
    package: &PackageId,
    packages: &HashMap<PackageId, cargo_metadata::Package>,
) -> anyhow::Result<Package> {
    let package = packages.get(package).context("package not found")?;

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

    let mut new_dependency = Dependency {
        name: dependency.name.to_string(),
        version: dependency
            .req
            .to_string()
            .trim_start_matches('^')
            .to_owned(),
        kind: dependency.kind.into(),
        features: HashMap::new(),
    };

    set_features(
        &mut new_dependency,
        package,
        dependency.uses_default_features,
        &dependency.features,
    );

    Ok(new_dependency)
}
