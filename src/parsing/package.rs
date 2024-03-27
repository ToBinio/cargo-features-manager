use crate::dependencies::dependency::{Dependency, DependencyType};

use cargo_metadata::{CargoOpt, PackageId};

use crate::parsing::workspace::parse_workspace;
use crate::parsing::{get_package_from_version, set_features, toml_document_from_path};
use anyhow::{anyhow, Context};

use std::collections::HashMap;

pub struct Package {
    pub dependencies: Vec<Dependency>,
    pub name: String,
    // path include the Cargo.toml
    pub manifest_path: String,
}

pub fn get_packages() -> anyhow::Result<(Vec<Package>, Option<Package>)> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .features(CargoOpt::AllFeatures)
        .exec()?;

    let metadata_packages: HashMap<PackageId, cargo_metadata::Package> = metadata
        .packages
        .into_iter()
        .map(|package| (package.id.clone(), package))
        .collect();

    let packages = metadata
        .workspace_members
        .iter()
        .map(|package| parse_package(package, &metadata_packages))
        .collect::<anyhow::Result<Vec<Package>>>()?;

    Ok((
        packages,
        parse_workspace(metadata.workspace_root.as_str(), &metadata_packages)?,
    ))
}

pub fn parse_package(
    package: &PackageId,
    packages: &HashMap<PackageId, cargo_metadata::Package>,
) -> anyhow::Result<Package> {
    let package = packages.get(package).context("package not found")?;

    let toml_doc = toml_document_from_path(package.manifest_path.as_str())?;

    let dependencies: anyhow::Result<Vec<Dependency>> = package
        .dependencies
        .iter()
        .map(|dep| parse_dependency(dep, packages, &toml_doc))
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
    document: &toml_edit::Document,
) -> anyhow::Result<Dependency> {
    let package = get_package_from_version(&dependency.name, &dependency.req, packages)?;

    let kind: DependencyType = dependency.kind.into();
    let mut workspace = false;

    //todo remove workaround when targets are handled...
    if dependency.target.is_none() {
        let deps = kind.get_item_from_doc(document)?;

        let deps = deps.as_table().context(format!(
            "could not parse dependencies as a table - {}",
            package.name
        ))?;

        let dep = deps.get(&dependency.name).ok_or(anyhow!(
            "could not find - dep:{} - {} - {:?}",
            dependency.name,
            deps,
            kind
        ))?;

        if let Some(dep) = dep.as_table_like() {
            if let Some(workspace_item) = dep.get("workspace") {
                workspace = workspace_item.as_bool().unwrap_or(false);
            }
        }
    }

    let mut new_dependency = Dependency {
        name: dependency.name.to_string(),
        version: dependency
            .req
            .to_string()
            .trim_start_matches('^')
            .to_owned(),
        kind,
        workspace,
        features: HashMap::new(),
    };

    set_features(
        &mut new_dependency,
        package,
        dependency.uses_default_features,
        &dependency.features,
    )?;

    Ok(new_dependency)
}
