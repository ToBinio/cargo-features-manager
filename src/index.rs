// use std::fs;
// use std::str::FromStr;
// use anyhow::Error;
// use clap::builder::Str;
// use semver::{Version, VersionReq};
// use toml_edit::{Item, Value};
//
// use crate::dependency::Dependency;
//
// pub struct Index {
//     crates_index: Box<crates_index::Index>,
// }
//
// impl Index {
//     // pub fn new() -> Index {
//     //     Index {
//     //         crates_index: Box::new(crates_index::Index::new_cargo_default().unwrap()),
//     //     }
//     // }
//
//     pub fn get_dependency(&self, crate_name: &str, item: &Item) -> anyhow::Result<Dependency> {
//         let mut enabled_features = vec![];
//
//         let mut uses_default = true;
//
//         let dependency;
//
//         if item.is_str() {
//             dependency = self.get_dependency_from_remote(
//                 crate_name.to_string(),
//                 item.as_str().unwrap().to_string(),
//                 true,
//                 vec![],
//             );
//         } else {
//             let table = item.as_inline_table().unwrap();
//
//             match table.get("path") {
//                 None => {
//                     let version_str = table.get("version").unwrap().as_str().unwrap();
//
//                     let features = match table.get("features") {
//                         None => Vec::new(),
//                         Some(value) => value.as_array().unwrap().iter().collect(),
//                     };
//
//                     for value in features {
//                         enabled_features.push(value.as_str().unwrap().to_string());
//                     }
//
//                     if let Some(value) = table.get("default-features") {
//                         uses_default = value.as_bool().unwrap();
//                     }
//
//                     dependency = self.get_dependency_from_remote(
//                         crate_name.to_string(),
//                         version_str.to_string(),
//                         uses_default,
//                         enabled_features,
//                     );
//                 }
//                 Some(path) => {
//                     todo!()
//                 }
//             }
//         }
//
//         dependency
//     }
//
//     fn get_dependency_from_remote(
//         &self,
//         crate_name: String,
//         version_str: String,
//         uses_default: bool,
//         enabled_features: Vec<String>,
//     ) -> anyhow::Result<Dependency> {
//         let version_req = VersionReq::parse(&version_str).unwrap();
//
//         let mut possible_versions: Vec<crates_index::Version> = self
//             .crates_index
//             .crate_(&crate_name)
//             .unwrap()
//             .versions()
//             .iter()
//             .filter(|version| version_req.matches(&Version::parse(version.version()).unwrap()))
//             .cloned()
//             .collect();
//
//         possible_versions.sort_by(|a, b| {
//             Version::parse(a.version())
//                 .unwrap()
//                 .cmp(&Version::parse(b.version()).unwrap())
//         });
//
//         match possible_versions.first() {
//             None => Err(Error::msg("no fitting version found")),
//             Some(version) => Ok(Dependency::new(
//                 version.clone(),
//                 enabled_features,
//                 uses_default,
//             )),
//         }
//     }
// }
