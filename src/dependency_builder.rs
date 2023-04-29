use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::str::FromStr;

use anyhow::anyhow;
use semver::{Version, VersionReq};
use toml_edit::Item;

use crate::dependency::{Dependency, DependencyOrigin};

pub struct DependencyBuilder {
    dep_name: String,
    version: String,

    origin: DependencyOrigin,

    all_features: HashMap<String, Vec<String>>,

    optional_dependency: Vec<String>,

    enabled_features: Vec<String>,
    uses_default: bool,
}

impl DependencyBuilder {
    pub fn build_dependency(dep_name: &str, item: &Item) -> anyhow::Result<Dependency> {
        let mut builder = DependencyBuilder {
            dep_name: dep_name.to_string(),

            version: "".to_string(),

            origin: DependencyOrigin::Remote,

            all_features: HashMap::new(),

            optional_dependency: vec![],

            enabled_features: vec![],
            uses_default: true,
        };

        if item.is_str() {
            builder.version = item
                .as_str()
                .ok_or(anyhow!("could not parse {} - version tag", dep_name))?
                .to_string();
            builder.set_features_from_index()?;
        } else {
            let table = item
                .as_inline_table()
                .ok_or(anyhow!("could not parse {} - body", dep_name))?;

            if let Some(value) = table.get("features") {
                builder.enabled_features = value
                    .as_array()
                    .ok_or(anyhow!("could not parse {} - enabled features", dep_name))?
                    .iter()
                    .map(|f| f.as_str().unwrap().to_string())
                    .collect();
            }

            if let Some(value) = table.get("default-features") {
                builder.uses_default = value
                    .as_bool()
                    .ok_or(anyhow!("could not parse {} - default-features", dep_name))?;
            }

            match table.get("path") {
                None => {
                    builder.version = table
                        .get("version")
                        .ok_or(anyhow!("could not parse {} - version", dep_name))?
                        .as_str()
                        .ok_or(anyhow!("could not parse {} - version", dep_name))?
                        .to_string();
                    builder.set_features_from_index()?;
                }
                Some(path) => {
                    let path = path
                        .as_str()
                        .ok_or(anyhow!("could not parse {} - path", dep_name))?
                        .to_string();

                    builder.origin = DependencyOrigin::Local(path.clone());

                    let path = "./".to_string() + &path + "/Cargo.toml";

                    let toml_document =
                        toml_edit::Document::from_str(&fs::read_to_string(path.clone()).map_err(
                            |_| anyhow!("could not find dependency {} - {}", dep_name, path),
                        )?)?;

                    builder.set_data_from_toml(toml_document)?;
                }
            }
        }

        Ok(builder.build())
    }

    fn set_data_from_toml(&mut self, toml: toml_edit::Document) -> anyhow::Result<()> {
        if let Some(features) = toml.get("features") {
            let features = features
                .as_table()
                .ok_or(anyhow!("could not parse {} - features", self.dep_name))?;

            for (feature_name, sub_features) in features {
                self.all_features.insert(
                    feature_name.to_string(),
                    sub_features
                        .as_array()
                        .ok_or(anyhow!("could not parse {} - features", self.dep_name))?
                        .iter()
                        .map(|x| x.as_str().unwrap().to_string())
                        .collect(),
                );
            }
        }

        if let Some(dependencies) = toml.get("dependencies") {
            let dependencies = dependencies
                .as_table()
                .ok_or(anyhow!("could not parse {} - dependencies", self.dep_name))?;

            for (dep_name, data) in dependencies {
                if let Some(data) = data.as_inline_table() {
                    if let Some(optional) = data.get("optional") {
                        if optional
                            .as_bool()
                            .ok_or(anyhow!("could not parse {} - dependencies", self.dep_name))?
                        {
                            self.optional_dependency.push(dep_name.to_string());
                        }
                    }
                }
            }
        }

        Ok(())
    }
    fn set_features_from_index(&mut self) -> anyhow::Result<()> {
        //todo cache
        let index = crates_index::Index::new_cargo_default()?;

        let version_req = VersionReq::parse(&self.version)?;

        let mut possible_versions: Vec<crates_index::Version> = index
            .crate_(&self.dep_name)
            .ok_or(anyhow!("could not find {} in local index", self.dep_name))?
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
            None => Err(anyhow!(
                "could not find appropriate version for {} in local index",
                self.dep_name
            )),
            Some(version) => {
                // add indirect features (features out of dependency)
                for dep in version.dependencies() {
                    if dep.is_optional() {
                        self.optional_dependency.push(dep.name().to_string());
                    }
                }

                self.all_features = version.features().clone();
                Ok(())
            }
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

        //flatten features
        for (name, sub) in &features_map {
            features.push((name.clone(), false));

            for name in sub {
                features.push((name.clone(), false));
            }
        }

        //add Optional dependencies
        for dep_name in &self.optional_dependency {
            let mut is_defined = false;

            for (_, sub_features) in &self.all_features {
                if sub_features.contains(&("dep:".to_string() + dep_name)) {
                    is_defined = true;
                    break;
                }
            }

            if !is_defined {
                features.push((dep_name.to_string(), false));
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

            origin: self.origin.clone(),

            features_map,

            features: features.clone(),
            default_features: default_features.clone(),
        };

        //enable features
        for (name, _) in features {
            if (self.uses_default && default_features.contains(&name))
                || self.enabled_features.contains(&name)
            {
                new_crate.enable_feature_usage(&name);
            }
        }

        new_crate
    }
}
