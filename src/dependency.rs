use std::collections::HashMap;

pub struct Dependency {
    pub(crate) dep_name: String,
    pub(crate) version: String,

    pub(crate) origin: DependencyOrigin,

    pub(crate) features_map: HashMap<String, Vec<String>>,
    pub(crate) features: Vec<(String, bool)>,
    pub(crate) default_features: Vec<String>,
}

impl Dependency {
    pub fn get_name(&self) -> String {
        self.dep_name.to_string()
    }

    pub fn get_version(&self) -> String {
        self.version.to_string()
    }

    pub fn get_features(&self) -> Vec<(String, bool)> {
        self.features.clone()
    }

    pub fn has_features(&self) -> bool {
        !self.features.is_empty()
    }

    pub fn get_features_count(&self) -> usize {
        self.features.len()
    }

    pub fn get_sub_features(&self, name: &String) -> Vec<String> {
        self.features_map.get(name).unwrap_or(&vec![]).clone()
    }

    fn get_all_enabled_features(&self) -> Vec<String> {
        self.features
            .iter()
            .filter(|(_, enabled)| *enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn can_use_default(&self) -> bool {
        let enabled_features = self.get_all_enabled_features();

        for name in &self.default_features {
            if !enabled_features.contains(name) {
                return false;
            }
        }

        true
    }

    pub fn get_enabled_features(&self) -> Vec<String> {
        let mut default_features = &vec![];

        if self.can_use_default() {
            default_features = &self.default_features;
        }

        self.features
            .iter()
            .filter(|(_, enabled)| *enabled)
            .map(|(name, _)| name.clone())
            .filter(|name| !default_features.contains(name))
            .collect()
    }

    pub fn toggle_feature_usage(&mut self, feature_index: usize) {
        let (name, enabled) = self.features.get(feature_index).unwrap();

        if *enabled {
            self.disable_feature_usage(&name.clone());
        } else {
            self.enable_feature_usage(&name.clone());
        }
    }

    pub fn enable_feature_usage(&mut self, feature_name: &String) {
        let index = self
            .get_feature_index(feature_name)
            .unwrap_or_else(|| panic!("feature named {} not found", feature_name));
        let data = self.features.get_mut(index).unwrap();

        if data.1 {
            //early return to prevent loop
            return;
        }

        data.1 = true;

        if !self.features_map.contains_key(feature_name) {
            return;
        }

        let sub_features = self.features_map.get(feature_name).unwrap().clone();

        for sub_feature_name in sub_features {
            self.enable_feature_usage(&sub_feature_name);
        }
    }

    pub fn disable_feature_usage(&mut self, feature_name: &String) {
        let index = self
            .get_feature_index(feature_name)
            .unwrap_or_else(|| panic!("feature named {} not found", feature_name));
        let data = self.features.get_mut(index).unwrap();

        if !data.1 {
            //early return to prevent loop
            return;
        }

        data.1 = false;

        for name in self.get_required_features(feature_name) {
            self.disable_feature_usage(&name)
        }
    }

    fn get_required_features(&self, feature_name: &String) -> Vec<String> {
        let mut dep_features = vec![];

        for (name, sub_features) in &self.features_map {
            if sub_features.contains(feature_name) {
                dep_features.push(name.to_string())
            }
        }

        dep_features
    }

    pub fn get_currently_required_features(&self, feature_name: &String) -> Vec<String> {
        self.get_required_features(feature_name)
            .iter()
            .filter(|name| {
                let index = self.get_feature_index(name).unwrap();
                self.features.get(index).unwrap().1
            })
            .map(|s| s.to_string())
            .collect()
    }

    pub fn is_default_feature(&self, feature_name: &String) -> bool {
        self.default_features.contains(feature_name)
    }

    fn get_feature_index(&self, feature_name: &String) -> Option<usize> {
        for (index, (name, _)) in self.features.iter().enumerate() {
            if name == feature_name {
                return Some(index);
            }
        }

        None
    }
}

#[derive(PartialEq, Clone)]
pub enum DependencyOrigin {
    Local(String),
    Remote,
}
