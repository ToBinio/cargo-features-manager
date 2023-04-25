use crates_index::Version;
use std::collections::HashMap;
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

    pub fn get_crate(&self, crate_name: &str, version_str: &str) -> Option<Crate> {
        for version in self.crates_index.crate_(crate_name).unwrap().versions() {
            if version.version() == version_str {
                return Some(Crate::new(version.clone()));
            }
        }

        None
    }
}
