use std::fs;
use std::path::Path;
use std::str::FromStr;

use anyhow::anyhow;
use toml_edit::{Array, Formatted, InlineTable, Item, Value};

use crate::dependency::{Dependency, DependencyOrigin};
use crate::dependency_builder::DependencyBuilder;

pub struct Document {
    toml_doc: toml_edit::Document,

    deps: Vec<Dependency>,

    path: String,
}

impl Document {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Document> {
        let file_content =
            fs::read_to_string(&path).map_err(|_| anyhow!("could not find Cargo.toml"))?;
        let doc = toml_edit::Document::from_str(&file_content)?;

        let deps = match doc.get_key_value("dependencies") {
            None => return Err(anyhow::Error::msg("no dependencies were found")),
            Some(some) => some.1.as_table().unwrap(),
        };

        let deps = deps
            .iter()
            .map(|(name, value)| DependencyBuilder::build_dependency(name, value))
            .collect::<Result<Vec<Dependency>, anyhow::Error>>()?;

        Ok(Document {
            toml_doc: doc,
            deps,
            path: path.as_ref().to_str().unwrap().to_string(),
        })
    }

    pub fn get_deps(&self) -> &Vec<Dependency> {
        &self.deps
    }

    pub fn get_dep(&self, index: usize) -> anyhow::Result<&Dependency> {
        match self.deps.get(index) {
            None => Err(anyhow::Error::msg("out of bounce")),
            Some(some) => Ok(some),
        }
    }

    pub fn get_dep_mut(&mut self, index: usize) -> &mut Dependency {
        self.deps.get_mut(index).unwrap()
    }

    pub fn write_dep(&mut self, dep_index: usize) {
        let (_name, deps) = self.toml_doc.get_key_value_mut("dependencies").unwrap();
        let deps = deps.as_table_mut().unwrap();

        let dependency = self.deps.get(dep_index).unwrap();

        if !dependency.can_use_default()
            || !dependency.get_features_to_enable().is_empty()
            || dependency.origin != DependencyOrigin::Remote
        {
            let mut table = InlineTable::new();

            if let DependencyOrigin::Local(path) = &dependency.origin {
                table.insert("path", Value::String(Formatted::new(path.to_string())));
            }

            //version
            if !dependency.version.is_empty() {
                table.insert(
                    "version",
                    Value::String(Formatted::new(dependency.get_version())),
                );
            }

            //features
            let mut features = Array::new();

            for name in dependency.get_features_to_enable() {
                features.push(Value::String(Formatted::new(name)));
            }

            if !features.is_empty() {
                table.insert("features", Value::Array(features));
            }

            //default-feature
            let uses_default = dependency.can_use_default();
            if !uses_default {
                table.insert(
                    "default-features",
                    Value::Boolean(Formatted::new(uses_default)),
                );
            }

            deps.insert(
                &dependency.get_name(),
                Item::Value(Value::InlineTable(table)),
            );
        } else {
            deps.insert(
                &dependency.get_name(),
                Item::Value(Value::String(Formatted::new(dependency.get_version()))),
            );
        }

        fs::write(self.path.clone(), self.toml_doc.to_string()).unwrap();
    }
}
