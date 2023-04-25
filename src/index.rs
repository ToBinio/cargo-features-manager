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
        let version_str;
        let mut enabled_features = vec![];

        if item.is_str() {
            version_str = item.as_str().unwrap();
        } else {
            let table = item.as_inline_table().unwrap();

            version_str = table.get("version").unwrap().as_str().unwrap();

            for value in table.get("features").unwrap().as_array().unwrap() {
                enabled_features.push(value.as_str().unwrap().to_string());
            }
        }

        for version in self.crates_index.crate_(crate_name).unwrap().versions() {
            if version.version() == version_str {
                return Some(Crate::new(version.clone(), enabled_features, true));
            }
        }

        None
    }
}
