use crate::rendering::scroll_selector::FeatureSelectorItem;
use cargo_metadata::DependencyKind;
use console::Emoji;
use fuzzy_matcher::skim::SkimMatcherV2;
use itertools::Itertools;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

pub struct Dependency {
    pub name: String,
    pub version: String,

    pub source: DependencySource,
    pub kind: DependencyKind,

    pub features: HashMap<String, FeatureData>,
}

impl Dependency {
    pub fn get_name(&self) -> String {
        self.name.to_string()
    }

    pub fn get_version(&self) -> String {
        self.version.to_string()
    }

    pub fn get_features_filtered_view(&self, filter: &str) -> Vec<FeatureSelectorItem> {
        let features: Vec<(&String, &FeatureData)> = self.features.iter().collect();

        if filter.is_empty() {
            features
                .iter()
                .sorted_by(|(name_a, data_a), (name_b, data_b)| {
                    if data_a.is_default && !data_b.is_default {
                        return Ordering::Less;
                    }

                    if data_b.is_default && !data_a.is_default {
                        return Ordering::Greater;
                    }

                    name_a.partial_cmp(name_b).unwrap()
                })
                .map(|(name, _)| FeatureSelectorItem::new(name, vec![]))
                .collect()
        } else {
            let matcher = SkimMatcherV2::default();

            features
                .iter()
                .filter_map(|(name, _)| matcher.fuzzy(name, filter, true).map(|some| (name, some)))
                .sorted_by(|(_, fuzzy_a), (_, fuzzy_b)| fuzzy_a.0.cmp(&fuzzy_b.0).reverse())
                .map(|(name, fuzzy)| (name, fuzzy.1))
                .map(|(name, indexes)| FeatureSelectorItem::new(name, indexes))
                .collect()
        }
    }

    pub fn get_feature(&self, feature_name: &str) -> &FeatureData {
        self.features.get(feature_name).unwrap()
    }

    pub fn has_features(&self) -> bool {
        !self.features.is_empty()
    }

    pub fn can_use_default(&self) -> bool {
        for data in self.features.values() {
            if data.is_default && !data.is_enabled {
                return false;
            }
        }

        true
    }

    pub fn get_features_to_enable(&self) -> Vec<String> {
        let can_use_default = self.can_use_default();

        self.features
            .iter()
            .filter(|(_, data)| data.is_enabled)
            .filter(|(_, data)| !can_use_default || !data.is_default)
            .map(|(name, _)| name.clone())
            .filter(|name| self.get_currently_dependent_features(name).is_empty())
            .collect()
    }

    pub fn toggle_feature(&mut self, feature_name: &str) {
        let data = self.features.get(feature_name).unwrap();

        if data.is_enabled {
            self.disable_feature(feature_name);
        } else {
            self.enable_feature(feature_name);
        }
    }

    pub fn enable_feature(&mut self, feature_name: &str) {
        let data = self
            .features
            .get_mut(feature_name)
            .unwrap_or_else(|| panic!("couldnt find package {}", feature_name));

        if data.is_enabled {
            //early return to prevent loop
            return;
        }

        data.is_enabled = true;

        //enable sub features
        let sub_features = data
            .sub_features
            .iter()
            .filter(|sub_feature| sub_feature.kind == FeatureType::Normal)
            .map(|sub_feature| sub_feature.name.to_string())
            .collect_vec();

        for sub_feature_name in sub_features {
            self.enable_feature(&sub_feature_name);
        }
    }

    pub fn disable_feature(&mut self, feature_name: &str) {
        let data = self.features.get_mut(feature_name).unwrap();

        if !data.is_enabled {
            //early return to prevent loop
            return;
        }

        data.is_enabled = false;

        for name in self.get_dependent_features(feature_name) {
            self.disable_feature(&name)
        }
    }

    /// returns all features which require the feature to be enabled
    fn get_dependent_features(&self, feature_name: &str) -> Vec<String> {
        let mut dep_features = vec![];

        for (name, data) in &self.features {
            if data
                .sub_features
                .iter()
                .any(|sub_feature| sub_feature.name == *feature_name)
            {
                dep_features.push(name.to_string())
            }
        }

        dep_features
    }

    /// returns all features which are currently enabled and require the feature to be enabled
    pub fn get_currently_dependent_features(&self, feature_name: &str) -> Vec<String> {
        self.get_dependent_features(feature_name)
            .iter()
            .filter(|name| self.features.get(*name).unwrap().is_enabled)
            .map(|s| s.to_string())
            .collect()
    }
}

#[derive(PartialEq, Clone)]
pub enum DependencySource {
    Local(String),
    Remote,
}

pub fn get_path_from_dependency_kind(kind: DependencyKind) -> &'static str {
    match kind {
        DependencyKind::Normal => "dependencies",
        DependencyKind::Development => "dev-dependencies",
        DependencyKind::Build => "build-dependencies",
        DependencyKind::Unknown => "dependencies",
    }
}

#[derive(Clone, Debug)]
pub struct FeatureData {
    pub sub_features: Vec<SubFeature>,
    pub is_default: bool,
    pub is_enabled: bool,
}

#[derive(Clone, Debug)]
pub struct SubFeature {
    pub name: String,
    pub kind: FeatureType,
}

#[derive(Clone, PartialEq, Debug)]
pub enum FeatureType {
    Normal,
    Dependency,
    DependencyFeature,
}

impl Display for SubFeature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.kind == FeatureType::Dependency {
            f.write_str(&format!(
                "{}{}",
                Emoji("📦", "dep:"),
                self.name.trim_start_matches("dep:")
            ))?
        } else {
            f.write_str(&self.name)?
        }

        Ok(())
    }
}

impl From<&str> for FeatureType {
    fn from(s: &str) -> Self {
        if s.starts_with("dep:") {
            return FeatureType::Dependency;
        }

        if s.contains('/') {
            return FeatureType::DependencyFeature;
        }

        FeatureType::Normal
    }
}
