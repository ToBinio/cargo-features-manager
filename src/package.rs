use crate::dependencies::dependency::Dependency;

pub struct Package {
    pub dependencies: Vec<Dependency>,
    pub name: String,
    pub toml_doc: toml_edit::Document,
    pub dir_path: String,
}
