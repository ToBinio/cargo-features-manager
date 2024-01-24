use crate::dependencies::dependency::Dependency;
use crate::dependencies::dependency_builder::DependencyBuilder;
use anyhow::anyhow;
use glob::glob;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use toml_edit::{Document, Table};

pub struct Package {
    pub dependencies: Vec<Dependency>,
    pub name: String,
    pub toml_doc: Document,
    pub dir_path: String,
    pub dependency_type: DependencyType,
}

pub enum DependencyType {
    Normal,
    Workspace,
}

impl DependencyType {
    pub fn key(&self) -> &'static str {
        match self {
            DependencyType::Normal => "dependencies",
            DependencyType::Workspace => "workspace.dependencies",
        }
    }
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

pub fn packages_from_workspace(
    document: &Document,
    base_path: String,
) -> anyhow::Result<Vec<Package>> {
    let mut packages = vec![];

    if let Some(members) = document
        .get("workspace")
        .ok_or(anyhow!("no workspace found"))?
        .as_table()
        .ok_or(anyhow!("no workspace found"))?
        .get("members")
    {
        let members = members.as_array().ok_or(anyhow!("no members found"))?;

        for entry in members {
            let path = entry.as_str().ok_or(anyhow!("invalid member found"))?;

            for path in glob(path)? {
                let path = path?;
                let document = document_from_path(&path)?;

                packages.push(package_from_document(
                    document,
                    path.to_str()
                        .ok_or(anyhow!("invalid path {:?}", path))?
                        .to_string(),
                )?);
            }
        }
    }

    if document.contains_key("package") {
        packages.push(package_from_document(document.clone(), base_path.clone())?)
    }

    if let Some(dependencies) = get_dependencies(document, DependencyType::Workspace.key())? {
        let dependencies = dependencies_from_table(&base_path, dependencies)?;

        packages.push(Package {
            dependencies,
            name: "󰏓 Workspace".to_string(),
            toml_doc: document.clone(),
            dir_path: "".to_string(),
            dependency_type: DependencyType::Workspace,
        });
    }

    Ok(packages)
}

pub fn package_from_document(doc: Document, base_path: String) -> anyhow::Result<Package> {
    let deps_table = get_dependencies(&doc, DependencyType::Normal.key())?
        .ok_or(anyhow!("no dependencies were found"))?;

    let name = doc
        .get("package")
        .ok_or(anyhow!("invalid Package - no name found"))?
        .as_table()
        .ok_or(anyhow!("invalid Package - no name found"))?
        .get("name")
        .ok_or(anyhow!("invalid Package - no name found"))?
        .as_str()
        .ok_or(anyhow!("invalid Package - no name found"))?;

    let deps = dependencies_from_table(&base_path, deps_table)?;

    Ok(Package {
        dependencies: deps,
        name: name.to_string(),
        toml_doc: doc,
        dir_path: base_path,
        dependency_type: DependencyType::Normal,
    })
}

fn get_dependencies<'a>(doc: &'a Document, key: &str) -> anyhow::Result<Option<&'a Table>> {
    let mut item = doc.as_item();

    for key in key.split('.') {
        let new_item = item.get(key);

        if new_item.is_none() {
            return Ok(None);
        }

        item = new_item.unwrap()
    }

    let deps_table = item
        .as_table()
        .ok_or(anyhow!("no dependencies were found"))?;

    Ok(Some(deps_table))
}

fn dependencies_from_table(base_path: &str, deps_table: &Table) -> anyhow::Result<Vec<Dependency>> {
    let deps: Vec<Option<Dependency>> = deps_table
        .iter()
        .map(|(name, value)| DependencyBuilder::build_dependency(name, value, base_path))
        .collect::<anyhow::Result<Vec<_>>>()?;

    let deps = deps.into_iter().flatten().collect();

    Ok(deps)
}
