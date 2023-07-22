use std::path::Path;
use std::str::FromStr;
use std::{fs, thread};

use crate::dependencies::dependencies_from_document;
use anyhow::{anyhow, bail};
use crates_index::SparseIndex;
use fuzzy_matcher::skim::SkimMatcherV2;
use itertools::Itertools;
use toml_edit::{Array, Formatted, InlineTable, Item, Value};

use crate::dependencies::dependency::{Dependency, DependencyOrigin};
use crate::dependencies::dependency_builder::DependencyBuilder;

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

        Ok(Document {
            deps: dependencies_from_document(&doc)?,
            toml_doc: doc,
            path: path.as_ref().to_str().unwrap().to_string(),
        })
    }

    pub fn get_deps_filtered_view(&self, filter: &str) -> Vec<(usize, Vec<usize>)> {
        let mut dep_vec = vec![];

        for (index, dep) in self.deps.iter().enumerate() {
            dep_vec.push((index, dep.get_name()));
        }

        let matcher = SkimMatcherV2::default();

        dep_vec
            .iter()
            .filter_map(|(index, name)| {
                matcher
                    .fuzzy(name, &filter, true)
                    .map(|some| (*index, some))
            })
            .sorted_by(|(_, fuzzy_a), (_, fuzzy_b)| fuzzy_a.0.cmp(&fuzzy_b.0).reverse())
            .map(|(index, fuzzy)| (index, fuzzy.1.iter().map(|i| *i as usize).collect()))
            .collect()
    }

    pub fn get_dep(&self, index: usize) -> anyhow::Result<&Dependency> {
        match self.deps.get(index) {
            None => Err(anyhow::Error::msg("out of bounce")),
            Some(some) => Ok(some),
        }
    }

    pub fn get_dep_index(&self, name: &String) -> anyhow::Result<usize> {
        for (index, current_crate) in self.deps.iter().enumerate() {
            if &current_crate.get_name() == name {
                return Ok(index);
            }
        }

        Err(anyhow::Error::msg(format!(
            "dependency \"{}\" could not be found",
            name
        )))
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
