use crate::edit::filter_view::item::FilterViewItem;
use crate::project::dependency::Dependency;
use crate::project::document::Document;
use crate::project::package::Package;
use color_eyre::eyre::ContextCompat;
use fuzzy_matcher::skim::SkimMatcherV2;
use itertools::Itertools;
use std::cmp::Ordering;

pub mod item;

pub struct FilterView {
    pub selected_index: usize,
    pub data: Vec<FilterViewItem>,
}

impl FilterView {
    pub fn shift(&mut self, shift: isize) {
        if !self.has_data() {
            self.selected_index = 0;
            return;
        }

        let mut selected_temp = self.selected_index as isize;

        selected_temp += self.data.len() as isize;
        selected_temp += shift;

        selected_temp %= self.data.len() as isize;

        self.selected_index = selected_temp as usize;
    }

    pub fn get_selected(&self) -> color_eyre::Result<&FilterViewItem> {
        self.data
            .get(self.selected_index)
            .context("nothing selected")
    }

    pub fn has_data(&self) -> bool {
        !self.data.is_empty()
    }

    pub fn data_from_dependency(dependency: &Dependency, filter: &str) -> Vec<FilterViewItem> {
        let features = dependency
            .features
            .iter()
            .filter(|feature| feature.0 != "default");

        if filter.is_empty() {
            features
                .sorted_by(|(name_a, data_a), (name_b, data_b)| {
                    if data_a.is_default && !data_b.is_default {
                        return Ordering::Less;
                    }

                    if data_b.is_default && !data_a.is_default {
                        return Ordering::Greater;
                    }

                    name_a.cmp(name_b)
                })
                .map(|(name, _)| FilterViewItem::from_feature(name, vec![]))
                .collect()
        } else {
            let matcher = SkimMatcherV2::default();

            features
                .filter_map(|(name, _)| matcher.fuzzy(name, filter, true).map(|some| (name, some)))
                .sorted_by(|(_, fuzzy_a), (_, fuzzy_b)| fuzzy_a.0.cmp(&fuzzy_b.0).reverse())
                .map(|(name, fuzzy)| (name, fuzzy.1))
                .map(|(name, indexes)| FilterViewItem::from_feature(name, indexes))
                .collect()
        }
    }

    pub fn data_from_package(
        package: &Package,
        filter: &str,
    ) -> color_eyre::Result<Vec<FilterViewItem>> {
        let deps = if filter.is_empty() {
            package
                .dependencies
                .iter()
                .sorted_by(|dependency_a, dependency_b| dependency_a.name.cmp(&dependency_b.name))
                .map(|dependency| FilterViewItem::from_dependency(dependency, vec![]))
                .collect()
        } else {
            let matcher = SkimMatcherV2::default();

            package
                .dependencies
                .iter()
                .filter_map(|dependency| {
                    matcher
                        .fuzzy(&dependency.get_name(), filter, true)
                        .map(|fuzzy_result| (dependency, fuzzy_result))
                })
                .sorted_by(|(_, fuzzy_a), (_, fuzzy_b)| fuzzy_a.0.cmp(&fuzzy_b.0).reverse())
                .map(|(dependency, fuzzy)| (dependency, fuzzy.1))
                .map(|(dependency, indexes)| FilterViewItem::from_dependency(dependency, indexes))
                .collect()
        };

        Ok(deps)
    }

    pub fn data_from_document(
        document: &Document,
        filter: &str,
    ) -> color_eyre::Result<Vec<FilterViewItem>> {
        let packages = if filter.is_empty() {
            document
                .get_packages()
                .iter()
                .sorted_by(|package_a, package_b| package_a.name.cmp(&package_b.name))
                .map(|package| FilterViewItem::from_package(package, vec![]))
                .collect()
        } else {
            let matcher = SkimMatcherV2::default();

            document
                .get_packages()
                .iter()
                .filter_map(|package| {
                    matcher
                        .fuzzy(&package.name, filter, true)
                        .map(|fuzzy_result| (package, fuzzy_result))
                })
                .sorted_by(|(_, fuzzy_a), (_, fuzzy_b)| fuzzy_a.0.cmp(&fuzzy_b.0).reverse())
                .map(|(package, fuzzy)| (package, fuzzy.1))
                .map(|(package, indexes)| FilterViewItem::from_package(package, indexes))
                .collect()
        };

        Ok(packages)
    }
}
