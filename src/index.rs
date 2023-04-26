use semver::{Version, VersionReq};
use toml_edit::Item;

use crate::crates::Crate;

pub struct Index {
    crates_index: Box<crates_index::Index>,
}

impl Index {
    pub fn new() -> Index {
        let index = crates_index::Index::new_cargo_default().unwrap();

        Index {
            crates_index: Box::new(index),
        }
    }

    pub fn get_crate(&self, crate_name: &str, item: &Item) -> Option<Crate> {
        let mut enabled_features = vec![];

        let version_str;
        let mut uses_default = true;

        if item.is_str() {
            version_str = item.as_str().unwrap();
        } else {
            let table = item.as_inline_table().unwrap();

            version_str = table.get("version").unwrap().as_str().unwrap();

            let features = match table.get("features") {
                None => Vec::new(),
                Some(value) => value.as_array().unwrap().iter().collect(),
            };

            for value in features {
                enabled_features.push(value.as_str().unwrap().to_string());
            }

            if let Some(value) = table.get("default-features") {
                uses_default = value.as_bool().unwrap();
            }
        }

        let version_req = VersionReq::parse(version_str).unwrap();

        let mut possible_versions: Vec<crates_index::Version> = self
            .crates_index
            .crate_(crate_name)
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

        possible_versions
            .first()
            .map(|some| Crate::new(some.clone(), enabled_features, uses_default))
    }
}
