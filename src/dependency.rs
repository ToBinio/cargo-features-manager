use bitap::Pattern;
use clap::builder::Str;
use std::cmp::Ordering;
use std::collections::HashMap;

use levenshtein::levenshtein;

pub struct Dependency {
    pub(crate) dep_name: String,
    pub(crate) version: String,

    pub(crate) origin: DependencyOrigin,

    pub(crate) features: HashMap<String, FeatureData>,
}

impl Dependency {
    pub fn get_name(&self) -> String {
        self.dep_name.to_string()
    }

    pub fn get_version(&self) -> String {
        self.version.to_string()
    }

    pub fn get_features_filtered_view(&self, filter: String) -> Vec<String> {
        let mut features: Vec<(&String, &FeatureData)> = self.features.iter().collect();

        if filter.is_empty() {
            features.sort_by(|(name_a, data_a), (name_b, data_b)| {
                if data_a.is_default && !data_b.is_default {
                    return Ordering::Less;
                }

                if data_b.is_default && !data_a.is_default {
                    return Ordering::Greater;
                }

                name_a.partial_cmp(name_b).unwrap()
            });

            features.iter().map(|(name, _)| name.to_string()).collect()
        } else {
            let pattern = Pattern::new(&filter).unwrap();
            let max_diff = (filter.len() as f32).log(3.0) as usize;

            let mut features: Vec<(String, usize)> = features
                .iter()
                .filter(|(name, _)| pattern.lev(&name, max_diff).next().is_some())
                .map(|(name, _)| (name.to_string(), levenshtein(name, &filter)))
                .collect();

            features.sort_by(|(_, lev_a), (_, lev_b)| lev_a.cmp(lev_b));

            features.iter().map(|(name, _)| name.to_string()).collect()
        }
    }

    pub fn get_feature(&self, feature_name: &String) -> &FeatureData {
        self.features.get(feature_name).unwrap()
    }

    pub fn has_features(&self) -> bool {
        !self.features.is_empty()
    }

    pub fn get_features_count(&self) -> usize {
        self.features.len()
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

    pub fn toggle_feature_usage(&mut self, feature_name: &String) {
        let data = self.features.get(feature_name).unwrap();

        if data.is_enabled {
            self.disable_feature_usage(feature_name);
        } else {
            self.enable_feature_usage(feature_name);
        }
    }

    pub fn enable_feature_usage(&mut self, feature_name: &String) {
        let data = self.features.get_mut(feature_name).unwrap();

        if data.is_enabled {
            //early return to prevent loop
            return;
        }

        data.is_enabled = true;

        //enable sub features
        let sub_features = data.sub_features.clone();

        for sub_feature_name in sub_features {
            self.enable_feature_usage(&sub_feature_name);
        }
    }

    pub fn disable_feature_usage(&mut self, feature_name: &String) {
        let data = self.features.get_mut(feature_name).unwrap();

        if !data.is_enabled {
            //early return to prevent loop
            return;
        }

        data.is_enabled = false;

        for name in self.get_dependent_features(feature_name) {
            self.disable_feature_usage(&name)
        }
    }

    /// returns all features which require the feature to be enabled
    fn get_dependent_features(&self, feature_name: &String) -> Vec<String> {
        let mut dep_features = vec![];

        for (name, data) in &self.features {
            if data.sub_features.contains(feature_name) {
                dep_features.push(name.to_string())
            }
        }

        dep_features
    }

    /// returns all features which are currently enabled and require the feature to be enabled
    pub fn get_currently_dependent_features(&self, feature_name: &String) -> Vec<String> {
        self.get_dependent_features(feature_name)
            .iter()
            .filter(|name| self.features.get(*name).unwrap().is_enabled)
            .map(|s| s.to_string())
            .collect()
    }
}

#[derive(PartialEq, Clone)]
pub enum DependencyOrigin {
    Local(String),
    Remote,
}

#[derive(Clone)]
pub struct FeatureData {
    pub(crate) sub_features: Vec<String>,
    pub(crate) is_default: bool,
    pub(crate) is_enabled: bool,
}
