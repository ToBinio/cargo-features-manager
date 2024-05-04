use crate::project::dependency::Dependency;
use crate::rendering::scroll_selector::SelectorItem;
use color_eyre::eyre::{bail, eyre};
use fuzzy_matcher::skim::SkimMatcherV2;
use itertools::Itertools;

pub struct Package {
    pub dependencies: Vec<Dependency>,
    pub name: String,
    // path include the Cargo.toml
    pub manifest_path: String,
}

impl Package {
    pub fn get_deps(&self) -> &Vec<Dependency> {
        &self.dependencies
    }

    //todo in rendering function
    pub fn get_deps_filtered_view(&self, filter: &str) -> color_eyre::Result<Vec<SelectorItem>> {
        let deps = if filter.is_empty() {
            self.dependencies
                .iter()
                .sorted_by(|dependency_a, dependency_b| dependency_a.name.cmp(&dependency_b.name))
                .map(|dependency| SelectorItem::from_dependency(dependency, vec![]))
                .collect()
        } else {
            let matcher = SkimMatcherV2::default();

            self.dependencies
                .iter()
                .filter_map(|dependency| {
                    matcher
                        .fuzzy(&dependency.get_name(), filter, true)
                        .map(|fuzzy_result| (dependency, fuzzy_result))
                })
                .sorted_by(|(_, fuzzy_a), (_, fuzzy_b)| fuzzy_a.0.cmp(&fuzzy_b.0).reverse())
                .map(|(dependency, fuzzy)| (dependency, fuzzy.1))
                .map(|(dependency, indexes)| SelectorItem::from_dependency(dependency, indexes))
                .collect()
        };

        Ok(deps)
    }

    pub fn get_dep(&self, name: &str) -> color_eyre::Result<&Dependency> {
        let dep = self.dependencies.iter().find(|dep| dep.get_name().eq(name));

        match dep {
            None => bail!("could not find dependency with name {}", name),
            Some(some) => Ok(some),
        }
    }

    pub fn get_dep_index(&self, name: &String) -> color_eyre::Result<usize> {
        Ok(self
            .dependencies
            .iter()
            .enumerate()
            .find(|(_, dep)| dep.get_name() == *name)
            .ok_or(eyre!("dependency \"{}\" could not be found", name))?
            .0)
    }

    pub fn get_dep_mut(&mut self, name: &str) -> color_eyre::Result<&mut Dependency> {
        let dep = self
            .dependencies
            .iter_mut()
            .find(|dep| dep.get_name().eq(name));

        match dep {
            None => bail!("could not find dependency with name {}", name),
            Some(some) => Ok(some),
        }
    }
}
