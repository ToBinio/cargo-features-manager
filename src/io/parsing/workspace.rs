use crate::io::parsing::dependency::parse_dependency_from_item;
use crate::io::util::toml_document_from_path;
use crate::project::dependency::Dependency;
use crate::project::package::Package;
use cargo_metadata::PackageId;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use console::Emoji;
use std::collections::HashMap;

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
