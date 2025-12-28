use crate::io::save::save_dependency;
use crate::project::dependency::Dependency;
use crate::project::document::Document;
use crate::prune::display::Display;
use crate::prune::parse::get_features_to_test;
use crate::{CleanLevel, PruneArgs};
use color_eyre::Result;
use color_eyre::eyre::{ContextCompat, eyre};
use dircpy::copy_dir;
use itertools::Itertools;
use std::collections::HashMap;
use std::ops::Not;
use std::path::Path;
use std::process::{Command, Stdio};

use tempfile::TempDir;

mod parse;

mod display;

type PackageName = String;
pub type DependencyName = String;
pub type FeatureName = String;
pub type FeaturesMap = HashMap<PackageName, HashMap<DependencyName, Vec<FeatureName>>>;

pub fn prune(args: &PruneArgs) -> Result<()> {
    let mut main_document = Document::new(".")?;

    //needed to be set here so the temp_dir lives long enough
    let tmp_dir = TempDir::with_prefix_in(".cargo-features-manager-", ".")?;

    let mut document = if args.no_tmp {
        Document::new(".")?
    } else {
        println!("Creating temporary project...");
        let project_path = tmp_dir.path();

        copy_dir(main_document.root_path(), project_path)?;

        match Document::new(project_path) {
            Ok(document) => {document}
            Err(err) => {
                return Err(err.wrap_err("Failed to create the temporary project - try to use cargo `features prune --no-tmp`"))
            }
        }
    };

    let features_to_test = get_features_to_test(&document, args.only_dependency)?;

    let mut pruner = Pruner {
        skip_tests: args.skip_tests,
        clean_level: args.clean.clone(),
        document: &mut document,
        known_features: known_features()?,
    };
    let to_be_disabled = pruner.run(features_to_test)?;

    if args.dry_run {
        return Ok(());
    }

    for (package_name, dependency) in to_be_disabled {
        for (dependency_name, features) in dependency {
            for feature in features {
                main_document
                    .get_package_mut(&package_name)?
                    .get_dep_mut(&dependency_name)?
                    .disable_feature(&feature)?;
            }

            save_dependency(&mut main_document, &package_name, &dependency_name)?;
        }
    }

    Ok(())
}

//give a map of known features that do not affect completion but remove functionality
pub fn known_features() -> Result<HashMap<String, Vec<String>>> {
    let file = include_str!("../../Known-Features.toml");

    let document: toml_edit::DocumentMut = file.parse()?;

    let mut map = HashMap::new();

    for (dependency, features) in document.as_table() {
        let features = features
            .as_array()
            .context("could not parse Known-Features.toml")?;

        let features = features
            .iter()
            .filter_map(|item| item.as_str())
            .map(|name| name.to_string())
            .collect_vec();

        map.insert(dependency.to_string(), features);
    }

    Ok(map)
}

struct Pruner<'a> {
    skip_tests: bool,
    clean_level: CleanLevel,
    document: &'a mut Document,
    known_features: HashMap<String, Vec<String>>,
}

