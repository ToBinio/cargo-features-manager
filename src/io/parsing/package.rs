use cargo_metadata::{CargoOpt, PackageId};

use crate::io::parsing::workspace::parse_workspace;
use color_eyre::Result;

use crate::io::parsing::dependency::parse_dependency;
use crate::io::util::toml_document_from_path;
use crate::project::dependency::Dependency;
use crate::project::package::Package;
use color_eyre::eyre::ContextCompat;
use semver::VersionReq;
use std::collections::HashMap;

pub fn get_packages() -> Result<(Vec<Package>, Option<Package>)> {
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
        .collect::<Result<Vec<Package>>>()?;

    Ok((
        packages,
        parse_workspace(metadata.workspace_root.as_str(), &metadata_packages)?,
    ))
}

pub fn parse_package(
    package: &PackageId,
    packages: &HashMap<PackageId, cargo_metadata::Package>,
) -> Result<Package> {
    let package = packages.get(package).context("package not found")?;

    let toml_doc = toml_document_from_path(package.manifest_path.as_str())?;

    let dependencies: Result<Vec<Dependency>> = package
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

pub fn get_package_from_version<'a>(
    name: &str,
    version_req: &VersionReq,
    packages: &'a HashMap<PackageId, cargo_metadata::Package>,
) -> Result<&'a cargo_metadata::Package> {
    packages
        .iter()
        .map(|(_, package)| package)
        .filter(|package| package.name == name)
        .find(|package| version_req.matches(&package.version) || version_req.to_string() == "*")
        .context(format!(
            "could not find version for {} {}",
            name, version_req
        ))
}
