use crate::crates::Crate;
use toml_edit::Item;

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

    //todo move func to crates
    pub fn get_crate(&self, crate_name: &str, item: &Item) -> Option<Crate> {
        let mut enabled_features = vec![];

        let version_str;
        let mut uses_default = true;

        if item.is_str() {
            version_str = item.as_str().unwrap();
        } else {
            let table = item.as_inline_table().unwrap();

            version_str = table.get("version").unwrap().as_str().unwrap();

            for value in table.get("features").unwrap().as_array().unwrap() {
                enabled_features.push(value.as_str().unwrap().to_string());
            }

            if let Some(value) = table.get("default-features") {
                uses_default = value.as_bool().unwrap();
            }
        }


        for version in self.crates_index.crate_(crate_name).unwrap().versions() {
            if version.version() == version_str {
                return Some(Crate::new(version.clone(), enabled_features, uses_default));
            }
        }

        None
    }
}