impl<'a> Pruner<'a> {
    fn run(&mut self, all_features: FeaturesMap) -> Result<FeaturesMap> {
        let mut features_map = HashMap::new();

        let mut has_known_features_enabled = false;

        let mut display = Display::new(&all_features, self.document);
        display.start()?;

        for (package_name, dependencies) in all_features
            .into_iter()
            .sorted_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b))
        {
            if self.prune_package(
                &package_name,
                &dependencies,
                &mut display,
                &mut features_map,
            )? {
                has_known_features_enabled = true
            };
        }

        if has_known_features_enabled {
            display.display_known_features_notice()?;
        }

        display.finish()?;

        Ok(features_map)
    }

    fn prune_package(
        &mut self,
        package_name: &str,
        dependencies: &HashMap<String, Vec<String>>,
        display: &mut Display,
        features_map: &mut FeaturesMap,
    ) -> Result<bool> {
        if dependencies.is_empty() {
            return Ok(false);
        }

        let mut has_known_features_enabled = false;

        display.next_package(package_name, dependencies)?;

        for (dependency_name, features) in dependencies
            .iter()
            .sorted_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b))
        {
            if self.prune_dependency(
                package_name,
                dependency_name,
                features,
                display,
                features_map,
            )? {
                has_known_features_enabled = true;
            };
        }

        if let CleanLevel::Package = self.clean_level {
            clean(self.document.root_path())?;
        }

        Ok(has_known_features_enabled)
    }

    fn prune_dependency(
        &mut self,
        package_name: &str,
        dependency_name: &str,
        features: &Vec<String>,
        display: &mut Display,
        features_map: &mut FeaturesMap,
    ) -> Result<bool> {
        if features.is_empty() {
            return Ok(false);
        }

        let mut has_known_features_enabled = false;

        let mut known_features_list = vec![];
        let dependency = self
            .document
            .get_package(package_name)?
            .get_dep(dependency_name)?;

        for feature_name in self.known_features.get(dependency_name).unwrap_or(&vec![]) {
            set_features_to_be_kept(
                dependency,
                feature_name.to_string(),
                &mut known_features_list,
            )
        }

        let mut to_be_disabled = vec![];
        to_be_disabled.append(&mut known_features_list.clone());

        display.next_dependency(dependency_name, features);

        for (id, feature) in features.iter().enumerate() {
            display.next_feature(id, feature)?;

            self.document
                .get_package_mut(package_name)?
                .get_dep_mut(dependency_name)?
                .disable_feature(feature)?;

            save_dependency(self.document, package_name, dependency_name)?;

            if to_be_disabled.contains(feature).not()
                && check(self.skip_tests, self.document.root_path())?
            {
                set_features_to_be_disabled(
                    self.document
                        .get_package(package_name)?
                        .get_dep(dependency_name)?,
                    feature.to_string(),
                    &mut to_be_disabled,
                );
            }

            //reset to start
            for feature in features {
                self.document
                    .get_package_mut(package_name)?
                    .get_dep_mut(dependency_name)?
                    .enable_feature(feature)?;
            }

            save_dependency(self.document, package_name, dependency_name)?;

            display.finish_feature()?;
        }

        let features_result = features
            .iter()
            .filter(|feature| to_be_disabled.contains(feature))
            .map(|feature| {
                if known_features_list.contains(feature) {
                    has_known_features_enabled = true;
                    (feature, true)
                } else {
                    (feature, false)
                }
            })
            .collect();

        display.finish_dependency(features_result)?;

        if let CleanLevel::Dependency = self.clean_level {
            clean(self.document.root_path())?;
        }

        let to_be_disabled = to_be_disabled
            .into_iter()
            .filter(|feature| known_features_list.contains(feature).not())
            .collect_vec();

        features_map
            .entry(package_name.to_string())
            .or_default()
            .insert(dependency_name.to_string(), to_be_disabled);

        Ok(has_known_features_enabled)
    }
}

fn set_features_to_be_disabled(
    dependency: &Dependency,
    feature: String,
    to_be_disabled: &mut Vec<String>,
) {
    if to_be_disabled.contains(&feature) {
        return;
    }

    to_be_disabled.push(feature.clone());

    dependency
        .features
        .iter()
        .filter(|(_, data)| {
            data.sub_features
                .iter()
                .any(|sub_feature| sub_feature.name == feature)
        })
        .for_each(|(name, _)| {
            set_features_to_be_disabled(dependency, name.to_string(), to_be_disabled);
        });
}

fn set_features_to_be_kept(
    dependency: &Dependency,
    feature: String,
    to_be_disabled: &mut Vec<String>,
) {
    if to_be_disabled.contains(&feature) {
        return;
    }

    to_be_disabled.push(feature.clone());

    if let Some(feature) = dependency.get_feature(&feature) {
        for sub_feature in &feature.sub_features {
            set_features_to_be_kept(dependency, sub_feature.name.clone(), to_be_disabled);
        }
    }
}

fn clean<P: AsRef<Path>>(path: P) -> Result<()> {
    let mut child = Command::new("cargo")
        .current_dir(path)
        .arg("clean")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let _ = child.wait()?.code().ok_or(eyre!("Could not clear"))?;

    Ok(())
}

fn check<P: AsRef<Path>>(skip_tests: bool, path: P) -> Result<bool> {
    if !build(&path)? {
        return Ok(false);
    }

    if !skip_tests && !test(&path)? {
        return Ok(false);
    }

    Ok(true)
}

fn build<P: AsRef<Path>>(path: P) -> Result<bool> {
    const BUILD_TARGETS: [&str; 6] = [
        "--bins",
        "--lib",
        "--examples",
        "--tests",
        "--benches",
        "--all-targets",
    ];

    for target in BUILD_TARGETS {
        let mut child = Command::new("cargo")
            .current_dir(&path)
            .arg("build")
            .arg(target)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let code = child.wait()?.code().ok_or(eyre!("Could not build"))?;

        if code != 0 {
            return Ok(false);
        }
    }

    Ok(true)
}

fn test<P: AsRef<Path>>(path: P) -> Result<bool> {
    let mut child = Command::new("cargo")
        .current_dir(path)
        .arg("test")
        .arg("--workspace")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let code = child.wait()?.code().ok_or(eyre!("Could not test"))?;

    Ok(code == 0)
}
