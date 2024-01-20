use crate::dependencies::dependency::Dependency;
use crate::dependencies::dependency_builder::DependencyBuilder;
use anyhow::anyhow;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use toml_edit::Document;

pub struct Package {
    pub dependencies: Vec<Dependency>,
    pub name: String,
    pub toml_doc: toml_edit::Document,
    pub dir_path: String,
}

pub fn document_from_path<P: AsRef<Path>>(dir_path: P) -> anyhow::Result<Document> {
    let path = dir_path.as_ref().join("Cargo.toml");

    let file_content = fs::read_to_string(&path)
        .map_err(|_| anyhow!("could not find Cargo.toml at {:?}", path))?;
    Ok(toml_edit::Document::from_str(&file_content)?)
}

pub fn is_workspace(document: &Document) -> bool {
    document.contains_key("workspace")
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

        let document = document_from_path(path)?;

        packages.push(package_from_document(document, path.to_string())?);
    }

    Ok(packages)
}

pub fn package_from_document(doc: Document, base_path: String) -> anyhow::Result<Package> {
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
        dir_path: base_path,
    })
}
