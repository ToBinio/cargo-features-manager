use crates_index::Version;
use std::cmp::Ordering;
use std::collections::HashMap;

pub struct Crate {
    version: Version,
    features_map: HashMap<String, Vec<String>>,
    features: Vec<(String, bool)>,
    default_features: Vec<String>,
}

impl Crate {
    pub fn new(version: Version, enabled_features: Vec<String>, has_default: bool) -> Crate {

        let mut features_map = HashMap::new();

        for (name, sub) in version.features() {
            //skip if is is default
            if *name == "default" {
                continue;
            }

            let sub: Vec<String> = sub.iter().filter(|name| !name.contains(':') && !name.contains('/')).map(|s| s.to_string()).collect();

            features_map.insert(name.to_string(),sub);
        }

        let default_features = version.features().get("default").unwrap().clone();

        let mut features = vec![];

        for (name, sub) in &features_map {
            features.push((name.clone(), false));

            for name in sub {
                features.push((name.clone(), false));
            }
        }

        features.dedup();
        features.sort_by(|(name_a, _), (name_b, _)| {
            if default_features.contains(name_a) && !default_features.contains(name_b) {
                return Ordering::Less;
            }

            if default_features.contains(name_b) && !default_features.contains(name_a) {
                return Ordering::Greater;
            }

            name_a.partial_cmp(name_b).unwrap()
        });

        for (name, enabled) in features.iter_mut() {
            if (has_default && default_features.contains(name)) || enabled_features.contains(name) {
                *enabled = true;
            }
        }

        Crate {
            version,
            features_map,
            features,
            default_features,
        }
    }

    pub fn get_name(&self) -> String {
        self.version.name().to_string()
    }

    pub fn get_version(&self) -> String {
        self.version.version().to_string()
    }

    pub fn get_features(&self) -> Vec<(String, bool)> {
        self.features.clone()
    }

    pub fn get_sub_features(&self, name: &String) -> Vec<String> {
        self.features_map.get(name).unwrap_or(&vec![]).clone()
    }

    pub fn get_features_count(&self) -> usize {
        self.features.len()
    }

    fn get_all_enabled_features(&self) -> Vec<String> {
        self.features
            .iter()
            .filter(|(_, enabled)| *enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn uses_default(&self) -> bool {
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

        if self.uses_default() {
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
            .get_index(feature_name)
            .expect(&format!("feature named {} not found", feature_name));
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
            .get_index(feature_name)
            .expect(&format!("feature named {} not found", feature_name));
        let data = self.features.get_mut(index).unwrap();

        if !data.1 {
            //early return to prevent loop
            return;
        }

        data.1 = false;

        let features = self.features_map.clone();

        for (name, sub_features) in features {
            if sub_features.contains(feature_name) {
                self.disable_feature_usage(&name)
            }
        }
    }

    pub fn is_default_feature(&self, feature_name: &String) -> bool {
        self.default_features.contains(&feature_name)
    }

    fn get_index(&self, feature_name: &String) -> Option<usize> {
        for (index, (name, _)) in self.features.iter().enumerate() {
            if name == feature_name {
                return Some(index);
            }
        }

        None
    }
}
