mod crates;
mod document;
mod index;

use crate::document::Document;
use crate::index::Index;
use crates_index::Version;
use std::any::Any;
use std::str::FromStr;

fn main() {
    let document = Document::new("./Cargo.toml", Index::new());

    for dep in document.get_deps() {
        println!("{}", dep.get_name());
        for name in dep.get_unique_features() {
            println!("  {}", name);
        }
    }
}
