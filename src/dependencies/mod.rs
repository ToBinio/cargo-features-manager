use crate::dependencies::dependency::Dependency;
use crate::dependencies::dependency_builder::DependencyBuilder;
use crate::package::Package;
use anyhow::{anyhow, bail};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use toml_edit::Document;

pub mod dependency;
pub mod dependency_builder;

//todo move to better location

pub fn document_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Document> {
    let file_content = fs::read_to_string(&path)
        .map_err(|_| anyhow!("could not find Cargo.toml at {:?}", path.as_ref()))?;
    Ok(toml_edit::Document::from_str(&file_content)?)
}

pub fn is_workspace(document: &Document) -> bool {
    return document.contains_key("workspace");
}

pub fn packages_from_workspace(document: &Document) -> anyhow::Result<Vec<Package>> {
    let members = document
        .get_key_value("workspace")
        .ok_or(anyhow!("no workspace found"))?
        .1
        .as_table()
        .ok_or(anyhow!("no workspace found"))?
        .get_key_value("members")
        .ok_or(anyhow!("no members found"))?
        .1
        .as_array()
        .ok_or(anyhow!("no members found"))?;

    let mut packages = vec![];

    for entry in members {
        let path = entry.as_str().ok_or(anyhow!("invalid member found"))?;

        let path = path.to_owned() + "/Cargo.toml";

        let document = document_from_path(&path)?;

        packages.push(package_from_document(document, path)?);
    }

    Ok(packages)
}

pub fn package_from_document(doc: Document, path: String) -> anyhow::Result<Package> {
    let deps_table = doc
        .get_key_value("dependencies")
        .ok_or(anyhow!("no dependencies were found"))?
        .1
        .as_table()
        .ok_or(anyhow!("no dependencies were found"))?;

    let name = doc
        .get_key_value("package")
        .ok_or(anyhow!("invalid Package - no name found"))?
        .1
        .as_table()
        .ok_or(anyhow!("invalid Package - no name found"))?
        .get_key_value("name")
        .ok_or(anyhow!("invalid Package - no name found"))?
        .1
        .as_str()
        .ok_or(anyhow!("invalid Package - no name found"))?;

    let deps = deps_table
        .iter()
        .map(|(name, value)| DependencyBuilder::build_dependency(name, value))
        .collect::<Result<Vec<Dependency>, anyhow::Error>>()?;

    Ok(Package {
        dependencies: deps,
        name: name.to_string(),
        toml_doc: doc,
        path,
    })
}
