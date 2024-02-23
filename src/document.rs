use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail};

use fuzzy_matcher::skim::SkimMatcherV2;
use itertools::Itertools;
use toml_edit::{Array, Formatted, InlineTable, Item, Value};

use crate::dependencies::dependency::{Dependency, DependencyType};
use crate::package::{get_packages, Package};

use crate::rendering::scroll_selector::DependencySelectorItem;

pub fn toml_document_from_path<P: AsRef<Path>>(dir_path: P) -> anyhow::Result<toml_edit::Document> {
    let file_content = fs::read_to_string(&dir_path)
        .map_err(|_| anyhow!("could not find Cargo.toml at {:?}", dir_path.as_ref()))?;

    Ok(file_content.parse()?)
}

pub struct Document {
    packages: Vec<Package>,
}

impl Document {
    pub fn new() -> anyhow::Result<Document> {
        let packages = get_packages()?;

        if packages.len() == 1 && packages.first().unwrap().dependencies.is_empty() {
            bail!("no dependencies were found")
        }

        Ok(Document { packages })
    }

    pub fn get_packages_names(&self) -> Vec<String> {
        self.packages
            .iter()
            .map(|package| package.name.to_string())
            .collect()
    }

    pub fn get_package(&self, package_id: usize) -> Option<&Package> {
        self.packages.get(package_id)
    }

    pub fn get_deps(&self, package_id: usize) -> &Vec<Dependency> {
        &self.packages.get(package_id).unwrap().dependencies
    }

    pub fn get_deps_mut(&mut self, package_id: usize) -> &mut Vec<Dependency> {
        &mut self.packages.get_mut(package_id).unwrap().dependencies
    }

    pub fn get_deps_filtered_view(
        &self,
        package_id: usize,
        filter: &str,
    ) -> Vec<DependencySelectorItem> {
        let matcher = SkimMatcherV2::default();

        self.get_deps(package_id)
            .iter()
            .filter_map(|dependency| {
                matcher
                    .fuzzy(&dependency.get_name(), filter, true)
                    .map(|fuzzy_result| (dependency, fuzzy_result))
            })
            .sorted_by(|(_, fuzzy_a), (_, fuzzy_b)| fuzzy_a.0.cmp(&fuzzy_b.0).reverse())
            .map(|(dependency, fuzzy)| (dependency, fuzzy.1))
            .map(|(dependency, indexes)| DependencySelectorItem::new(dependency, indexes))
            .collect()
    }

    pub fn get_dep(&self, package_id: usize, name: &str) -> anyhow::Result<&Dependency> {
        let dep = self
            .get_deps(package_id)
            .iter()
            .find(|dep| dep.dep_name.eq(name));

        match dep {
            None => bail!("could not find dependency with name {}", name),
            Some(some) => Ok(some),
        }
    }

    pub fn get_dep_index(&self, package_id: usize, name: &String) -> anyhow::Result<usize> {
        Ok(self
            .get_deps(package_id)
            .iter()
            .enumerate()
            .find(|(_, dep)| dep.get_name() == *name)
            .ok_or(anyhow!("dependency \"{}\" could not be found", name))?
            .0)
    }

    pub fn get_dep_mut(
        &mut self,
        package_id: usize,
        name: &str,
    ) -> anyhow::Result<&mut Dependency> {
        let dep = self
            .get_deps_mut(package_id)
            .iter_mut()
            .find(|dep| dep.dep_name.eq(name));

        match dep {
            None => bail!("could not find dependency with name {}", name),
            Some(some) => Ok(some),
        }
    }

    pub fn write_dep_by_name(&mut self, package_id: usize, name: &str) -> anyhow::Result<()> {
        let (index, _) = self
            .get_deps(package_id)
            .iter()
            .enumerate()
            .find(|(_index, dep)| dep.get_name().eq(name))
            .ok_or(anyhow!("could not find dependency with name {}", name))?;

        self.write_dep(package_id, index)
    }

    pub fn write_dep(&mut self, package_id: usize, dep_index: usize) -> anyhow::Result<()> {
        let package = self.packages.get_mut(package_id).unwrap();

        let key = package.dependency_type.key();

        let mut doc = toml_document_from_path(&package.manifest_path)?;
        let mut deps = doc.as_item_mut();

        for key in key.split('.') {
            deps = deps
                .get_mut(key)
                .ok_or(anyhow!("could not find dependency - {}", package.name))?;
        }

        let deps = deps.as_table_mut().unwrap();

        let dependency = package.dependencies.get(dep_index).unwrap();

        let mut enabled_features = dependency.get_features_to_enable();

        if !dependency.can_use_default()
            || !enabled_features.is_empty()
            || dependency.dep_type != DependencyType::Remote
        {
            let mut table = InlineTable::new();

            if let DependencyType::Local(path) = &dependency.dep_type {
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

            enabled_features.sort();

            for name in enabled_features {
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

        let package = self.packages.get(package_id).unwrap();

        fs::write(&package.manifest_path, doc.to_string()).unwrap();

        Ok(())
    }
    pub fn is_workspace(&self) -> bool {
        self.packages.len() > 1
    }
}
