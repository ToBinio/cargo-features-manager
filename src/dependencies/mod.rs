use crate::dependencies::dependency::Dependency;
use crate::dependencies::dependency_builder::DependencyBuilder;
use anyhow::bail;
use toml_edit::Document;

pub mod dependency;
pub mod dependency_builder;

pub fn dependencies_from_document(doc: &Document) -> anyhow::Result<Vec<Dependency>> {
    let deps_table = match doc.get_key_value("dependencies") {
        None => bail!("no dependencies were found"),
        Some(some) => some.1.as_table().unwrap(),
    };

    deps_table
        .iter()
        .map(|(name, value)| DependencyBuilder::build_dependency(name, value))
        .collect::<Result<Vec<Dependency>, anyhow::Error>>()
}
