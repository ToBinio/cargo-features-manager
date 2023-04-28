use std::cmp::Ordering;
use crate::dependency::Dependency;
use anyhow::Error;
use clap::builder::Str;
use semver::{Version, VersionReq};
use std::collections::HashMap;
use toml_edit::Item;

pub struct DependencyBuilder {
    dep_name: String,
    version: String,

    item: Item,

    all_features: HashMap<String, Vec<String>>,

    enabled_features: Vec<String>,
    uses_default: bool,
}

impl DependencyBuilder {
    pub fn new(dep_name: &str, item: &Item) -> anyhow::Result<Dependency> {
        let mut builder = DependencyBuilder {
            dep_name: dep_name.to_string(),

            version: "".to_string(),
            item: item.clone(),

            all_features: HashMap::new(),

            enabled_features: vec![],
            uses_default: true,
        };

        if item.is_str() {
            builder.version = item.as_str().unwrap().to_string();
            builder.set_features_from_remote()?;
        } else {
            let table = item.as_inline_table().unwrap();

            match table.get("path") {
                None => {
                    builder.version = table.get("version").unwrap().as_str().unwrap().to_string();
                    builder.set_features_from_remote()?;

                    if let Some(value) = table.get("features") {
                        builder.enabled_features = value
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|f| f.as_str().unwrap().to_string())
                            .collect();
                    }

                    if let Some(value) = table.get("default-features") {
                        builder.uses_default = value.as_bool().unwrap();
                    }
                }
                Some(path) => {
                    todo!()
                }
            }
        }

        Ok(builder.build())
    }

    fn set_features_from_remote(&mut self) -> anyhow::Result<()> {
        //todo cache
        let index = crates_index::Index::new_cargo_default().unwrap();

        let version_req = VersionReq::parse(&self.version).unwrap();

        let mut possible_versions: Vec<crates_index::Version> = index
            .crate_(&self.dep_name)
            .unwrap()
            .versions()
            .iter()
            .filter(|version| version_req.matches(&Version::parse(version.version()).unwrap()))
            .cloned()
            .collect();

        possible_versions.sort_by(|a, b| {
            Version::parse(a.version())
                .unwrap()
                .cmp(&Version::parse(b.version()).unwrap())
        });

        match possible_versions.first() {
            None => Err(Error::msg("no fitting version found")),
            Some(version) => {
                let mut all_features = version.features().clone();

                // add indirect features (features out of dependency)
                for dep in version.dependencies() {
                    if dep.is_optional() {
                        all_features.insert(dep.name().to_string(), vec![]);
                    }
                }

                Ok(self.all_features =  all_features)
            },
        }
    }

    fn build(&self) -> Dependency {
        let mut features_map = HashMap::new();

        for (name, sub) in &self.all_features {
            //skip if is is default
            if *name == "default" {
                continue;
            }

            let sub: Vec<String> = sub
                .iter()
                .filter(|name| !name.contains(':') && !name.contains('/'))
                .map(|s| s.to_string())
                .collect();

            features_map.insert(name.to_string(), sub);
        }

        let default_features = self.all_features.get("default").unwrap_or(&vec![]).clone();

        let mut features = vec![];

        // flatten features
        for (name, sub) in &features_map {
            features.push((name.clone(), false));

            for name in sub {
                features.push((name.clone(), false));
            }
        }

        features.sort_by(|(name_a, _), (name_b, _)| {
            if default_features.contains(name_a) && !default_features.contains(name_b) {
                return Ordering::Less;
            }

            if default_features.contains(name_b) && !default_features.contains(name_a) {
                return Ordering::Greater;
            }

            name_a.partial_cmp(name_b).unwrap()
        });

        features.dedup();

        let mut new_crate = Dependency {
            dep_name: self.dep_name.to_string(),
            version: self.version.to_string(),
            features_map,
            features: features.clone(),
            default_features: default_features.clone(),
        };

        //enable features
        for (name, _) in features {
            if (self.uses_default && default_features.contains(&name)) || self.enabled_features.contains(&name)
            {
                new_crate.enable_feature_usage(&name);
            }
        }

        new_crate
    }
}
