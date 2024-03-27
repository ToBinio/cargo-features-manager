use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Context};

use fuzzy_matcher::skim::SkimMatcherV2;
use itertools::Itertools;
use toml_edit::{Array, Formatted, InlineTable, Item, Value};

use crate::dependencies::dependency::{get_path_from_dependency_kind, Dependency};
use crate::parsing::package::{get_packages, Package};
use crate::parsing::toml_document_from_path;

use crate::rendering::scroll_selector::DependencySelectorItem;

pub struct Document {
    packages: Vec<Package>,
}

impl Document {
    pub fn new() -> anyhow::Result<Document> {
        let packages = get_packages()?;

        if packages.len() == 1
            && packages
                .first()
                .expect("no package found")
                .dependencies
                .is_empty()
        {
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

    pub fn get_deps(&self, package_id: usize) -> anyhow::Result<&Vec<Dependency>> {
        Ok(&self
            .packages
            .get(package_id)
            .context("package not found")?
            .dependencies)
    }

    pub fn get_deps_mut(&mut self, package_id: usize) -> anyhow::Result<&mut Vec<Dependency>> {
        Ok(&mut self
            .packages
            .get_mut(package_id)
            .context("package not found")?
            .dependencies)
    }

    pub fn get_deps_filtered_view(
        &self,
        package_id: usize,
        filter: &str,
    ) -> anyhow::Result<Vec<DependencySelectorItem>> {
        let matcher = SkimMatcherV2::default();

        let deps = self
            .get_deps(package_id)?
            .iter()
            .filter_map(|dependency| {
                matcher
                    .fuzzy(&dependency.get_name(), filter, true)
                    .map(|fuzzy_result| (dependency, fuzzy_result))
            })
            .sorted_by(|(_, fuzzy_a), (_, fuzzy_b)| fuzzy_a.0.cmp(&fuzzy_b.0).reverse())
            .map(|(dependency, fuzzy)| (dependency, fuzzy.1))
            .map(|(dependency, indexes)| DependencySelectorItem::new(dependency, indexes))
            .collect();

        Ok(deps)
    }

    pub fn get_dep(&self, package_id: usize, name: &str) -> anyhow::Result<&Dependency> {
        let dep = self
            .get_deps(package_id)?
            .iter()
            .find(|dep| dep.name.eq(name));

        match dep {
            None => bail!("could not find dependency with name {}", name),
            Some(some) => Ok(some),
        }
    }

    pub fn get_dep_index(&self, package_id: usize, name: &String) -> anyhow::Result<usize> {
        Ok(self
            .get_deps(package_id)?
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
            .get_deps_mut(package_id)?
            .iter_mut()
            .find(|dep| dep.name.eq(name));

        match dep {
            None => bail!("could not find dependency with name {}", name),
            Some(some) => Ok(some),
        }
    }

    pub fn write_dep(&mut self, package_id: usize, name: &str) -> anyhow::Result<()> {
        let (index, _) = self
            .get_deps(package_id)?
            .iter()
            .enumerate()
            .find(|(_index, dep)| dep.get_name().eq(name))
            .ok_or(anyhow!("could not find dependency with name {}", name))?;

        self.write_dep_raw(package_id, index)
    }

    fn write_dep_raw(&mut self, package_id: usize, dep_index: usize) -> anyhow::Result<()> {
        let package = self
            .packages
            .get_mut(package_id)
            .context("package not found")?;

        let dependency = package
            .dependencies
            .get(dep_index)
            .context("dependency not found")?;
        let mut features_to_enable = dependency.get_features_to_enable();

        let mut doc = toml_document_from_path(&package.manifest_path)?;
        let deps = dependency.kind.get_mut_item_from_doc(&mut doc)?;

        let deps = deps.as_table_mut().context(format!(
            "could not parse dependencies as a table - {}",
            package.name
        ))?;

        let table = match deps
            .get_mut(&dependency.get_name())
            .context("dependency not found")?
            .as_table_like_mut()
        {
            None => {
                deps.insert(
                    &dependency.get_name(),
                    Item::Value(Value::InlineTable(InlineTable::new())),
                );

                deps.get_mut(&dependency.get_name())
                    .context(format!("could not find {} in dependency", dependency.name))?
                    .as_table_like_mut()
                    .context(format!("could not parse {} as a table", dependency.name))?
            }
            Some(table) => table,
        };

        let has_custom_attributes = table
            .get_values()
            .iter()
            .map(|(name, _)| name.first().map(|key| key.to_string()).unwrap_or_default())
            .any(|name| !["features", "default-features", "version"].contains(&&*name));

        //check if entry has to be table or can just be string with version
        if dependency.can_use_default() && features_to_enable.is_empty() && !has_custom_attributes {
            deps.insert(
                &dependency.get_name(),
                Item::Value(Value::String(Formatted::new(dependency.get_version()))),
            );
        } else {
            //version
            if !dependency.version.is_empty() && !table.contains_key("git") {
                table.insert(
                    "version",
                    Item::Value(Value::String(Formatted::new(dependency.get_version()))),
                );
            }

            //features
            let mut features = Array::new();

            features_to_enable.sort();

            for name in features_to_enable {
                features.push(Value::String(Formatted::new(name)));
            }

            if features.is_empty() {
                table.remove("features");
            } else {
                table.insert("features", Item::Value(Value::Array(features)));
            }

            //default-feature
            if dependency.can_use_default() {
                table.remove("default-features");
            } else {
                table.insert(
                    "default-features",
                    Item::Value(Value::Boolean(Formatted::new(false))),
                );
            }
        }

        let package = self.packages.get(package_id).context("package not found")?;

        fs::write(&package.manifest_path, doc.to_string()).map_err(anyhow::Error::from)
    }
    pub fn is_workspace(&self) -> bool {
        self.packages.len() > 1
    }
}
