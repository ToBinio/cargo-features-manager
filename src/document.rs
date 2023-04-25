use crate::crates::Crate;
use crate::index::Index;
use crates_index::Version;
use std::fs;
use std::path::Path;
use std::str::FromStr;

pub struct Document {
    toml_doc: toml_edit::Document,
    index: Index,
}

impl Document {
    pub fn new<P: AsRef<Path>>(path: P, index: Index) -> Document {
        let file_content = fs::read_to_string(path).unwrap();
        let doc = toml_edit::Document::from_str(&file_content).unwrap();

        Document {
            toml_doc: doc,
            index,
        }
    }

    pub fn get_deps(&self) -> Vec<Crate> {
        let (_name, deps) = self.toml_doc.get_key_value("dependencies").unwrap();
        let deps = deps.as_table().unwrap();

        let mut vec = vec![];

        for (name, value) in deps {
            if value.is_str() {
                vec.push(self.index.get_crate(name, value.as_str().unwrap()).unwrap());
            }
        }

        vec
    }
}
