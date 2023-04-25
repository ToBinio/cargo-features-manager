use crates_index::Version;

//todo handle no-default
pub struct Crate {
    version: Version,
    features: Vec<(String, bool)>,
    default_features: Vec<String>,
}

impl Crate {
    pub fn new(version: Version, enabled_features: Vec<String>, has_default: bool) -> Crate {
        let mut features = vec![];

        let default_features = version.features().get("default").unwrap().clone();

        for (name, sub) in version.features() {
            //skip if is is default
            if *name == "default" {
                continue;
            }

            features.push((name.clone(), false));

            for name in sub {
                //skip if it is a dep or a feature of a dep
                if name.contains(':') || name.contains('/') {
                    continue;
                }

                features.push((name.clone(), false));
            }
        }

        features.dedup();
        features.sort();

        for (name, enabled) in features.iter_mut() {
            if (has_default && default_features.contains(name)) || enabled_features.contains(name) {
                *enabled = true;
            }
        }

        Crate {
            version,
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
        //todo enable sub packages

        let data = self.features.get_mut(feature_index).unwrap();

        data.1 = !data.1;
    }

    pub fn is_default_feature(&self, feature_name: &String) -> bool {
        self.default_features.contains(&feature_name)
    }
}
