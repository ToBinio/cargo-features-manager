use crate::dependencies::dependency::Dependency;
use crate::dependencies::dependency_builder::DependencyBuilder;
use anyhow::bail;
use crates_index::SparseIndex;
use std::thread;
use toml_edit::Document;

pub mod dependency;
pub mod dependency_builder;

pub fn dependencies_from_document(doc: &Document) -> anyhow::Result<Vec<Dependency>> {
    let deps_table = match doc.get_key_value("dependencies") {
        None => bail!("no dependencies were found"),
        Some(some) => some.1.as_table().unwrap(),
    };

    thread::scope(|scope| {
        for (name, value) in deps_table {
            scope.spawn(|| {
                let index = SparseIndex::new_cargo_default()?;

                let request: ureq::Request = index.make_cache_request(name)?.into();

                let response: http::Response<String> = request.call()?.into();

                let (parts, body) = response.into_parts();
                let response = http::Response::from_parts(parts, body.into_bytes());

                index.parse_cache_response(name, response, true)?;

                DependencyBuilder::build_dependency(name, value)
            });
        }
    });

    deps_table
        .iter()
        .map(|(name, value)| DependencyBuilder::build_dependency(name, value))
        .collect::<Result<Vec<Dependency>, anyhow::Error>>()
}
